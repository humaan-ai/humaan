<script lang="ts">
  import { onDestroy, untrack } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { studio, type LocalProject, type StudioView } from '$lib/studio.svelte';
  import type { Robot, SavedVersion } from '$lib/builder/model';
  import { loadProjectUrdf, type ProjectRobots, type RobotDraft } from '$lib/builder/project';
  import type { RunSummary } from '$lib/simulation/model';
  import RobotArm from './icons/RobotArm.svelte';
  import PartThumbnail from './PartThumbnail.svelte';
  import { Button } from '$lib/components/ui/button/index.js';
  import ArrowUpRight from '@lucide/svelte/icons/arrow-up-right';
  import ArrowRight from '@lucide/svelte/icons/arrow-right';
  import Box from '@lucide/svelte/icons/box';
  import Play from '@lucide/svelte/icons/play';
  import Layers from '@lucide/svelte/icons/layers';
  import Radio from '@lucide/svelte/icons/radio';
  import Folder from '@lucide/svelte/icons/folder';
  import Boxes from '@lucide/svelte/icons/boxes';
  import Blocks from '@lucide/svelte/icons/blocks';
  import Server from '@lucide/svelte/icons/server';
  import RefreshCw from '@lucide/svelte/icons/refresh-cw';
  import History from '@lucide/svelte/icons/history';
  let { project, onnavigate }: { project: LocalProject; onnavigate: (section: 'files' | 'assets') => void } = $props();
  type SessionSummary = { id: string; name: string; createdAt: number };
  let versions = $state<SavedVersion[] | null>(null), runs = $state<RunSummary[] | null>(null), sessions = $state<SessionSummary[] | null>(null);
  let robots = $state<ProjectRobots | null>(null);
  let loading = $state(true), openingRobot = $state(false), errors = $state<string[]>([]), revision = $state(0);
  let request = 0;
  const latest = $derived(versions?.[0]);
  const featuredRobot = $derived(robots?.activeRobotId === 'legacy' && versions?.some(version => !version.robotId) ? undefined
    : robots?.robots.find(robot => robot.id === robots?.activeRobotId) || robots?.robots[0]);
  const featuredVersion = $derived(featuredRobot
    ? versions?.find(version => version.robotId === featuredRobot.id)
    : robots?.activeRobotId === 'legacy' ? versions?.find(version => !version.robotId) : latest);
  const featuredName = $derived(featuredRobot?.name || featuredVersion?.name);
  const activity = $derived([
    ...(versions || []).map(v => ({ key: `version-${v.id}`, name: v.name, time: v.savedAt, label: 'Robot version saved', kind: 'version' })),
    ...(runs || []).map(r => ({ key: `run-${r.id}`, name: r.name, time: r.createdAt, label: r.status === 'failed' ? 'Simulation failed' : 'Simulation completed', kind: r.status })),
    ...(sessions || []).map(s => ({ key: `session-${s.id}`, name: s.name, time: s.createdAt, label: 'Live Link session recorded', kind: 'session' }))
  ].sort((a, b) => b.time - a.time).slice(0, 5));
  const workflow = [
    { label: 'Build', view: 'builder', icon: Box, description: 'Assemble your robot and save a version.' },
    { label: 'Simulate', view: 'simulation', icon: Play, description: 'Validate motion in a physics environment.' },
    { label: 'Train', view: 'loop', icon: Layers, description: 'Run experiments and compare results.' },
    { label: 'Connect', view: 'deployments', icon: Radio, description: 'Mirror physical motion with Live Link.' }
  ] as const;
  function open(view: StudioView) { if (view === 'loop') studio.loopSection = 'training'; studio.activeView = view; }
  async function refresh() {
    const current = ++request; loading = true; errors = [];
    const results = await Promise.allSettled([
      invoke<SavedVersion[]>('builder_list', { root: project.root, kind: 'versions' }),
      invoke<RunSummary[]>('simulation_list', { root: project.root }),
      invoke<SessionSummary[]>('deployment_list', { root: project.root, kind: 'sessions' }),
      invoke<ProjectRobots>('builder_project', { root: project.root })
    ]);
    if (current !== request) return;
    const [v, r, s, p] = results;
    versions = v.status === 'fulfilled' ? v.value.toSorted((a,b) => b.savedAt - a.savedAt) : null;
    runs = r.status === 'fulfilled' ? r.value : null;
    sessions = s.status === 'fulfilled' ? s.value : null;
    robots = p.status === 'fulfilled' ? p.value : null;
    errors = results.flatMap((result, index) => result.status === 'rejected' ? [`${['Robot versions', 'Simulation runs', 'Live Link sessions', 'Project robots'][index]}: ${String(result.reason)}`] : []);
    loading = false; revision++;
  }
  $effect(() => { project.root; if (studio.activeView === 'project') untrack(() => { void refresh(); }); });
  onDestroy(() => request++);
  function date(value: number) { return new Date(value).toLocaleString(undefined, { month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit' }); }
  async function loadFeaturedRobot(): Promise<Robot> {
    const root = project.root, entry = featuredRobot, version = featuredVersion;
    if (entry) {
      const draft = await invoke<RobotDraft | null>('builder_draft_load', { root, id: entry.id });
      if (draft) return draft.robot;
    }
    if (version) return invoke<Robot>('builder_load', { root, kind: 'versions', id: version.id });
    if (entry?.source && !entry.missing) return (await loadProjectUrdf(root, entry.source)).robot;
    throw new Error('Robot source is unavailable.');
  }
  async function continueInBuilder() {
    openingRobot = true;
    try {
      if (featuredRobot && robots?.activeRobotId !== featuredRobot.id) {
        await invoke('builder_activate_robot', { root: project.root, id: featuredRobot.id });
      }
      open('builder');
    } catch (cause) { errors = [...errors, `Cannot open robot: ${String(cause)}`]; }
    finally { openingRobot = false; }
  }
</script>

<div class="overview min-h-0 flex-1 overflow-auto" aria-label="Project overview" aria-busy={loading}>
  <div class="mx-auto w-full max-w-[1160px] px-6 py-7 @min-[800px]:px-10 @min-[800px]:py-9">
    <div class="flex items-start justify-between gap-4">
      <div class="min-w-0"><div class="mb-3 flex items-center gap-2 text-[10px] font-medium uppercase tracking-[0.16em] text-muted"><span class="h-1.5 w-1.5 rounded-full bg-[#828b82]"></span>Local project</div><h2 class="break-words text-2xl font-medium tracking-tight text-title">{project.project.name}</h2><p class="mt-2 text-sm text-muted">From assembly to real-world feedback.</p></div>
      <Button variant="ghost" size="icon-sm" aria-label="Refresh project overview" title="Refresh overview" disabled={loading} onclick={refresh}><RefreshCw size={15} class={loading ? 'animate-spin' : ''} /></Button>
    </div>
    <div class="mt-7 grid grid-cols-3 divide-x divide-border border-y border-border py-4" aria-label="Project totals">
      {#each [{ label: 'Robot versions', count: versions?.length, view: 'builder' }, { label: 'Simulation runs', count: runs?.length, view: 'simulation' }, { label: 'Recorded sessions', count: sessions?.length, view: 'deployments' }] as stat}
        <button class="group px-4 text-left first:pl-0 last:pr-0 focus-visible:outline-2 focus-visible:outline-ring" onclick={() => { if (stat.view === 'deployments') studio.deploymentSection = 'sessions'; open(stat.view as StudioView); }}><span class="flex items-center gap-3 text-xl tabular-nums text-title">{loading ? '—' : stat.count ?? '—'}<ArrowUpRight size={13} class="text-muted opacity-0 transition-opacity group-hover:opacity-100 group-focus-visible:opacity-100" /></span><span class="mt-1 block text-xs text-muted">{stat.label}</span></button>
      {/each}
    </div>
    {#if errors.length}<details class="mt-4 rounded-md border border-border px-3 py-2 text-xs text-muted"><summary class="cursor-pointer">Some project information is unavailable. Refresh to retry.</summary>{#each errors as error}<p class="mt-2 break-words">{error}</p>{/each}</details>{/if}
    <div class="mt-7 grid gap-7 @min-[800px]:grid-cols-[minmax(0,1fr)_260px]">
      <div class="min-w-0">
        <section class="overflow-hidden rounded-lg border border-border bg-[#1b1b1b]" aria-label="Current robot">
          <div class="grid @min-[600px]:grid-cols-[minmax(0,1fr)_220px]">
            <div class="flex min-w-0 flex-col items-start p-5">
              <span class="text-[10px] font-medium uppercase tracking-widest text-muted">{featuredRobot ? 'Project robot' : featuredVersion ? 'Latest saved robot' : 'Start building'}</span>
              <h3 class="mt-3 break-words text-lg font-medium text-title">{loading ? 'Loading your project…' : featuredName || (versions === null ? 'Your robot workspace' : 'Give your robot a starting point')}</h3>
              {#if featuredRobot?.source}<p class="mt-2 w-full truncate font-mono text-[10px] text-muted" title={featuredRobot.source}>{featuredRobot.source}</p>{/if}
              <p class="mb-5 mt-2 text-xs leading-relaxed text-muted">{featuredRobot
                ? `${robots!.robots.length} ${robots!.robots.length === 1 ? 'robot' : 'robots'} in this project. ${featuredRobot.missing ? 'Source missing; showing a saved draft or version when available.' : 'Open Builder to edit this robot or choose another.'}`
                : featuredVersion ? `${featuredVersion.links} links · Saved ${date(featuredVersion.savedAt)}` : versions === null ? 'Open Builder to inspect your assembly and saved versions.' : 'Add a robot or place a URDF and its meshes in robots/.'}</p>
              <Button class="mt-auto" disabled={loading || openingRobot} onclick={continueInBuilder}><Box size={14} />{featuredName ? 'Continue in Builder' : 'Open Builder'}<ArrowRight size={14} /></Button>
            </div>
            <div class="flex min-h-44 items-center justify-center border-t border-border bg-background @min-[600px]:border-l @min-[600px]:border-t-0">
              {#if featuredName}{#key `${project.root}-${featuredRobot?.id || featuredVersion?.id}-${revision}`}<PartThumbnail name={featuredName} load={loadFeaturedRobot} />{/key}{:else}<div class="flex h-28 w-28 items-center justify-center rounded-full border border-[#303030] bg-[#1d1d1d]"><RobotArm size={56} strokeWidth={1} class="text-[#737373]" /></div>{/if}
            </div>
          </div>
        </section>
        <section class="mt-7" aria-label="Robot workflow">
          <h3 class="mb-3 text-xs font-medium text-title">Your workflow</h3>
          <div class="grid grid-cols-2 overflow-hidden rounded-lg border border-border">
            {#each workflow as step, i}<button class="group flex items-start gap-3 p-4 text-left transition-colors hover:bg-[#252525] focus-visible:-outline-offset-2 focus-visible:outline-2 focus-visible:outline-ring {i < 2 ? 'border-b border-border' : ''} {i % 2 === 0 ? 'border-r border-border' : ''}" onclick={() => { if (step.view === 'deployments') studio.deploymentSection = 'robots'; open(step.view); }}><step.icon size={17} strokeWidth={1.5} class="mt-0.5 shrink-0 text-muted" /><span class="min-w-0 flex-1"><span class="flex items-center justify-between gap-2 text-xs font-medium text-title">{step.label}<ArrowUpRight size={13} class="text-muted opacity-0 group-hover:opacity-100 group-focus-visible:opacity-100" /></span><span class="mt-1.5 block text-xs leading-relaxed text-muted">{step.description}</span></span></button>{/each}
          </div>
        </section>
        <section class="mt-7" aria-label="Recent project activity"><div class="mb-2 flex items-center gap-2"><History size={14} class="text-muted" /><h3 class="text-xs font-medium text-title">Recent activity</h3></div>
          {#each activity as item (item.key)}<div class="flex items-center gap-3 border-b border-border py-3"><div class="flex h-8 w-8 shrink-0 items-center justify-center rounded-md bg-[#272727] text-muted">{#if item.kind === 'version'}<Box size={14} />{:else if item.kind === 'session'}<Radio size={14} />{:else}<Play size={14} />{/if}</div><div class="min-w-0 flex-1"><p class="truncate text-xs text-title" title={item.name}>{item.name}</p><p class="mt-1 text-[11px] {item.kind === 'failed' ? 'text-warning' : 'text-muted'}">{item.label}</p></div><time class="shrink-0 text-[10px] text-muted" datetime={new Date(item.time).toISOString()}>{date(item.time)}</time></div>
          {:else}<div class="rounded-md border border-dashed border-border px-4 py-5 text-xs leading-relaxed text-muted">{loading ? 'Loading recent activity…' : errors.length ? 'Activity appears here when project information is available.' : 'Saved robot versions, simulation runs, and recorded sessions will appear here.'}</div>{/each}
        </section>
      </div>
      <aside class="min-w-0 space-y-6" aria-label="Project resources">
        <section><h3 class="mb-2 text-xs font-medium text-title">Project resources</h3>{#each [{ label: 'Files', description: 'Browse and edit source files', icon: Folder, section: 'files' }, { label: 'Assets', description: 'Models, meshes, and recordings', icon: Boxes, section: 'assets' }] as item}<button class="group -mx-2 flex w-[calc(100%+16px)] items-center gap-3 rounded-md px-2 py-3 text-left hover:bg-selection focus-visible:outline-2 focus-visible:outline-ring" onclick={() => onnavigate(item.section as 'files' | 'assets')}><item.icon size={17} strokeWidth={1.5} class="shrink-0 text-muted" /><span class="flex-1"><span class="block text-xs text-title">{item.label}</span><span class="mt-1 block text-[11px] text-muted">{item.description}</span></span><ArrowUpRight size={13} class="text-muted" /></button>{/each}</section>
        <section class="border-t border-border pt-5"><h3 class="mb-2 text-xs font-medium text-title">Extend your workspace</h3>{#each [{ label: 'Catalog', description: 'Explore robots and reusable modules', icon: Blocks, view: 'catalog' }, { label: 'Compute', description: 'Manage cloud GPU resources', icon: Server, view: 'cloud' }] as item}<button class="group -mx-2 flex w-[calc(100%+16px)] items-center gap-3 rounded-md px-2 py-3 text-left hover:bg-selection focus-visible:outline-2 focus-visible:outline-ring" onclick={() => open(item.view as StudioView)}><item.icon size={17} strokeWidth={1.5} class="shrink-0 text-muted" /><span class="flex-1"><span class="block text-xs text-title">{item.label}</span><span class="mt-1 block text-[11px] leading-relaxed text-muted">{item.description}</span></span><ArrowUpRight size={13} class="text-muted" /></button>{/each}</section>
        <section class="border-t border-border pt-5"><h3 class="text-xs font-medium text-title">Project location</h3><p class="mt-3 break-all font-mono text-[11px] leading-relaxed text-muted">{project.root}</p><details class="mt-4 text-[11px] text-muted"><summary class="cursor-pointer hover:text-title">Project details</summary><dl class="mt-3"><dt>Project ID</dt><dd class="mt-1 break-all font-mono">{project.project.id}</dd></dl></details></section>
      </aside>
    </div>
  </div>
</div>
<style>
  .overview { container-type: inline-size; }
</style>
