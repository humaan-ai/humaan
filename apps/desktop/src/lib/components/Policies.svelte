<script lang="ts">
  import { untrack, onDestroy } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { toast } from 'svelte-sonner';
  import SlidersHorizontal from '@lucide/svelte/icons/sliders-horizontal';
  import Plus from '@lucide/svelte/icons/plus';
  import RefreshCw from '@lucide/svelte/icons/refresh-cw';
  import Check from '@lucide/svelte/icons/check';
  import RotateCcw from '@lucide/svelte/icons/rotate-ccw';
  import Play from '@lucide/svelte/icons/play';
  import Pause from '@lucide/svelte/icons/pause';
  import { studio } from '$lib/studio.svelte';
  import type { Catalog } from '$lib/training/model';
  import type { Policy, PolicySummary, Assignment, Compatibility } from '$lib/deployments/policies';
  import StudioSidebar from './StudioSidebar.svelte';
  import WorkspaceColumn from './WorkspaceColumn.svelte';
  import DeploymentNavigation from './DeploymentNavigation.svelte';
  import BuilderViewport from './BuilderViewport.svelte';
  import { Button } from '$lib/components/ui/button/index.js';
  import { Input } from '$lib/components/ui/input/index.js';
  import { NativeSelect } from '$lib/components/ui/native-select/index.js';
  import * as Dialog from '$lib/components/ui/dialog/index.js';
  let { root, blocked, onsection }: { root: string; blocked: boolean; onsection: (section: 'robots' | 'policies' | 'sessions') => void } = $props();
  let policies = $state<PolicySummary[]>([]), robots = $state<{ id: string; name: string }[]>([]), runs = $state<Catalog['runs']>([]);
  let policy = $state<Policy | null>(null), robotId = $state(''), assignment = $state<Assignment | null>(null), compatibility = $state<Compatibility | null>(null);
  let busy = $state(false), error = $state(''), query = $state(''), addOpen = $state(false), runId = $state(''), name = $state('');
  let tab = $state<'overview' | 'replay' | 'controller'>('overview'), frameIndex = $state(0), playing = $state(false);
  let disposed = false, initialized = false;
  const current = $derived(assignment?.history.at(-1));
  const frames = $derived(policy?.run.result?.frames || []);
  async function work(action: () => Promise<void>) {
    if (busy || blocked) return; busy = true; studio.policyBusy = true; error = '';
    try { await action(); } catch (cause) { if (!disposed) error = String(cause); } finally { busy = false; studio.policyBusy = false; }
  }
  async function refresh() {
    [policies, robots] = await Promise.all([invoke<PolicySummary[]>('policy_list', { root }), invoke<typeof robots>('deployment_list', { root, kind: 'robots' })]);
  }
  async function check() {
    compatibility = null; assignment = null;
    if (!robotId) return;
    assignment = await invoke<Assignment>('policy_assignment', { root, robotId });
    if (policy) compatibility = await invoke<Compatibility>('policy_check', { root, policyId: policy.id, robotId });
  }
  async function choose(id: string) {
    await work(async () => { playing = false; policy = null; compatibility = null; policy = await invoke<Policy>('policy_load', { root, id }); tab = 'overview'; frameIndex = 0; await check(); });
  }
  async function add() {
    await work(async () => { const catalog = await invoke<Catalog>('training_catalog', { root }); runs = catalog.runs.filter(run => run.status === 'completed'); runId = studio.policyRunId || ''; studio.policyRunId = ''; name = runs.find(r => r.id === runId)?.name || ''; addOpen = true; });
  }
  async function promote() {
    await work(async () => { policy = await invoke<Policy>('policy_promote', { root, runId, name: name.trim() }); addOpen = false; frameIndex = 0; playing = false; tab = 'overview'; await refresh(); await check(); toast.success('Policy saved to Deployments.'); });
  }
  async function assign(policyId: string | null, restoreId: string | null = null) {
    await work(async () => { assignment = await invoke<Assignment>('policy_assign', { root, robotId, policyId, expectedId: current?.id || null, restoreId }); toast.success(restoreId ? 'Previous assignment restored locally.' : policyId ? 'Policy assigned locally. Hardware is not running.' : 'Local policy assignment removed.'); await check(); });
  }
  $effect(() => { if (!initialized && !blocked && !busy) { initialized = true; untrack(() => { void work(async () => { await refresh(); robotId = studio.policyRobotId; studio.policyRobotId = ''; if (robotId) await check(); }); }); } });
  // A completed training run can open the registration dialog once the initial load finishes.
  $effect(() => { if (!busy && !blocked && studio.policyRunId) void add(); });
  onDestroy(() => { disposed = true; studio.policyBusy = false; });
  $effect(() => { if (!playing) return; if (frameIndex >= frames.length - 1) { playing = false; return; } const timer = setTimeout(() => frameIndex++, Math.max(1, (frames[frameIndex + 1].time - frames[frameIndex].time) * 1000)); return () => clearTimeout(timer); });
