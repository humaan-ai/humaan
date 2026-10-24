<script lang="ts">
  import { onMount, untrack } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { toast } from 'svelte-sonner';
  import { Button } from '$lib/components/ui/button/index.js';
  import { Input } from '$lib/components/ui/input/index.js';
  import { NativeSelect } from '$lib/components/ui/native-select/index.js';
  import SlidersHorizontal from '@lucide/svelte/icons/sliders-horizontal';
  import ChartLine from '@lucide/svelte/icons/chart-line';
  import Columns2 from '@lucide/svelte/icons/columns-2';
  import FlaskConical from '@lucide/svelte/icons/flask-conical';
  import Play from '@lucide/svelte/icons/play';
  import RefreshCw from '@lucide/svelte/icons/refresh-cw';
  import Settings from '@lucide/svelte/icons/settings';
  import LoaderCircle from '@lucide/svelte/icons/loader-circle';
  import StudioSidebar from './StudioSidebar.svelte';
  import ExperienceNavigation from './ExperienceNavigation.svelte';
  import LoopNavigation from './LoopNavigation.svelte';
  import WorkspaceColumn from './WorkspaceColumn.svelte';
  import BuilderViewport from './BuilderViewport.svelte';
  import { studio } from '$lib/studio.svelte';
  import { comparableRuns, type Catalog, type Experiment, type TrainingRun } from '$lib/training/model';
  let { root, reviewDirty = false, loopDirty = false }: { root: string; reviewDirty?: boolean; loopDirty?: boolean } = $props();
  let catalog = $state<Catalog>({ experiences: [], loops: [], environments: [], tasks: [], experiments: [], runs: [] });
  let versions = $state<{ id: string; name: string; savedAt: number }[]>([]);
  let busy = $state('');
  let error = $state('');
  let experienceId = $state(''), loopId = $state(''), versionId = $state(''), environmentId = $state(''), taskId = $state('');
  let newExperienceOpen = $state(false), newLoopOpen = $state(false), newEnvironmentOpen = $state(false), newTaskOpen = $state(false);
  let experienceName = $state(''), loopName = $state(''), environmentName = $state(''), taskName = $state('');
  let gravity = $state(9.81), duration = $state(1), targetFraction = $state(0.6), tolerance = $state(0.05);
  let name = $state(''), iterations = $state(3), population = $state(8), seed = $state(42);
  let compute = $state('local'), host = $state(''), cloudPython = $state('/usr/bin/python3');
  let experiment = $state<Experiment | null>(null), run = $state<TrainingRun | null>(null);
  let tab = $state<'setup' | 'results' | 'compare'>('setup');
  let frameIndex = $state(0), playing = $state(false), elapsed = $state(0);
  const loops = $derived(catalog.loops.filter(l => l.experienceId === experienceId));
  const experiments = $derived(catalog.experiments.filter(e => e.loop.id === loopId));
  const runs = $derived(catalog.runs.filter(r => r.loopId === loopId));
  const comparison = $derived(comparableRuns(catalog.runs, run));
  const frames = $derived(run?.result?.frames ?? []);
  const unsaved = $derived(!!(name || experienceName || loopName || environmentName || taskName) || (!experiment && (iterations !== 3 || population !== 8 || seed !== 42)));
  function discardDraft() { name = ''; experienceName = ''; loopName = ''; environmentName = ''; taskName = ''; iterations = 3; population = 8; seed = 42; }
  $effect(() => { const context = loopId; untrack(() => { if (experiment && experiment.loop.id !== context) experiment = null; if (run && run.loopId !== context) { run = null; playing = false; } }); });
  $effect(() => { studio.trainingDirty = unsaved; });
  $effect(() => { if (!loops.some(l => l.id === loopId)) loopId = loops[0]?.id ?? ''; });
  $effect(() => { if (playing && frames.length && studio.activeView === 'loop' && studio.loopSection === 'training') { const timer = setInterval(() => { if (frameIndex >= frames.length-1) playing = false; else frameIndex++; }, 50); return () => clearInterval(timer); } else playing = false; });
  $effect(() => { if (busy === 'Training') { elapsed = 0; const timer = setInterval(() => elapsed++, 1000); return () => clearInterval(timer); } });
  async function refresh() {
    const [next, robots] = await Promise.all([invoke<Catalog>('training_catalog', { root }), invoke<typeof versions>('builder_list', { root, kind: 'versions' })]);
    catalog = next; versions = robots;
    if (!experienceId) experienceId = next.experiences[0]?.id ?? '';
    if (!versionId || !robots.some(r => r.id === versionId)) versionId = robots[0]?.id ?? '';
    if (!environmentId) environmentId = next.environments[0]?.id ?? '';
    if (!taskId) taskId = next.tasks[0]?.id ?? '';
  }
  async function work(label: string, action: () => Promise<void>) {
    if (busy) return; busy = label; studio.trainingBusy = true; error = '';
    try { await action(); } catch (cause) { error = String(cause); toast.error(error); } finally { busy = ''; studio.trainingBusy = false; }
  }
  async function create(kind: string, input: Record<string, unknown>) {
    await work('Saving', async () => {
      const saved = await invoke<{id:string}>('training_create', { root, kind, input });
      if (kind === 'experiences') { experienceId = saved.id; experienceName = ''; loopId = ''; newExperienceOpen = false; }
      if (kind === 'loops') { loopId = saved.id; loopName = ''; newLoopOpen = false; }
      if (kind === 'environments') { environmentId = saved.id; environmentName = ''; newEnvironmentOpen = false; }
      if (kind === 'tasks') { taskId = saved.id; taskName = ''; newTaskOpen = false; }
      await refresh(); toast.success('Saved '+input.name);
    });
  }
  async function pin() {
    await work('Saving experiment', async () => {
      experiment = await invoke<Experiment>('training_create', { root, kind: 'experiments', input: { name, loopId, versionId, environmentId, taskId, config: { iterations, population, seed } } });
      name = ''; iterations = 3; population = 8; seed = 42; await refresh(); toast.success('Experiment saved. Choose compute and launch.');
    });
  }
  async function launch() {
    if (!experiment) return;
    await work('Training', async () => {
      playing = false;
        run = await invoke<TrainingRun>('training_run', { root, experimentId: experiment!.id, python: studio.simulationPython, execution: compute === 'local' ? { kind: 'local' } : { kind: 'cloud', host, python: cloudPython } });
        frameIndex = 0; tab = 'results';
        if (run.status === 'completed') toast.success('Training completed'); else toast.error(run.result?.error ?? 'Training failed');
        await refresh();

    });
  }
  async function openRun(id: string) { await work('Loading run', async () => { run = await invoke<TrainingRun>('training_load', { root, kind: 'runs', id }); frameIndex = 0; playing = false; tab = 'results'; }); }
  function percent(n: number) { return (n*100).toFixed(1)+'%'; }
  onMount(() => { void work('Loading training', refresh); return () => { studio.trainingDirty = false; }; });
