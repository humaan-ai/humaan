<script lang="ts">
  import { onMount, untrack } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { toast } from 'svelte-sonner';
  import { studio } from '$lib/studio.svelte';
  import { emptyRobot, type Robot, type SavedVersion } from '$lib/builder/model';
  import { simulationChecks, type RunSummary, type SimulationRun } from '$lib/simulation/model';
  import PanelResizeHandle from './PanelResizeHandle.svelte';
  import X from '@lucide/svelte/icons/x';
  import ScanSearch from '@lucide/svelte/icons/scan-search';
  import StudioSidebar from './StudioSidebar.svelte';
  import WorkspaceColumn from './WorkspaceColumn.svelte';
  import BuilderViewport from './BuilderViewport.svelte';
  import { Button } from '$lib/components/ui/button/index.js';
  import { Input } from '$lib/components/ui/input/index.js';
  import { NativeSelect } from '$lib/components/ui/native-select/index.js';
  import LoaderCircle from '@lucide/svelte/icons/loader-circle';
  import * as Dialog from '$lib/components/ui/dialog/index.js';
  import Check from '@lucide/svelte/icons/check';
  import Trash2 from '@lucide/svelte/icons/trash-2';
  import ChevronRight from '@lucide/svelte/icons/chevron-right';
  import Play from '@lucide/svelte/icons/play';
  import Pause from '@lucide/svelte/icons/pause';
  import RotateCcw from '@lucide/svelte/icons/rotate-ccw';
  import Settings from '@lucide/svelte/icons/settings';
  import RefreshCw from '@lucide/svelte/icons/refresh-cw';
  import CircleAlert from '@lucide/svelte/icons/circle-alert';
  import TriangleAlert from '@lucide/svelte/icons/triangle-alert';
  let { root }: { root: string } = $props();
  let versions = $state<SavedVersion[]>([]), runs = $state<RunSummary[]>([]);
  let versionId = $state(''), robot = $state<Robot>(emptyRobot()), selected = $state('');
  let duration = $state(3), gravity = $state(9.81), targets = $state<Record<string, number>>({});
  let busy = $state(false), error = $state(''), run = $state<SimulationRun | null>(null), frameIndex = $state(0), playing = $state(false);
  const checks = $derived(simulationChecks(robot));
  const frames = $derived(run?.result.frames || []);
  const displayed = $derived(run?.robot || robot);
  const frame = $derived(frames[frameIndex]);
  let inspectorOpen = $state(true), inspectorWidth = $state(280);
  let inspectorTab = $state<'preflight' | 'details'>('preflight');
  const inspectorChecks = $derived(simulationChecks(displayed).toSorted((a, b) => Number(b.severity === 'error') - Number(a.severity === 'error')));
  const inspectorStatus = $derived(run ? run.status === 'completed' ? 'Completed' : 'Failed' : !versionId ? 'No robot' : inspectorChecks.length ? `${inspectorChecks.length} issue${inspectorChecks.length === 1 ? '' : 's'}` : 'Ready');
  const statusTone = $derived(run ? run.status === 'completed' ? 'success' : 'error' : !versionId ? 'neutral' : inspectorChecks.some(c => c.severity === 'error') ? 'error' : inspectorChecks.length ? 'warning' : 'success');
  const statusHint = $derived(statusTone === 'error' ? run ? 'Run failed. Open Inspector for details.' : 'Simulation is blocked. Review preflight issues.' : statusTone === 'warning' ? 'Review preflight warnings before running.' : 'Open Inspector');
  $effect(() => { inspectorTab = run ? 'details' : 'preflight'; });
  function setInspector(open: boolean) {
    inspectorOpen = open;
    localStorage.setItem('humaan-simulation-inspector-open', String(open));
  }
  function resizeInspector(width: number) {
    inspectorWidth = width;
    localStorage.setItem('humaan-simulation-inspector-width', String(width));
  }
  let deleteOpen = $state(false), pendingDelete = $state<RunSummary | null>(null);
  async function deleteRun() {
    const item = pendingDelete; if (!item || busy) return;
    deleteOpen = false; pendingDelete = null;
    await work(async () => {
      await invoke('simulation_delete', { root, id: item.id });
      if (run?.id === item.id) { playing = false; run = null; frameIndex = 0; selected = robot.links[0]?.id || ''; }
      await refresh();
      toast.success(`Deleted saved run ${item.name}.`, { id: 'simulation-delete' });
    }, 'Deleting saved run…');
  }
  let progressLabel = $state('Preparing simulation…');
  let elapsed = $state(0);
  let disposed = false;
  $effect(() => {
    if (!busy) return;
    const started = Date.now();
    const timer = setInterval(() => elapsed = Math.floor((Date.now() - started) / 1000), 1000);
    return () => clearInterval(timer);
  });
  let runtimeError = $state('');
  const runtimeBadge = $derived(studio.simulationPython ? 'Custom' : runtimeError ? 'Error' : studio.simulationRuntime?.installing ? 'Installing' : studio.simulationRuntime?.ready ? 'Ready' : studio.simulationRuntime ? 'Not installed' : 'Checking');
  const runtimeDetail = $derived(studio.simulationPython ? 'Custom Python runtime' : runtimeError ? `Cannot check MuJoCo: ${runtimeError}` : studio.simulationRuntime?.phase || 'Checking MuJoCo…');
  async function refreshRuntime() {
    try {
      const status = await invoke<NonNullable<typeof studio.simulationRuntime>>('simulation_tools_status');
      if (!disposed) { studio.simulationRuntime = status; runtimeError = ''; }
    } catch (cause) { if (!disposed) runtimeError = String(cause); }
  }
  async function work(action: () => Promise<void>, label = 'Preparing simulation…') { if (busy) return; progressLabel = label; elapsed = 0; busy = true; error = ''; try { await action(); } catch (cause) { if (!disposed) error = String(cause); } finally { busy = false; } }
  async function refresh() {
    [versions, runs] = await Promise.all([invoke<SavedVersion[]>('builder_list', { root, kind: 'versions' }), invoke<RunSummary[]>('simulation_list', { root })]);
    if (versionId && !versions.some(v => v.id === versionId)) { versionId = ''; robot = emptyRobot(); }
  }
  async function choose(id: string) {
    playing = false; run = null; frameIndex = 0; versionId = ''; robot = emptyRobot();
    const next = await invoke<Robot>('builder_load', { root, kind: 'versions', id });
    robot = next; versionId = id; selected = next.links[0]?.id || '';
    targets = Object.fromEntries(next.joints.filter(j => j.type !== 'fixed').map(j => [j.id, j.type === 'continuous' ? 0 : Math.max(j.lower, Math.min(j.upper, 0))]));
  }
  async function launch() {
    playing = false;
    await work(async () => {
      const result = await invoke<SimulationRun>('simulation_run', { root, versionId, python: studio.simulationPython, config: { duration, gravity, targets: $state.snapshot(targets) } });
      if (disposed) return;
      run = result; frameIndex = 0; selected = result.robot.links[0]?.id || ''; progressLabel = 'Loading results…'; await refresh();
      if (result.status === 'completed') toast.success('Simulation completed. Results saved.');
      else toast.error('Simulation failed. See the saved run for details.');
    }, 'Running simulation…');
  }
  async function openRun(id: string) { playing = false; await work(async () => { run = await invoke<SimulationRun>('simulation_load', { root, id }); frameIndex = 0; selected = run.robot.links[0]?.id || ''; }, 'Loading saved run…'); }
  $effect(() => { studio.simulationBusy = busy; });
  $effect(() => {
    if (studio.activeView !== 'simulation') playing = false;
    else untrack(() => { void refreshRuntime(); if (!studio.simulationVersion) void work(refresh); });
  });
  onMount(() => {
    inspectorOpen = localStorage.getItem('humaan-simulation-inspector-open') !== 'false';
    const savedWidth = Number(localStorage.getItem('humaan-simulation-inspector-width'));
    if (Number.isFinite(savedWidth) && savedWidth >= 240) inspectorWidth = Math.min(440, savedWidth);
    const savedPython = localStorage.getItem('humaan-simulation-python') || '';
    // Earlier Simulation builds persisted their default system interpreter.
    // Only migrate that implicit default; explicit custom choices stay custom.
    if (savedPython === 'python3' && !localStorage.getItem('humaan-simulation-runtime-mode')) {
      localStorage.removeItem('humaan-simulation-python');
      studio.simulationPython = '';
    } else studio.simulationPython = savedPython;
    void refreshRuntime();
    void work(refresh);
    const timer = setInterval(() => { if (playing && frameIndex < frames.length - 1) frameIndex++; else playing = false; }, 50);
    return () => { disposed = true; clearInterval(timer); studio.simulationBusy = false; };
  });
  $effect(() => {
    const request = studio.simulationVersion;
    if (studio.activeView === 'simulation' && request && !busy) {
      studio.simulationVersion = '';
      void work(async () => { await refresh(); await choose(request); });
    }
  });