</script>
<div class="flex h-full min-h-0 gap-1">
  <StudioSidebar title="Deployments">
    <DeploymentNavigation active="policies" disabled={busy || blocked} onselect={onsection} />
    <div class="flex items-center gap-2 p-3"><Input class="min-w-0 w-full" aria-label="Search policies" placeholder="Search policies" bind:value={query} /><Button size="icon-sm" aria-label="Add trained policy" title="Add trained policy" disabled={busy || blocked} onclick={add}><Plus size={15} /></Button></div>
    <div class="min-h-0 flex-1 overflow-auto">{#each policies.filter(p => `${p.name} ${p.robotName}`.toLowerCase().includes(query.toLowerCase())) as item}<button class="flex w-full items-center gap-2 px-4 py-2 text-left text-xs hover:bg-selection disabled:opacity-50 {policy?.id === item.id ? 'bg-selection text-title' : 'text-muted'}" disabled={busy || blocked} onclick={() => choose(item.id)}><SlidersHorizontal size={14} class="shrink-0" /><span class="min-w-0 flex-1"><span class="block truncate">{item.name}</span><span class="mt-1 block truncate text-[10px] text-muted">{item.robotName}</span></span></button>{:else}<p class="px-4 py-3 text-xs text-muted">{query ? 'No matching policies.' : 'Add a policy from a completed training run.'}</p>{/each}</div>
    <div class="border-t border-border p-4 text-xs leading-relaxed text-muted">Policies and assignments are saved locally. Hardware execution is unavailable.</div>
  </StudioSidebar>
  <WorkspaceColumn active={studio.activeView === 'deployments'}>
    <section class="workbench-panel flex min-h-0 flex-1 flex-col overflow-hidden" aria-label="Policies workspace">
      <header class="flex h-[35px] shrink-0 items-center gap-2 border-b border-border px-3 text-xs"><SlidersHorizontal size={14} /><span class="truncate">{policy?.name || 'Policies'}</span><Button size="icon-sm" variant="ghost" class="ml-auto" aria-label="Refresh policies" disabled={busy || blocked} onclick={() => work(async () => { await refresh(); await check(); })}><RefreshCw size={14} /></Button></header>
      {#if error}<p role="alert" class="border-b border-border px-4 py-3 text-xs text-error">{error}</p>{/if}
      {#if blocked && !busy}<p class="border-b border-border p-3 text-xs text-warning">Disconnect Live Link and save its recording before changing policy assignments.</p>{/if}
      {#if policy}
        <div class="flex h-[57px] shrink-0 items-center gap-1 border-b border-border px-4">{#each ['overview','replay','controller'] as item}<Button variant="ghost" class={tab === item ? 'bg-selection' : ''} onclick={() => { tab = item as typeof tab; playing = false; }}>{item === 'overview' ? 'Overview' : item === 'replay' ? 'Evaluation replay' : 'Controller'}</Button>{/each}<span class="ml-auto rounded border border-border px-2 py-1 text-[10px] text-muted">Saved policy</span></div>
        <div class="flex min-h-0 flex-1 overflow-auto">
          <div class="flex min-h-0 min-w-0 flex-1 flex-col">
            {#if tab === 'replay'}
              {#if frames.length}<BuilderViewport robot={policy.run.experiment.robot} selected="" onselect={() => {}} poses={frames[frameIndex]?.poses} /><div class="flex items-center gap-3 border-t border-border p-3"><Button size="icon-sm" aria-label={playing ? 'Pause evaluation' : 'Play evaluation'} onclick={() => { if (frameIndex >= frames.length - 1) frameIndex = 0; playing = !playing; }}>{#if playing}<Pause size={14} />{:else}<Play size={14} />{/if}</Button><input type="range" class="min-w-0 flex-1 accent-white" aria-label="Policy replay timeline" min={0} max={frames.length - 1} bind:value={frameIndex} oninput={() => playing = false} /><span class="text-xs text-muted">{frames[frameIndex]?.time.toFixed(2)} s</span></div>{:else}<p class="p-6 text-muted">No evaluation replay was saved with this run.</p>{/if}
            {:else if tab === 'controller'}<div class="overflow-auto p-6"><h2 class="text-sm font-medium">Joint-feedback controller</h2><p class="mb-4 mt-2 text-xs leading-relaxed text-muted">Observations: joint position and velocity. Actions: clamped position-servo targets. Parameters remain pinned to their training run.</p><div class="overflow-auto rounded border border-border"><table class="w-full text-left text-xs"><thead class="bg-surface text-muted"><tr>{#each ['Joint','Gain','Damping','Target'] as heading}<th class="p-3 font-medium">{heading}</th>{/each}</tr></thead><tbody>{#each policy.run.result?.policy.jointIds || [] as id, i}<tr class="border-t border-border"><td class="p-3">{policy.run.experiment.robot.joints.find(j => j.id === id)?.name || id}</td><td class="p-3 font-mono">{policy.run.result?.policy.gain[i].toFixed(4)}</td><td class="p-3 font-mono">{policy.run.result?.policy.damping[i].toFixed(4)}</td><td class="p-3 font-mono">{policy.run.result?.policy.target[i].toFixed(4)} {policy.run.experiment.robot.joints.find(j => j.id === id)?.type === 'prismatic' ? 'm' : 'rad'}</td></tr>{/each}</tbody></table></div><details class="mt-4 text-xs"><summary class="cursor-pointer text-muted">Policy artifact</summary><pre class="mt-3 overflow-auto">{JSON.stringify(policy.run.result?.policy, null, 2)}</pre></details></div>
            {:else}<div class="overflow-auto p-6"><p class="text-[10px] uppercase tracking-widest text-muted">Trained policy</p><h2 class="mt-2 text-xl font-medium">{policy.name}</h2><p class="mt-2 text-xs text-muted">{policy.run.experiment.robot.name} · {policy.run.result?.policy.format}</p><div class="my-6 grid grid-cols-2 gap-3"><div class="rounded border border-border p-4"><p class="text-xs text-muted">Evaluation success</p><p class="mt-2 text-2xl">{policy.run.result?.evaluation ? `${(policy.run.result.evaluation.successRate * 100).toFixed(0)}%` : '—'}</p></div><div class="rounded border border-border p-4"><p class="text-xs text-muted">Mean normalized error</p><p class="mt-2 text-2xl">{policy.run.result?.evaluation?.meanError.toFixed(4) ?? '—'}</p></div></div><p class="mb-6 text-xs leading-relaxed text-muted">Evaluation results are from simulation. Matching a saved robot definition does not verify physical calibration or hardware performance.</p><dl class="grid grid-cols-2 gap-5 text-xs">{#each [{ label: 'Source run', value: policy.run.name },{ label: 'Robot version', value: policy.versionId },{ label: 'Experience', value: policy.run.experiment.experience.name },{ label: 'Loop', value: policy.run.experiment.loop.name },{ label: 'Task', value: policy.run.experiment.task.name },{ label: 'Engine', value: policy.run.result?.engine || 'Unknown' }] as field}<div><dt class="text-muted">{field.label}</dt><dd class="mt-2 break-all">{field.value}</dd></div>{/each}</dl><Button class="mt-6" onclick={() => { studio.loopSection = 'training'; studio.activeView = 'loop'; }}>Training experiments</Button></div>{/if}
          </div>
          <aside class="w-72 shrink-0 overflow-auto border-l border-border p-4 text-xs" aria-label="Policy assignment">
            <h3 class="font-medium">Robot assignment</h3><NativeSelect class="mt-3 w-full" aria-label="Policy target robot" bind:value={robotId} disabled={busy || blocked} onchange={event => { robotId = event.currentTarget.value; compatibility = null; assignment = null; void work(check); }}><option value="">Select physical robot</option>{#each robots as robot}<option value={robot.id}>{robot.name}</option>{/each}</NativeSelect>
            {#if !robots.length}<p class="mt-3 text-muted">Add a robot connection in Robots first.</p>{/if}
            {#if compatibility}<div class="mt-4 rounded border border-border p-3"><p class="flex items-center gap-2 {compatibility.compatible ? 'text-success' : 'text-warning'}">{#if compatibility.compatible}<Check size={14} />Configuration matches{:else}Incompatible configuration{/if}</p>{#each compatibility.issues as issue}<p class="mt-2 leading-relaxed text-muted">{issue}</p>{/each}</div>{/if}
            {#if assignment}<p class="mt-4 text-muted">Current local assignment</p><p class="mt-1 break-words">{current?.policyName || 'None'}</p><Button class="mt-3 w-full" disabled={busy || blocked || !compatibility?.compatible || current?.policyId === policy.id} onclick={() => assign(policy!.id)}>{current?.policyId === policy.id ? 'Assigned locally' : 'Assign policy'}</Button>{#if current?.policyId}<Button class="mt-2 w-full" variant="ghost" disabled={busy || blocked} onclick={() => assign(null)}>Remove assignment</Button>{/if}{/if}
            <div class="mt-5 border-t border-border pt-4"><h3 class="font-medium">Hardware runtime</h3><p class="mt-2 text-muted">Unavailable · Not running</p><p class="mt-2 leading-relaxed text-muted">The current bridge reads telemetry only. Start/stop requires a motor-control adapter.</p></div>
            {#if assignment?.history.length}<div class="mt-5 border-t border-border pt-4"><h3 class="mb-2 font-medium">Assignment history</h3>{#each assignment.history.toReversed() as event}<div class="border-b border-border py-3"><div class="flex items-center gap-2"><span class="min-w-0 flex-1 break-words">{event.policyName || 'Unassigned'}</span>{#if current?.id === event.id}<Check size={14} class="shrink-0 text-muted" aria-label="Current assignment" />{:else}<Button size="icon-sm" variant="ghost" aria-label={`Restore ${event.policyName || 'unassigned'} assignment`} title="Restore assignment" disabled={busy || blocked || current?.policyId === event.policyId} onclick={() => assign(null, event.id)}><RotateCcw size={13} /></Button>{/if}</div><p class="mt-1 text-[10px] text-muted">{event.action} · {new Date(event.createdAt).toLocaleString()}</p></div>{/each}</div>{/if}
          </aside>
        </div>
      {:else}<div class="flex flex-1 flex-col items-center justify-center gap-4 p-8 text-center"><SlidersHorizontal size={32} class="text-muted" /><h2 class="text-lg">Manage your trained policies</h2><p class="max-w-md text-sm leading-relaxed text-muted">Save a completed run as a policy, inspect its evaluation, and assign it to a matching robot configuration.</p><Button disabled={busy || blocked} onclick={add}><Plus size={14} />Add trained policy</Button></div>{/if}
    </section>
  </WorkspaceColumn>
</div>
<Dialog.Root bind:open={addOpen}><Dialog.Content onInteractOutside={event => event.preventDefault()}><Dialog.Title>Add trained policy</Dialog.Title><Dialog.Description>Preserve the controller, evaluation, and robot snapshot from a completed training run.</Dialog.Description><form class="mt-4 space-y-4" onsubmit={event => { event.preventDefault(); void promote(); }}><label class="block text-xs">Training run<NativeSelect class="mt-2 w-full" aria-label="Policy training run" bind:value={runId} disabled={busy} onchange={event => { runId = event.currentTarget.value; name = runs.find(run => run.id === runId)?.name || ''; }}><option value="">Select completed run</option>{#each runs as run}<option value={run.id}>{run.name} · {run.robotName} · {new Date(run.createdAt).toLocaleString()}</option>{/each}</NativeSelect></label>{#if !runs.length}<p class="text-xs text-muted">Complete a local training run or retrieve a cloud result first.</p>{/if}<label class="block text-xs">Policy name<Input class="mt-2 w-full" bind:value={name} required maxlength={200} disabled={busy} /></label>{#if error}<p role="alert" class="text-xs text-error">{error}</p>{/if}<div class="flex justify-end gap-2"><Button type="button" disabled={busy} onclick={() => addOpen = false}>Cancel</Button><Button type="submit" disabled={busy || !runId || !name.trim()}>Save policy</Button></div></form></Dialog.Content></Dialog.Root>