</script>

<svelte:window onbeforeunload={(event) => { if (studio.trainingBusy || unsaved) { event.preventDefault(); event.returnValue = ''; } }} />
<div class="flex h-full min-h-0 gap-1">
  <StudioSidebar title="Experiences">
    <ExperienceNavigation busy={!!busy} {reviewDirty} loopDirty={loopDirty || unsaved} />
    <div class="flex min-h-0 flex-1 flex-col overflow-y-auto p-3 text-xs">
      <label>Experience<NativeSelect class="mt-1 w-full" bind:value={experienceId} disabled={!!busy}><option value="">Select experience</option>{#each catalog.experiences as e}<option value={e.id}>{e.name}</option>{/each}</NativeSelect></label>
      <details bind:open={newExperienceOpen} class="mt-2"><summary>New experience</summary><form class="mt-2 flex gap-1" onsubmit={e => { e.preventDefault(); create('experiences', { name: experienceName }); }}><Input aria-label="New experience name" placeholder="Joint control" bind:value={experienceName} required maxlength={200} /><Button type="submit" disabled={!!busy}>Add</Button></form></details>
      <label class="mt-4">Loop<NativeSelect class="mt-1 w-full" bind:value={loopId} disabled={!!busy}><option value="">Select loop</option>{#each loops as l}<option value={l.id}>{l.name}</option>{/each}</NativeSelect></label>
      <details bind:open={newLoopOpen} class="mt-2"><summary>New loop</summary><form class="mt-2 flex gap-1" onsubmit={e => { e.preventDefault(); create('loops', { name: loopName, experienceId }); }}><Input aria-label="New loop name" placeholder="Baseline" bind:value={loopName} required maxlength={200} /><Button type="submit" disabled={!!busy || !experienceId}>Add</Button></form></details>
      <div class="mt-5 flex items-center justify-between"><span>Experiments ({experiments.length})</span><Button size="icon" variant="ghost" aria-label="Refresh training" disabled={!!busy} onclick={() => work('Refreshing', refresh)}><RefreshCw size={14} /></Button></div>
      {#each experiments as e}<Button variant="ghost" class="h-auto justify-start whitespace-normal py-2 text-left {experiment?.id === e.id ? 'bg-selection' : ''}" disabled={!!busy} onclick={() => { experiment = e; tab = 'setup'; }}><FlaskConical size={14} /><span class="min-w-0 truncate">{e.name}</span></Button>{/each}
      <Button class="mt-2" disabled={!!busy} onclick={() => { experiment = null; tab = 'setup'; }}>New experiment</Button>
      <details open class="mt-5"><summary>Saved training runs ({runs.length})</summary><div class="mt-2">{#each runs as r}<Button variant="ghost" class="block h-auto w-full rounded-none py-2 text-left {run?.id === r.id ? 'bg-selection' : ''}" disabled={!!busy} onclick={() => openRun(r.id)}><span class="block truncate">{r.name}</span><span class="block text-[10px] text-muted">{r.status} · {r.execution.kind} · {new Date(r.createdAt).toLocaleString()}</span></Button>{:else}<p class="py-3 text-muted">Launch a saved experiment to begin.</p>{/each}</div></details>
    </div>
  </StudioSidebar>
  <WorkspaceColumn active={studio.activeView === 'loop'}>
    <section class="workbench-panel relative flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden" aria-label="Training workspace" aria-busy={!!busy}>
      <LoopNavigation instance="training" learningSetDirty={loopDirty} />
      <div id="loop-workspace-training" role="tabpanel" aria-labelledby="loop-section-training-training" class="flex min-h-0 flex-1 flex-col">
      <div role="group" aria-label="Training actions" class="flex h-[57px] shrink-0 items-center gap-3 border-b border-border px-4">
        {#each [{ id: 'setup', label: 'Setup', icon: SlidersHorizontal }, { id: 'results', label: 'Results', icon: ChartLine }, { id: 'compare', label: 'Compare', icon: Columns2 }] as const as item}
          <Button variant="outline" class={tab === item.id ? 'bg-selection text-title' : ''} aria-pressed={tab === item.id} onclick={() => tab = item.id}><item.icon size={14} />{item.label}</Button>
        {/each}
        <Button variant="outline" size="icon" class="ml-auto" aria-label="Training tools settings" onclick={() => studio.openTools = true}><Settings size={14} /></Button>
      </div>
      {#if busy === "Training"}<div role="status" class="absolute left-1/2 top-[104px] z-20 flex -translate-x-1/2 items-center gap-2 rounded-lg border border-border bg-surface px-4 py-2 shadow-lg"><LoaderCircle size={15} class="animate-spin motion-reduce:animate-none" />Training · {elapsed}s</div>{/if}
      <div class="min-h-0 flex-1 overflow-auto p-5">
        {#if error}<p role="alert" class="mb-4 text-error">{error}</p>{/if}
        {#if tab === 'setup'}
          <h2 class="text-lg font-medium">{experiment ? experiment.name : 'New experiment'}</h2>
          <p class="mt-2 max-w-3xl text-xs leading-relaxed text-muted">Learn joint-position feedback in MuJoCo. Experiments pin a robot, environment, task, and training configuration. The same snapshot runs locally or on your cloud host.</p>
          {#if !experiment}
            <form class="mt-5 max-w-3xl space-y-4" onsubmit={e => { e.preventDefault(); pin(); }}>
              <fieldset disabled={!!busy} class="grid gap-4 sm:grid-cols-2">
                <label class="sm:col-span-2">Experiment name<Input class="mt-1 w-full" bind:value={name} placeholder="Joint reach baseline" required maxlength={200} /></label>
                <label>Robot version<NativeSelect class="mt-1 w-full" bind:value={versionId} required><option value="">Save a robot in Builder</option>{#each versions as v}<option value={v.id}>{v.name} · {new Date(v.savedAt).toLocaleString()}</option>{/each}</NativeSelect></label>
                <label>Environment<NativeSelect class="mt-1 w-full" bind:value={environmentId} required><option value="">Create an environment below</option>{#each catalog.environments as e}<option value={e.id}>{e.name} · {e.gravity} m/s² · {e.duration}s</option>{/each}</NativeSelect></label>
                <label class="sm:col-span-2">Task<NativeSelect class="mt-1 w-full" bind:value={taskId} required><option value="">Create a task below</option>{#each catalog.tasks as t}<option value={t.id}>{t.name} · target {percent(t.targetFraction)} · tolerance {percent(t.tolerance)}</option>{/each}</NativeSelect></label>
                <label>Iterations<Input type="number" class="mt-1 w-full" bind:value={iterations} min={1} max={20} required /></label>
                <label>Population<Input type="number" class="mt-1 w-full" bind:value={population} min={4} max={32} required /></label>
                <label>Training seed<Input type="number" class="mt-1 w-full" bind:value={seed} min={0} max={99999} required /></label>
              </fieldset>
              <p class="text-xs text-muted">CEM optimizes feedback gains over 3 training starts. Evaluation uses 5 held-out starts. Fixed base, finite revolute/prismatic joints, position servos; no camera or demonstration learning.</p>
              <div class="flex gap-2"><Button type="submit" variant="default" disabled={!!busy || !loopId || !versionId || !taskId || !environmentId}>Save experiment</Button>{#if unsaved}<Button disabled={!!busy} onclick={discardDraft}>Discard draft</Button>{/if}</div>
              {#if !loopId}<p class="text-xs text-muted">Create or select an Experience and Loop in the left panel.</p>{/if}
            </form>
            <div class="mt-6 grid max-w-3xl gap-4 sm:grid-cols-2">
              <details bind:open={newEnvironmentOpen} class="rounded border border-border p-3"><summary>New environment</summary><form class="mt-3 space-y-3" onsubmit={e => { e.preventDefault(); create('environments', { name: environmentName, gravity, duration }); }}><label class="block">Environment name<Input bind:value={environmentName} required maxlength={200} /></label><label class="block">Gravity (m/s²)<Input type="number" bind:value={gravity} min={0} max={20} step={0.01} required /></label><label class="block">Episode duration (s)<Input type="number" bind:value={duration} min={0.1} max={5} step={0.1} required /></label><p class="text-xs text-muted">Fixed root at saved placement, floor at Z=0, friction 0.8.</p><Button type="submit" disabled={!!busy}>Save environment</Button></form></details>
              <details bind:open={newTaskOpen} class="rounded border border-border p-3"><summary>New task</summary><form class="mt-3 space-y-3" onsubmit={e => { e.preventDefault(); create('tasks', { name: taskName, targetFraction, tolerance }); }}><label class="block">Task name<Input bind:value={taskName} required maxlength={200} /></label><label class="block">Target fraction of joint range<Input type="number" bind:value={targetFraction} min={0.05} max={0.95} step={0.01} required /></label><label class="block">Success tolerance (fraction)<Input type="number" bind:value={tolerance} min={0.001} max={0.25} step={0.001} required /></label><p class="text-xs text-muted">At episode end, every actuated joint must be within tolerance of the target. Fractions allow different robot designs to share this task.</p><Button type="submit" disabled={!!busy}>Save task</Button></form></details>
            </div>
          {:else}
            <dl class="mt-5 grid max-w-3xl gap-3 rounded border border-border p-4 text-xs sm:grid-cols-2"><div>Experience / Loop<p class="mt-1 text-title">{experiment.experience.name} / {experiment.loop.name}</p></div><div>Robot version<p class="mt-1 break-all text-title">{experiment.robotName ?? experiment.robot?.name} · {experiment.versionId}</p></div><div>Environment<p class="mt-1 text-title">{experiment.environment.name} · {experiment.environment.gravity} m/s² · {experiment.environment.duration}s</p></div><div>Task<p class="mt-1 text-title">{experiment.task.name} · target {percent(experiment.task.targetFraction)} · tolerance {percent(experiment.task.tolerance)}</p></div><div>Trainer<p class="mt-1 text-title">CEM · {experiment.config.iterations} iterations × {experiment.config.population} candidates · seed {experiment.config.seed}</p></div><div>Experiment ID<p class="mt-1 break-all font-mono text-title">{experiment.id}</p></div></dl>
            <fieldset disabled={!!busy} class="mt-5 max-w-lg space-y-3"><label class="block">Compute<NativeSelect class="mt-1 w-full" bind:value={compute}><option value="local">Local · MuJoCo</option><option value="cloud">Cloud · SSH host</option><option value="runpod">Cloud · provision RunPod GPU</option></NativeSelect></label>
            {#if compute === 'runpod'}<p class="text-xs text-muted">Open Compute to review costs and provision a persistent cloud job for this experiment. The current MuJoCo trainer uses CPU.</p>{:else if compute === 'cloud'}<label class="block">SSH host<Input class="mt-1 w-full" bind:value={host} placeholder="user@host or SSH config alias" /></label><label class="block">Cloud Python executable<Input class="mt-1 w-full" bind:value={cloudPython} /></label><p class="text-xs leading-relaxed text-muted">Uses your existing SSH keys and known hosts. Install MuJoCo 3.2.7 and NumPy on the host first. Launch sends this robot snapshot and its meshes to the host. CPU training; no provisioning or GPU acceleration. Keep Humaan connected until results return.</p>{:else}<p class="text-xs text-muted">Uses the MuJoCo runtime selected in Settings → Tools. Worker limit: 180 seconds.</p>{/if}
            {#if compute === 'runpod'}<Button onclick={() => { studio.cloudExperimentId = experiment!.id; studio.activeView = 'cloud'; }}>Configure cloud job</Button>{:else}<Button variant="default" disabled={compute === 'cloud' && (!host.trim() || !cloudPython.trim())} onclick={launch}><Play size={14} />Launch training</Button>{/if}</fieldset>
          {/if}
        {:else if tab === 'results'}
          {#if run}<h2 class="text-lg">{run.name} · {run.status}</h2><p class="mt-2 break-all text-xs text-muted">{run.experiment.experience.name} / {run.experiment.loop.name} · {run.execution.kind} · Robot version {run.versionId}</p>
            {#if run.result?.error || run.error}<p role="alert" class="mt-4 text-error">{run.result?.error ?? run.error}</p>{/if}
            {#if run.status === 'completed' && run.result}
              <div class="my-4 grid gap-3 text-xs sm:grid-cols-3"><div class="rounded border border-border p-3">Evaluation success<p class="mt-1 text-lg">{percent(run.result.evaluation.successRate)}</p><p class="text-muted">5 held-out starts · baseline {percent(run.result.baseline.successRate)}</p></div><div class="rounded border border-border p-3">Mean final error<p class="mt-1 text-lg">{percent(run.result.evaluation.meanError)}</p><p class="text-muted">Fraction of joint range</p></div><div class="rounded border border-border p-3">Mean trajectory error<p class="mt-1 text-lg">{percent(run.result.evaluation.meanLoss)}</p><p class="text-muted">Baseline {percent(run.result.baseline.meanLoss)}</p></div></div>
              <div class="relative flex h-80 overflow-hidden rounded border border-border"><BuilderViewport robot={run.experiment.robot} selected="" onselect={() => {}} poses={frames[frameIndex]?.poses} floorHeight={0} /></div>
              <div class="my-3 flex items-center gap-3"><Button onclick={() => { if (frameIndex >= frames.length-1) frameIndex = 0; playing = !playing; }}>{playing ? 'Pause' : 'Play evaluation'}</Button><input class="min-w-0 flex-1" type="range" aria-label="Training evaluation timeline" min={0} max={Math.max(0,frames.length-1)} bind:value={frameIndex} oninput={() => playing = false} /><span class="text-xs">{frames[frameIndex]?.time.toFixed(2)}s</span></div>
              <details class="mt-4"><summary>Training metrics</summary><table class="mt-2 w-full text-left text-xs"><thead><tr><th>Iteration</th><th>Best training trajectory error</th></tr></thead><tbody>{#each run.result.history as metric}<tr class="border-t border-border"><td class="py-2">{metric.iteration}</td><td>{percent(metric.loss)}</td></tr>{/each}</tbody></table></details>
              <Button class="mt-4" disabled={studio.deploymentBusy} onclick={() => { studio.policyRunId = run!.id; studio.deploymentSection = 'policies'; studio.activeView = 'deployments'; }}>Manage policy in Deployments</Button>
              <details class="mt-4"><summary>Policy artifact · {run.result.policy.format}</summary><p class="my-2 text-xs text-muted">Observation: joint position and velocity. Action: clamped position-servo target. Stored with this run and its pinned experiment; physical deployment is not enabled.</p><pre class="overflow-auto rounded bg-surface p-3 text-xs">{JSON.stringify(run.result.policy,null,2)}</pre></details>
              <details class="mt-4"><summary>Runtime and assumptions</summary><p class="mt-2 text-xs">{run.result.engine}</p>{#each run.result.warnings as warning}<p class="mt-2 text-xs text-muted">{warning}</p>{/each}</details>
            {/if}
          {:else}<p class="text-muted">Select a saved run to inspect its policy, evaluation, and robot playback.</p>{/if}
        {:else}
          <h2 class="text-lg">Compare runs</h2><p class="my-3 max-w-3xl text-xs text-muted">Select a completed run as the reference. Comparisons match environment settings, task targets and tolerance, evaluation seeds, trainer adapter, and MuJoCo version. Different embodiments can still differ in physical difficulty.</p>
          <NativeSelect aria-label="Comparison reference" value={run?.id ?? ''} disabled={!!busy} onchange={e => { const id = e.currentTarget.value; if (id) void work('Loading comparison', async () => { run = await invoke<TrainingRun>('training_load',{root,kind:'runs',id}); }); }}><option value="">Select completed run</option>{#each catalog.runs.filter(r => r.status === 'completed') as r}<option value={r.id}>{r.name} · {r.robotName} · {new Date(r.createdAt).toLocaleString()}</option>{/each}</NativeSelect>
          <div class="mt-4 overflow-x-auto">
            <table class="w-full text-left text-sm" aria-label="Training run comparison">
              <thead class="border-b border-border text-xs text-muted">
                <tr>{#each ['Run / Robot version', 'Compute', 'Success', 'Final error', 'Trajectory error'] as heading}<th class="px-3 py-3 font-medium">{heading}</th>{/each}</tr>
              </thead>
              <tbody>
                {#each comparison as r}
                  <tr class="border-b border-border hover:bg-hover">
                    <td class="px-3 py-3"><button class="text-left font-medium text-title hover:underline focus-visible:outline focus-visible:outline-ring disabled:opacity-45" disabled={!!busy} onclick={() => openRun(r.id)}>{r.name}</button><p class="mt-1 text-xs text-muted">{r.robotName} · {r.versionId}</p></td>
                    <td class="px-3 py-3">{r.execution.kind}</td>
                    <td class="px-3 py-3 tabular-nums">{percent(r.evaluation!.successRate)}</td>
                    <td class="px-3 py-3 tabular-nums">{percent(r.evaluation!.meanError)}</td>
                    <td class="px-3 py-3 tabular-nums">{percent(r.evaluation!.meanLoss)}</td>
                  </tr>
                {:else}
                  <tr><td colspan="5" class="px-3 py-8 text-center text-muted">No comparable completed runs selected.</td></tr>
                {/each}
              </tbody>
            </table>
          </div>
        {/if}
      </div>
      </div>
    </section>
  </WorkspaceColumn>
</div>