</script>

<svelte:window onbeforeunload={(event) => { if (busy) { event.preventDefault(); event.returnValue = ''; } }} />

<div class="flex h-full min-h-0 gap-1">
  <StudioSidebar title="Simulation">
    <div class="flex h-7 shrink-0 items-center justify-between border-b border-border px-2 text-xs text-muted"><div class="flex min-w-0 items-center gap-2"><span class="shrink-0">Local · MuJoCo</span><span role="status" aria-label="Simulation runtime status" title={runtimeDetail} class="inline-flex min-w-0 items-center gap-1 rounded border px-1.5 py-0.5 text-[10px] leading-none {runtimeBadge === 'Ready' ? 'border-success/30 bg-success/10 text-success' : runtimeBadge === 'Error' ? 'border-error/30 bg-error/10 text-error' : 'border-border bg-surface text-muted'}"><span class="size-1 shrink-0 rounded-full bg-current" aria-hidden="true"></span><span class="truncate">{runtimeBadge}</span><span class="sr-only"> · {runtimeDetail}</span></span></div><div class="flex shrink-0 items-center"><Button variant="ghost" size="icon" class="size-6" aria-label="Simulation settings" title="Simulation settings" onclick={() => studio.openTools = true}><Settings size={13} /></Button><Button variant="ghost" size="icon" class="size-6" aria-label="Refresh simulation versions" disabled={busy} onclick={() => work(refresh)}><RefreshCw size={13} /></Button></div></div>
    <div class="flex min-h-0 min-w-0 flex-1 flex-col gap-3 overflow-y-auto p-3 text-xs">
      <label class="flex min-w-0 flex-col gap-1">Robot version<NativeSelect class="w-full min-w-0 max-w-full" aria-label="Simulation robot version" value={versionId} disabled={busy} onchange={event => { const id = event.currentTarget.value; if (id) void work(() => choose(id)); }}><option value="" disabled>Select saved version</option>{#each versions as v}<option value={v.id}>{v.name} · {new Date(v.savedAt).toLocaleString()}</option>{/each}</NativeSelect></label>
      {#if !versions.length}<p class="text-muted">Save a robot version in Builder to begin.</p>{/if}
      <label class="flex flex-col gap-1">Duration (seconds)<Input type="number" min="0.1" max="10" step="0.1" aria-label="Simulation duration" bind:value={duration} disabled={busy} /></label>
      <label class="flex flex-col gap-1">Gravity (m/s²)<Input type="number" min="0" max="20" step="0.01" aria-label="Simulation gravity" bind:value={gravity} disabled={busy} /></label>
      <p class="text-muted">Fixed base · Floor at 0 m · 2 ms physics step · 20 Hz playback</p>
      {#if robot.joints.some(j => j.type !== 'fixed')}<h3 class="font-medium">Joint position targets</h3>{/if}
      {#each robot.joints.filter(j => j.type !== 'fixed') as j}<label class="flex flex-col gap-1">{j.name} ({j.type === 'prismatic' ? 'm' : 'rad'})<Input type="number" step="0.01" min={j.type === 'continuous' ? undefined : j.lower} max={j.type === 'continuous' ? undefined : j.upper} aria-label={`Target ${j.name}`} bind:value={targets[j.id]} disabled={busy} /></label>{/each}
      <Button variant="default" disabled={busy || !versionId || checks.some(c => c.severity === 'error') || !Number.isFinite(duration) || duration < .1 || duration > 10 || !Number.isFinite(gravity) || gravity < 0 || gravity > 20} onclick={launch}><Play size={14} />{busy ? 'Working…' : 'Run simulation'}</Button>
      <details open class="group/runs -mx-3 border-t border-border">
        <summary class="flex h-7 cursor-pointer list-none items-center gap-1 px-2 text-xs text-title [&::-webkit-details-marker]:hidden"><ChevronRight size={12} class="group-open/runs:rotate-90" />Saved runs<span class="ml-auto text-[10px] text-muted">{runs.length}</span></summary>
        <div class="pb-2" role="group" aria-label="Saved runs">
          {#each runs as item (item.id)}
            <div class="group/run flex items-center {run?.id === item.id ? 'bg-selection' : 'hover:bg-hover'}">
              <button class="flex min-w-0 flex-1 items-center gap-2 px-3 py-1.5 text-left text-xs {run?.id === item.id ? 'bg-selection text-title' : 'text-muted hover:bg-hover'}" disabled={busy} onclick={() => { if (run?.id !== item.id) void openRun(item.id); }} aria-pressed={run?.id === item.id} title={`${item.name} · ${item.status} · ${new Date(item.createdAt).toLocaleString()} · ${item.id}`}>
                <span class="flex w-3 shrink-0 items-center">{#if run?.id === item.id}<Check size={13} aria-hidden="true" />{/if}</span>
                <span class="min-w-0 flex-1"><span class="block truncate">{item.name} · {item.status}</span><span class="block truncate text-[10px] text-muted">{new Date(item.createdAt).toLocaleString()}</span></span>
              </button>
              <Button size="icon" variant="ghost" class="mr-1 size-6 shrink-0 opacity-0 group-hover/run:opacity-100 group-focus-within/run:opacity-100 hover:text-error" disabled={busy} aria-label={`Delete run ${item.name}`} title="Delete saved run" onclick={() => { pendingDelete = item; deleteOpen = true; }}><Trash2 size={13} /></Button>
            </div>
          {:else}<p class="px-3 py-2 text-xs text-muted">No simulation runs yet.</p>{/each}
        </div>
      </details>
    </div>
  </StudioSidebar>
  <WorkspaceColumn active={studio.activeView === 'simulation'}>
    <section class="workbench-panel relative flex min-h-0 flex-1 flex-col" aria-label="Simulation workspace" aria-busy={busy}>
      <header class="flex h-[35px] shrink-0 items-center gap-2 border-b border-border px-3 text-xs">
        <span class="min-w-0 truncate">{run ? `${run.name} · ${run.status}` : 'Simulation setup'}</span>
        <button
          aria-label={`Simulation status: ${inspectorStatus}`}
          aria-expanded={inspectorOpen}
          aria-controls="simulation-inspector"
          title={statusHint}
          class="inline-flex h-6 shrink-0 items-center gap-1.5 rounded-md border px-2 text-[11px] font-semibold leading-none transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-current focus-visible:ring-offset-2 focus-visible:ring-offset-surface {statusTone === 'error' ? 'border-error/60 bg-error/20 text-error hover:bg-error/30' : statusTone === 'warning' ? 'border-warning/50 bg-warning/15 text-warning hover:bg-warning/25' : statusTone === 'success' ? 'border-success/30 bg-success/10 text-success hover:bg-success/20' : 'border-border bg-surface text-muted hover:bg-hover'}"
          onclick={() => { inspectorTab = run ? 'details' : 'preflight'; setInspector(true); }}
        >
          {#if statusTone === 'error'}<CircleAlert size={13} strokeWidth={2.25} aria-hidden="true" />
          {:else if statusTone === 'warning'}<TriangleAlert size={13} strokeWidth={2.25} aria-hidden="true" />
          {:else if statusTone === 'success'}<Check size={13} strokeWidth={2.25} aria-hidden="true" />{/if}
          <span>{inspectorStatus}</span>
          {#if statusTone === 'error' || statusTone === 'warning'}<ChevronRight size={12} aria-hidden="true" />{/if}
        </button>
        <div class="ml-auto flex shrink-0 items-center gap-1">
          {#if run}<Button size="sm" variant="ghost" disabled={busy} onclick={() => { run = null; playing = false; }}>Setup</Button>{/if}
          <Button size="sm" variant="ghost" class={inspectorOpen ? 'text-title' : 'text-muted'} aria-label="Toggle simulation Inspector" title={inspectorOpen ? 'Hide Inspector' : 'Show Inspector'} aria-expanded={inspectorOpen} aria-controls="simulation-inspector" onclick={() => setInspector(!inspectorOpen)}>
            <ScanSearch size={18} strokeWidth={1.5} aria-hidden="true" />
            Inspector
          </Button>
        </div>
      </header>
      {#if error}<p role="alert" class="p-3 text-xs text-error">{error}</p>{/if}
      {#if busy}
        <div class="pointer-events-none absolute inset-x-3 top-[47px] z-20 flex justify-center">
          <div role="status" aria-label="Simulation progress" class="flex max-w-full items-center gap-3 rounded-lg border border-border-strong bg-surface/95 px-4 py-3 shadow-lg backdrop-blur-sm">
            <LoaderCircle size={16} class="shrink-0 animate-spin text-muted motion-reduce:animate-none" aria-hidden="true" />
            <span class="truncate text-xs text-title">{progressLabel}</span>
            <span class="shrink-0 text-xs tabular-nums text-muted" aria-hidden="true">{elapsed}s</span>
          </div>
        </div>
      {/if}
      <div class="flex min-h-0 flex-1">
        <BuilderViewport robot={displayed} {selected} onselect={id => selected = id} poses={frame?.poses} floorHeight={run ? run.result.floorHeight ?? -0.01 : 0} />
        {#if inspectorOpen}
        <aside id="simulation-inspector" class="relative ml-1 flex min-h-0 shrink-0 flex-col border-l border-border text-xs" style:width={`${inspectorWidth}px`} aria-label="Simulation Inspector">
          <PanelResizeHandle label="Resize simulation Inspector" width={inspectorWidth} min={240} max={440} edge="left" onresize={resizeInspector} />
          <div class="flex h-[35px] shrink-0 items-center justify-between border-b border-border px-3"><span class="font-medium">Inspector</span><Button variant="ghost" size="icon-xs" aria-label="Close simulation Inspector" onclick={() => setInspector(false)}><X size={14} /></Button></div>
          <div class="flex shrink-0 gap-1 border-b border-border p-2" role="group" aria-label="Inspector sections">
            <Button size="sm" variant="ghost" class={inspectorTab === 'preflight' ? 'bg-selection text-title' : ''} aria-pressed={inspectorTab === 'preflight'} onclick={() => inspectorTab = 'preflight'}>Preflight</Button>
            <Button size="sm" variant="ghost" class={inspectorTab === 'details' ? 'bg-selection text-title' : ''} aria-pressed={inspectorTab === 'details'} disabled={!run} onclick={() => inspectorTab = 'details'}>Run details</Button>
          </div>
          <div class="min-h-0 flex-1 overflow-y-auto p-3">
          {#if inspectorTab === 'details' && run}
            <p class="mt-2 break-all text-muted">Robot version: {run.versionId}</p><p class="mt-1 text-muted">{run.result.engine || 'Runtime unavailable'} · {run.config.duration} s · Gravity {run.config.gravity}</p>
            {#if run.result.error}<div class="mt-3 rounded border border-border p-2"><h4 class="font-medium">Saved run error</h4><p class="mt-1 text-muted">Recorded {new Date(run.createdAt).toLocaleString()}. This describes the runtime used for that attempt.</p><p role="alert" class="mt-2 text-error">{run.result.error}</p>{#if !studio.simulationPython && studio.simulationRuntime?.ready}<p class="mt-2 text-success">Managed MuJoCo is ready now. Start a new run to use it.</p>{/if}</div>{/if}
            {#if run.status === 'completed'}<p class="mt-3">Peak contacts: {run.result.maxContacts}</p><p class="mt-1 text-muted">A completed run is a physics check, not a task success evaluation.</p>{/if}
            {#each Object.entries(frame?.joints || {}) as [id, value]}<p class="mt-2">{run.robot.joints.find(j => j.id === id)?.name}: {value.toFixed(3)}</p>{/each}
            {#each run.result.warnings || [] as warning}<p class="mt-3 text-warning">{warning}</p>{/each}
          {:else}
            {#if !run && !versionId}<p class="text-muted">Select a saved robot version to check its simulation properties.</p>{:else}
            <div class="flex items-center justify-between"><h3 class="font-medium">Preflight checks</h3><span class="text-muted">{inspectorChecks.length ? `${inspectorChecks.length} issues` : 'Ready'}</span></div>
            {#if run}<p class="mt-2 text-muted">Checks for this run’s saved robot snapshot.</p>{/if}
            {#each inspectorChecks as check}<button class="mt-2 block w-full text-left {check.severity === 'error' ? 'text-error' : 'text-warning'}" onclick={() => { if (check.linkId) selected = check.linkId; }}>{check.message}</button>{:else}<p class="mt-2 text-success">Ready for local simulation.</p>{/each}
            <p class="mt-4 text-muted">Edit properties in Builder and save a new version to resolve issues. Unsaved Builder changes are never used by a run.</p>
            {/if}
          {/if}
          </div>
        </aside>
        {/if}
      </div>
      {#if frames.length}<div class="flex h-10 shrink-0 items-center gap-2 border-t border-border px-3 text-xs"><Button size="sm" aria-label={playing ? 'Pause simulation playback' : 'Play simulation playback'} onclick={() => { if (frameIndex === frames.length - 1) frameIndex = 0; playing = !playing; }}>{#if playing}<Pause size={14} />{:else}<Play size={14} />{/if}</Button><Button size="sm" aria-label="Reset simulation playback" onclick={() => { playing = false; frameIndex = 0; }}><RotateCcw size={14} /></Button><input class="min-w-0 flex-1" type="range" aria-label="Simulation timeline" min="0" max={frames.length-1} step="1" bind:value={frameIndex} oninput={() => playing = false} /><span>{(frame?.time || 0).toFixed(2)} / {frames.at(-1)?.time.toFixed(2)} s</span></div>{/if}
    </section>
  </WorkspaceColumn>
</div>


<Dialog.Root bind:open={deleteOpen}>{#if deleteOpen && pendingDelete}<Dialog.Content>
  <Dialog.Title>Delete saved run?</Dialog.Title>
  <Dialog.Description>Delete {pendingDelete.name} from {new Date(pendingDelete.createdAt).toLocaleString()}? This removes its recorded results and cannot be undone. Saved robot versions are kept.</Dialog.Description>
  <div class="mt-4 flex justify-end gap-2"><Button onclick={() => { deleteOpen = false; pendingDelete = null; }}>Cancel</Button><Button variant="destructive" onclick={deleteRun}>Delete run</Button></div>
</Dialog.Content>{/if}</Dialog.Root>
