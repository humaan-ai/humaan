<script lang="ts">
  import { onMount, tick, untrack } from 'svelte';
  import { toast } from 'svelte-sonner';
  import { DropdownMenu } from 'bits-ui';
  import Trash2 from '@lucide/svelte/icons/trash-2';
  import Check from '@lucide/svelte/icons/check';
  import { invoke } from '@tauri-apps/api/core';
  import { studio } from '$lib/studio.svelte';
  import { Button } from '$lib/components/ui/button/index.js';
  import { Input } from '$lib/components/ui/input/index.js';
  import { NativeSelect } from '$lib/components/ui/native-select/index.js';
  import * as Dialog from '$lib/components/ui/dialog/index.js';
  import StudioSidebar from './StudioSidebar.svelte';
  import WorkspaceColumn from './WorkspaceColumn.svelte';
  import PartThumbnail from './PartThumbnail.svelte';
  import RobotPicker from './RobotPicker.svelte';
  import BuilderViewport from './BuilderViewport.svelte';
  import BuilderInspector from './BuilderInspector.svelte';
  import { emptyRobot, roots, attach, clone, removeLink, validate, type Robot, type SavedVersion } from '$lib/builder/model';
  import { simulationChecks } from '$lib/simulation/model';
  import { importUrdf } from '$lib/builder/urdf';
  import { importedPath, loadProjectUrdf, prepareRobotImport, type ProjectRobot, type ProjectRobots, type RobotDraft } from '$lib/builder/project';
  import Wrench from '@lucide/svelte/icons/wrench';
  import SlidersHorizontal from '@lucide/svelte/icons/sliders-horizontal';
  import Play from '@lucide/svelte/icons/play';
  import Eye from '@lucide/svelte/icons/eye';
  import Move3d from '@lucide/svelte/icons/move-3d';
  import Scan from '@lucide/svelte/icons/scan';
  import Save from '@lucide/svelte/icons/save';
  import Box from '@lucide/svelte/icons/box';
  import ChevronRight from '@lucide/svelte/icons/chevron-right';
  import Filter from '@lucide/svelte/icons/filter';
  import X from '@lucide/svelte/icons/x';
  import LayoutGrid from '@lucide/svelte/icons/layout-grid';
  import ArrowLeft from '@lucide/svelte/icons/arrow-left';
  import FilePlus from '@lucide/svelte/icons/file-plus';
  import FolderOpen from '@lucide/svelte/icons/folder-open';
  import FileUp from '@lucide/svelte/icons/file-up';
  import ChevronsDownUp from '@lucide/svelte/icons/chevrons-down-up';
  import ChevronsUpDown from '@lucide/svelte/icons/chevrons-up-down';
  import ChevronDown from '@lucide/svelte/icons/chevron-down';

  let { root }: { root: string } = $props();
  let robot = $state<Robot>(emptyRobot());
  let saved = $state(JSON.stringify(emptyRobot()));
  let selected = $state('');
  let collapsed = $state<Record<string, boolean>>({});
  $effect(() => {
    const ancestors: string[] = [];
    let id = selected;
    while (id && !ancestors.includes(id)) {
      ancestors.push(id);
      id = robot.joints.find(j => j.child === id)?.parent || '';
    }
    untrack(() => {
      for (const ancestor of ancestors.slice(1)) collapsed[ancestor] = false;
    });
  });
  let editingName = $state(false);
  let nameDraft = $state('');
  let nameInput = $state<HTMLInputElement | null>(null);
  async function editName() {
    if (busy) return;
    nameDraft = robot.name; editingName = true;
    await tick(); nameInput?.focus(); nameInput?.select();
  }
  function commitName() {
    if (!editingName) return;
    const name = nameDraft.trim();
    if (name) robot.name = name;
    editingName = false;
  }
  let allVersions = $state<SavedVersion[]>([]), modules = $state<SavedVersion[]>([]);
  let projectRobots = $state<ProjectRobot[]>([]);
  let activeRobotId = $state('');
  const versions = $derived(allVersions.filter(version => (version.robotId || 'legacy') === activeRobotId));
  const activeProjectRobot = $derived(projectRobots.find(entry => entry.id === activeRobotId));
  let robotPickerOpen = $state(false), pickedRobotId = $state('');
  const robotEntries = $derived([
    ...projectRobots.map(entry => ({ id: entry.id, name: entry.source?.split('/').at(-1) || entry.name, path: entry.source || '', missing: entry.missing })),
    ...(allVersions.some(version => !version.robotId) || (activeRobotId === 'legacy' && robot.links.length)
      ? [{ id: 'legacy', name: 'Existing project assembly', path: '', missing: false }] : []),
  ]);
  let draftError = $state('');
  let draftQueue = Promise.resolve();
  let assemblyOpen = $state(true), partsOpen = $state(false);
  let section = $state<'Assembly' | 'Parts' | 'Catalog'>('Assembly');
  function openCatalog() { section = 'Catalog'; }
  let partSearch = $state('');
  let partSource = $state('All');
  let filtersOpen = $state(false);
  let jointFilter = $state('All');
  const query = $derived(partSearch.trim().toLowerCase());
  const filteringAssembly = $derived(!!query || jointFilter !== 'All');
  const matchingLinks = $derived(robot.links.filter(link => {
    const joint = robot.joints.find(joint => joint.child === link.id);
    return (jointFilter === 'All' || (jointFilter === 'Root' ? !joint : joint?.type === jointFilter)) &&
      `${link.name} ${joint?.name || ''} ${link.sensors.map(sensor => `${sensor.name} ${sensor.type}`).join(' ')}`.toLowerCase().includes(query);
  }));
  const visibleLinks = $derived.by(() => {
    const ids = new Set(matchingLinks.map(link => link.id));
    for (const link of matchingLinks) {
      let id = link.id; const visited = new Set<string>();
      while (!visited.has(id)) {
        visited.add(id); const parent = robot.joints.find(joint => joint.child === id)?.parent;
        if (!parent) break; ids.add(parent); id = parent;
      }
    }
    return ids;
  });
  $effect(() => { if (query) { assemblyOpen = true; partsOpen = true; } });
  function clearFilters() { partSearch = ''; partSource = 'All'; jointFilter = 'All'; }

  const catalog = $derived([
    ...robot.links.map(link => ({ id: `link:${link.id}`, name: link.name, source: 'Robot parts', detail: `${link.mass} kg · ${link.sensors.length} sensors` })),
    ...modules.map(module => ({ id: module.id, name: module.name, source: 'Saved modules', detail: `${module.links} links` })),
  ]);
  const visibleParts = $derived(catalog.filter(part => (partSource === 'All' || part.source === partSource) && `${part.name} ${part.detail}`.toLowerCase().includes(query)));
  let mode = $state<'Assemble' | 'Configure' | 'Test'>('Assemble');
  let collision = $state(false);
  let explode = $state(false);
  let positions = $state<Record<string, number>>({});
  let currentVersion = $state('');
  let busy = $state(true), error = $state('');
  function notify(message: string) { toast.success(message, { id: 'builder-success' }); }
  let warnings = $state<string[]>([]);
  let library = $state(''), parentMount = $state('origin'), childMount = $state('origin');
  let moduleRobot = $state<Robot>(emptyRobot());
  let filesInput: HTMLInputElement, folderInput: HTMLInputElement;
  let importFiles = $state<File[]>([]), chosenUrdf = $state(''), importOpen = $state(false);
  let importError = $state('');
  let destinationName = $state('');
  let createOpen = $state(false), createName = $state(''), createError = $state('');
  let confirmOpen = $state(false), confirmTitle = $state(''), confirmText = $state('');
  let pending: (() => void | Promise<void>) | null = null;
  const dirty = $derived(JSON.stringify(robot) !== saved);
  const issues = $derived(validate(robot));
  const physicsChecks = $derived(simulationChecks(robot));
  const selectedIndex = $derived(robot.links.findIndex(l => l.id === selected));
  const selectedLink = $derived(robot.links[selectedIndex]);
  const moduleRoot = $derived(roots(moduleRobot)[0]);
  $effect(() => { studio.builderDirty = dirty; studio.builderBusy = busy; });
  $effect(() => { if (selectedLink && !selectedLink.mounts.some(m => m.name === parentMount)) parentMount = selectedLink.mounts[0]?.name || ''; });
  $effect(() => { if (moduleRoot && !moduleRoot.mounts.some(m => m.name === childMount)) childMount = moduleRoot.mounts[0]?.name || ''; });
  async function work(operation: () => Promise<void>) { if (busy) return; busy = true; error = ''; try { await operation(); } catch (cause) { error = String(cause); } finally { busy = false; } }
  async function refresh() {
    [allVersions, modules] = await Promise.all([invoke<SavedVersion[]>('builder_list', { root, kind: 'versions' }), invoke<SavedVersion[]>('builder_list', { root, kind: 'modules' })]);
  }
  function draft(): RobotDraft { return clone({ robot, saved, currentVersion, selected, positions, warnings }); }
  function persistDraft(id = activeRobotId, value = draft()) {
    if (!id) return Promise.resolve();
    const task = draftQueue.then(async () => {
      await invoke<void>('builder_draft_save', { root, id, draft: value });
      projectRobots = projectRobots.map(entry => entry.id === id && entry.name !== value.robot.name ? { ...entry, name: value.robot.name } : entry);
    });
    draftQueue = task.catch(() => {});
    return task;
  }
  $effect(() => {
    if (busy || !activeRobotId) return;
    const id = activeRobotId, value = draft();
    const timer = setTimeout(() => {
      void persistDraft(id, value).then(() => { draftError = ''; }).catch(cause => { draftError = `Could not save draft: ${cause}`; });
    }, 600);
    return () => clearTimeout(timer);
  });
  async function openRobot(id: string) {
    const entry = projectRobots.find(entry => entry.id === id);
    const stored = await invoke<RobotDraft | null>('builder_draft_load', { root, id });
    const latest = allVersions.find(version => (version.robotId || 'legacy') === id);
    let next: Robot, nextWarnings: string[] = [];
    if (stored) next = stored.robot;
    else if (latest) next = await invoke<Robot>('builder_load', { root, kind: 'versions', id: latest.id });
    else if (entry?.source) {
      if (entry.missing) throw new Error(`Source file is missing: ${entry.source}`);
      const result = await loadProjectUrdf(root, entry.source);
      next = result.robot; nextWarnings = result.warnings;
    } else next = emptyRobot();
    // Drafts may contain incomplete edits, which must remain recoverable.
    if (!stored && next.links.length) {
      const errors = validate(next); if (errors.length) throw new Error(errors.join('\n'));
    }
    if (id !== 'legacy') next.projectRobotId = id;
    await invoke('builder_activate_robot', { root, id });
    activeRobotId = id;
    replace(next, stored ? stored.currentVersion : latest?.id || '');
    if (stored) {
      saved = stored.saved; selected = stored.selected; positions = stored.positions; warnings = stored.warnings;
    } else warnings = nextWarnings;
  }
  async function switchRobot(id: string) {
    if (id === activeRobotId) return;
    await work(async () => { await persistDraft(); await openRobot(id); });
  }
  function browseRobots() { pickedRobotId = activeRobotId; error = ''; robotPickerOpen = true; }
  async function previewProjectRobot(id: string): Promise<Robot> {
    if (id === activeRobotId) return clone(robot);
    const stored = await invoke<RobotDraft | null>('builder_draft_load', { root, id });
    if (stored) return stored.robot;
    const latest = allVersions.find(version => (version.robotId || 'legacy') === id);
    if (latest) return invoke<Robot>('builder_load', { root, kind: 'versions', id: latest.id });
    const entry = projectRobots.find(entry => entry.id === id);
    if (entry?.source && !entry.missing) return (await loadProjectUrdf(root, entry.source)).robot;
    throw new Error('No robot preview available.');
  }
  async function openPickedRobot() {
    await switchRobot(pickedRobotId);
    if (activeRobotId === pickedRobotId) robotPickerOpen = false;
  }
  function beginCreate() {
    let name = 'new-robot', index = 2;
    while (projectRobots.some(entry => entry.source?.startsWith(`robots/${name}/`))) name = `new-robot-${index++}`;
    createName = name; createError = ''; createOpen = true;
  }
  async function createRobot() {
    if (busy) return;
    busy = true; createError = '';
    try {
      await persistDraft();
      const entry = await invoke<ProjectRobot>('builder_add_robot', { root, name: createName.trim() });
      projectRobots = [...projectRobots, entry];
      await openRobot(entry.id);
      await persistDraft();
      createOpen = false;
      notify(`Created ${entry.source}.`);
    } catch (cause) { createError = String(cause); }
    finally { busy = false; }
  }
  async function rescanRobots() {
    await work(async () => {
      await persistDraft();
      const project = await invoke<ProjectRobots>('builder_project', { root });
      projectRobots = project.robots;
      await refresh();
    });
  }
  function replace(next: Robot, id = '') {
    explode = false; collapsed = {}; assemblyOpen = true; partsOpen = false; section = 'Assembly'; library = ''; moduleRobot = emptyRobot(); editingName = false; robot = next; saved = id ? JSON.stringify(next) : JSON.stringify(emptyRobot()); currentVersion = id; selected = next.links[0]?.id || ''; positions = {}; warnings = [];
  }
  function confirm(title: string, text: string, action: () => void | Promise<void>) { confirmTitle = title; confirmText = text; pending = action; confirmOpen = true; }
  function replaceSafely(action: () => void | Promise<void>) { if (dirty) confirm('Discard Builder changes?', 'Your unsaved assembly changes will be discarded. Saved versions remain available.', action); else void action(); }
  async function readVersion(id: string) {
    await work(async () => { const next = await invoke<Robot>('builder_load', { root, kind: 'versions', id }); const errors = validate(next); if (errors.length) throw new Error(errors.join('\n')); replace(next, id); });
  }
  async function save(kind: 'versions' | 'modules') {
    await work(async () => {
      if (issues.length) throw new Error(issues.join('\n'));
      const snapshot = clone(robot);
      const version = await invoke<SavedVersion>('builder_save', { root, kind, robot: snapshot });
      if (kind === 'versions') { currentVersion = version.id; saved = JSON.stringify(snapshot); }
      await persistDraft();
      notify(kind === 'versions' ? `Saved ${version.name}.` : `Added ${version.name} to the module library.`);
      await refresh();
    });
  }
  async function loadPart(value: string): Promise<Robot> {
    const link = value.startsWith('link:') ? robot.links.find(link => link.id === value.slice(5)) : undefined; const next = link ? { ...emptyRobot(), name: link.name, links: [clone(link)], assets: clone(Object.fromEntries(Object.entries(robot.assets).filter(([key]) => [...link.visuals, ...link.collisions].some(shape => shape.kind === 'mesh' && shape.asset === key)))) } : await invoke<Robot>('builder_load', { root, kind: 'modules', id: value }); return next;
  }
  function deleteVersion(version: SavedVersion) {
    confirm('Delete saved version?', `Delete “${version.name}” saved on ${new Date(version.savedAt).toLocaleString()}? This cannot be undone. The assembly currently open will remain in your workspace.`, () => work(async () => {
      await invoke('builder_delete_version', { root, id: version.id });
      if (currentVersion === version.id) { currentVersion = ''; saved = JSON.stringify(emptyRobot()); }
      await refresh(); notify(`Deleted saved version ${version.name}.`);
    }));
  }
  async function chooseModule(value: string) {
    await work(async () => { const next = await loadPart(value); const errors = validate(next); if (errors.length) throw new Error(errors.join('\n')); moduleRobot = next; library = value; section = 'Parts'; });
  }
  function addModule() {
    error = ''; try { const next = clone(robot); selected = attach(next, moduleRobot, selected, parentMount, childMount); robot = next; section = 'Assembly'; assemblyOpen = true; } catch (cause) { error = String(cause); }
  }
  function pickFiles(files: FileList | null) {
    if (!files?.length) return;
    importError = '';
    importFiles = Array.from(files);
    const urdfs = importFiles.filter(file => /\.urdf$/i.test(file.name));
    chosenUrdf = urdfs.length === 1 ? urdfs[0].webkitRelativePath || urdfs[0].name : '';
    destinationName = (importFiles[0]?.webkitRelativePath.split('/')[0] || urdfs[0]?.name.replace(/\.urdf$/i, '') || 'robot')
      .replace(/[\\/:*?"<>|]/g, '-').replace(/^\.+|\.+$/g, '') || 'robot';
    importOpen = true;
  }
  async function runImport() {
    if (busy) return;
    busy = true;
    importError = '';
    try {
      // Validate before copying, so missing meshes don't leave partial robots.
      await importUrdf(importFiles, chosenUrdf);
      const files = await prepareRobotImport(importFiles);
      const selectedFile = importFiles.find(file => (file.webkitRelativePath || file.name) === chosenUrdf)!;
      const source = `robots/${destinationName.trim()}/${importedPath(selectedFile)}`;
      await persistDraft();
      const project = await invoke<ProjectRobots>('builder_import_robot', { root, name: destinationName.trim(), files });
      projectRobots = project.robots;
      const entry = projectRobots.find(entry => entry.source === source);
      if (!entry) throw new Error('Imported files were not found. Refresh the project robot list.');
      await openRobot(entry.id);
      await persistDraft();
      error = ''; importOpen = false;
      notify(`Imported into robots/${destinationName.trim()}/.`);
    } catch (cause) {
      importError = cause instanceof Error ? cause.message : String(cause);
    } finally {
      busy = false;
    }
  }
  onMount(() => {
    let disposed = false;
    void (async () => {
      try {
        const project = await invoke<ProjectRobots>('builder_project', { root });
        await refresh();
        if (disposed) return;
        projectRobots = project.robots;
        const restored = project.activeRobotId;
        const id = restored && (restored === 'legacy' || projectRobots.some(entry => entry.id === restored))
          ? restored : projectRobots[0]?.id || 'legacy';
        await openRobot(id);
      }
      catch (cause) { if (!disposed) error = String(cause); }
      finally { if (!disposed) busy = false; }
    })();
    return () => {
      if (activeRobotId) void persistDraft().catch(cause => toast.error(`Could not save Builder draft: ${cause}`));
      toast.dismiss('builder-success'); disposed = true; studio.builderDirty = false; studio.builderBusy = false;
    };
  });
</script>

<svelte:window onbeforeunload={(event) => { if (dirty || busy) { event.preventDefault(); event.returnValue = ''; } }} />
<input class="hidden" type="file" multiple accept=".urdf,.stl,.obj" bind:this={filesInput} onchange={() => { pickFiles(filesInput.files); filesInput.value = ''; }} />
<input class="hidden" type="file" webkitdirectory multiple bind:this={folderInput} onchange={() => { pickFiles(folderInput.files); folderInput.value = ''; }} />

{#snippet tree(id: string, depth: number, visited: string[] = [])}
  {@const link = robot.links.find(l => l.id === id)}
  {#if link && visibleLinks.has(id) && !visited.includes(id)}
    {@const children = robot.joints.filter(j => j.parent === id)}
    {@const parentJoint = robot.joints.find(j => j.child === id)}
    {@const hasChildren = children.length > 0 || link.sensors.length > 0}
    <div class="flex h-6 w-full min-w-0 items-center {selected === id ? 'bg-selection text-title' : 'text-muted'}" style={`padding-left: ${6 + Math.min(depth, 10) * 10}px`}>
      {#if hasChildren}
        <button class="flex h-6 w-4 shrink-0 items-center justify-center hover:text-title focus-visible:outline focus-visible:outline-ring" aria-label={`${collapsed[id] ? 'Expand' : 'Collapse'} ${link.name}`} aria-expanded={filteringAssembly || !collapsed[id]} disabled={filteringAssembly} onclick={() => collapsed[id] = !collapsed[id]}>
          {#if collapsed[id] && !filteringAssembly}<ChevronRight size={12} />{:else}<ChevronDown size={12} />{/if}
        </button>
      {:else}<span class="w-4 shrink-0"></span>{/if}
      <button class="flex h-full min-w-0 flex-1 items-center gap-1.5 pr-2 text-left text-xs hover:bg-hover focus-visible:outline focus-visible:outline-ring" aria-current={selected === id ? 'true' : undefined} title={`${link.name}${parentJoint ? ` · ${parentJoint.type} · ${parentJoint.name}` : ''}`} onclick={() => { selected = id; section = 'Assembly'; }} onkeydown={(event) => {
        if (event.key === 'ArrowLeft' && hasChildren) { event.preventDefault(); collapsed[id] = true; }
        if (event.key === 'ArrowRight' && hasChildren) { event.preventDefault(); collapsed[id] = false; }
      }}>
        <Box size={14} class="shrink-0" aria-hidden="true" /><span class="truncate">{link.name}</span>
      </button>
    </div>
    {#if filteringAssembly || !collapsed[id]}
      {#each link.sensors as sensor}<p class="truncate py-1 text-xs text-muted" style={`padding-left: ${32 + depth * 10}px`} title={sensor.name}>{sensor.type} · {sensor.name}</p>{/each}
      {#each children as joint}{@render tree(joint.child, depth + 1, [...visited, id])}{/each}
    {/if}
  {/if}
{/snippet}

<div class="flex h-full min-h-0 gap-1">
  <StudioSidebar title="Builder">
    <fieldset disabled={busy} class="m-0 flex min-h-0 w-full min-w-0 flex-1 flex-col border-0 p-0">
      <div class="group flex h-7 shrink-0 items-center gap-1 px-2">
        <span class="min-w-0 flex-1 truncate text-xs font-medium text-title" title={robot.name}>{robot.name}</span>
        <DropdownMenu.Root>
          <DropdownMenu.Trigger disabled={busy} aria-label="Add robot" class="flex h-6 shrink-0 items-center gap-1 rounded px-1 text-xs text-title hover:bg-hover"><FilePlus size={14} />Add robot<ChevronDown size={12} /></DropdownMenu.Trigger>
          <DropdownMenu.Portal>
            <DropdownMenu.Content align="end" sideOffset={4} class="z-50 min-w-48 rounded-md border border-border bg-surface p-1 shadow-xl">
              <DropdownMenu.Item class="flex cursor-pointer items-center gap-2 rounded px-2 py-1.5 text-xs outline-none data-highlighted:bg-hover" onSelect={beginCreate}><FilePlus size={14} />Create blank robot</DropdownMenu.Item>
              <DropdownMenu.Item class="flex cursor-pointer items-center gap-2 rounded px-2 py-1.5 text-xs outline-none data-highlighted:bg-hover" onSelect={() => filesInput.click()}><FileUp size={14} />Import URDF files…</DropdownMenu.Item>
              <DropdownMenu.Item class="flex cursor-pointer items-center gap-2 rounded px-2 py-1.5 text-xs outline-none data-highlighted:bg-hover" onSelect={() => folderInput.click()}><FolderOpen size={14} />Import folder…</DropdownMenu.Item>
            </DropdownMenu.Content>
          </DropdownMenu.Portal>
        </DropdownMenu.Root>
        <Button variant="ghost" size="icon" class="size-6" disabled={!!issues.length} aria-label="Save assembly as module" title="Save assembly as module" onclick={() => save('modules')}><Save size={14} /></Button>
      </div>
      <div class="space-y-1 border-b border-border px-2 pb-2">
        <div class="flex items-center justify-between text-[10px] text-muted">
          <span>Project robot</span>
          <button type="button" class="hover:text-title" onclick={rescanRobots}>Refresh robots</button>
        </div>
        <button type="button" aria-label="Choose project robot" aria-haspopup="dialog" disabled={busy} onclick={browseRobots}
          class="flex w-full min-w-0 items-center gap-2 rounded-md border border-border bg-raised px-2.5 py-2 text-left hover:border-border-strong focus-visible:outline-2 focus-visible:outline-ring">
          <Box size={17} class="shrink-0 text-muted" />
          <span class="min-w-0 flex-1">
            <span class="block truncate text-xs font-medium text-title">{activeProjectRobot?.source?.split('/').at(-1) || (robot.links.length ? robot.name : 'Choose a robot')}</span>
            <span class="mt-0.5 block truncate text-[10px] text-muted" title={activeProjectRobot?.source || ''}>{activeProjectRobot?.source || `${robotEntries.length} project robots · Browse previews`}</span>
          </span>
          <LayoutGrid size={14} class="shrink-0 text-muted" />
        </button>
        {#if activeProjectRobot?.missing}<p class="text-[10px] text-warning">Source missing. Your draft and saved versions remain available.</p>{/if}
        {#if draftError}<p class="text-xs text-error" role="alert">{draftError}</p>{/if}
      </div>
      <div class="min-w-0 shrink-0 px-2 py-2">
        <div class="relative min-w-0">
          <Input class={partSearch ? 'pr-16' : 'pr-9'} aria-label="Search Builder" placeholder="Search assembly & parts…" bind:value={partSearch} />
          <div class="absolute inset-y-0 right-1 flex items-center gap-0.5">
            {#if partSearch}<Button size="icon" variant="ghost" class="size-6" aria-label="Clear search" title="Clear search" onclick={() => partSearch = ''}><X size={13} /></Button>{/if}
            <DropdownMenu.Root bind:open={filtersOpen}>
              <DropdownMenu.Trigger class="flex size-6 items-center justify-center rounded hover:bg-hover focus-visible:outline focus-visible:outline-ring {partSource !== 'All' || jointFilter !== 'All' ? 'bg-selection text-title' : 'text-muted'}" aria-label="Builder filters" title="Filters"><Filter size={14} /></DropdownMenu.Trigger>
              <DropdownMenu.Portal>
                <DropdownMenu.Content align="end" sideOffset={6} class="z-50 w-56 rounded-md border border-border bg-surface p-1 shadow-xl outline-none">
                  <DropdownMenu.Group>
                    <DropdownMenu.GroupHeading class="px-2 py-1.5 text-[10px] font-medium uppercase text-muted">Parts · Source</DropdownMenu.GroupHeading>
                    <DropdownMenu.RadioGroup bind:value={partSource}>
                      {#each ['All', 'Robot parts', 'Saved modules'] as source}<DropdownMenu.RadioItem value={source} class="flex cursor-default items-center gap-2 rounded px-2 py-1.5 text-xs outline-none data-[highlighted]:bg-selection"><span class="flex w-3 items-center">{#if partSource === source}<Check size={12} />{/if}</span>{source === 'All' ? 'All sources' : source}</DropdownMenu.RadioItem>{/each}
                    </DropdownMenu.RadioGroup>
                  </DropdownMenu.Group>
                  <DropdownMenu.Separator class="my-1 h-px bg-border" />
                  <DropdownMenu.Group>
                    <DropdownMenu.GroupHeading class="px-2 py-1.5 text-[10px] font-medium uppercase text-muted">Assembly · Joint type</DropdownMenu.GroupHeading>
                    <DropdownMenu.RadioGroup bind:value={jointFilter} onValueChange={() => assemblyOpen = true}>
                      {#each ['All', 'Root', 'fixed', 'revolute', 'continuous', 'prismatic'] as type}<DropdownMenu.RadioItem value={type} class="flex cursor-default items-center gap-2 rounded px-2 py-1.5 text-xs outline-none data-[highlighted]:bg-selection"><span class="flex w-3 items-center">{#if jointFilter === type}<Check size={12} />{/if}</span>{type === 'All' ? 'All joint types' : type === 'Root' ? 'Root links' : type[0].toUpperCase() + type.slice(1)}</DropdownMenu.RadioItem>{/each}
                    </DropdownMenu.RadioGroup>
                  </DropdownMenu.Group>
                  <DropdownMenu.Separator class="my-1 h-px bg-border" />
                  <DropdownMenu.Item class="cursor-default rounded px-2 py-1.5 text-xs outline-none data-[highlighted]:bg-selection" onSelect={() => { partSource = 'All'; jointFilter = 'All'; }}>Reset filters</DropdownMenu.Item>
                </DropdownMenu.Content>
              </DropdownMenu.Portal>
            </DropdownMenu.Root>
          </div>
        </div>
      </div>
      <div class="min-h-0 min-w-0 flex-1 overflow-x-hidden overflow-y-auto">
      <details bind:open={assemblyOpen} class="group/assembly border-t border-border">
        <summary class="flex h-7 cursor-pointer list-none items-center gap-1 px-2 text-xs font-medium uppercase text-title [&::-webkit-details-marker]:hidden" onclick={() => section = 'Assembly'}><ChevronRight size={12} class="group-open/assembly:rotate-90" />Assembly<span class="ml-auto text-[10px] text-muted">{robot.links.length}</span></summary>
        <div class="flex h-7 shrink-0 items-center justify-end gap-2 px-3 text-[10px] text-muted">
          <span class="mr-auto text-[10px] uppercase tracking-wide">{matchingLinks.length}{filteringAssembly ? ` / ${robot.links.length}` : ''} parts</span><button class="rounded p-1 hover:bg-hover hover:text-title" aria-label="Expand all" title="Expand all" onclick={() => collapsed = {}}><ChevronsUpDown size={13} /></button>
          <button class="rounded p-1 hover:bg-hover hover:text-title" aria-label="Collapse all" title="Collapse all" onclick={() => collapsed = Object.fromEntries(robot.links.map(link => [link.id, true]))}><ChevronsDownUp size={13} /></button>
        </div>
        <div class="min-h-0 min-w-0 flex-1 overflow-x-hidden pb-2" role="region" aria-label="Assembly">
          {#if robot.links.length && !matchingLinks.length}<p class="px-3 py-2 text-xs text-muted">No matching assembly parts.</p>{/if}
          {#each roots(robot) as link}{@render tree(link.id, 0)}{:else}<p class="px-3 py-2 text-xs text-muted">Use Add robot to create or import a robot into this project.</p>{/each}
        </div>
      </details>
      <details bind:open={partsOpen} class="group/catalog border-t border-border">
        <summary class="flex h-7 cursor-pointer list-none items-center gap-1 px-2 text-xs font-medium uppercase text-title [&::-webkit-details-marker]:hidden" onclick={() => section = 'Parts'}><ChevronRight size={12} class="group-open/catalog:rotate-90" />Parts<span class="ml-auto text-[10px] text-muted">{catalog.length}</span></summary>
        <div class="px-2 pt-1"><Button size="sm" variant="ghost" class="w-full justify-start" onclick={openCatalog}><LayoutGrid size={14} />View all parts</Button><Button size="sm" variant="ghost" class="w-full justify-start" onclick={() => studio.activeView = 'catalog'}><FolderOpen size={14} />Browse catalog</Button></div>
        <div class="min-h-0 flex-1 overflow-auto pb-2" role="region" aria-label="Parts catalog">
          {#each ['Robot parts', 'Saved modules'] as source}
            {@const parts = visibleParts.filter(part => part.source === source)}
            {#if parts.length}<details open class="group/parts">
              <summary class="flex h-6 cursor-pointer list-none items-center gap-1 px-2 text-[10px] font-medium uppercase tracking-wide text-muted [&::-webkit-details-marker]:hidden"><ChevronRight size={12} class="group-open/parts:rotate-90" />{source}<span class="ml-auto">{parts.length}</span></summary>
              {#each parts as part}<button class="flex h-6 w-full items-center gap-1.5 pr-2 pl-6 text-left text-xs {library === part.id ? 'bg-selection text-title' : 'text-muted hover:bg-hover'}" title={`${part.name} · ${part.detail}`} aria-pressed={library === part.id} onclick={() => chooseModule(part.id)}><Box size={14} class="shrink-0" /><span class="truncate">{part.name}</span></button>{/each}
            </details>{/if}
          {/each}
          {#if !visibleParts.length}<p class="px-3 py-2 text-xs text-muted">{catalog.length ? 'No matching parts.' : 'Import a robot or save a module to populate your catalog.'}</p>{/if}
        </div>
      </details>
      <details class="group/add min-w-0 border-t border-border">
        <summary class="flex h-7 cursor-pointer list-none items-center gap-1 px-2 text-xs text-title [&::-webkit-details-marker]:hidden"><ChevronRight size={12} class="group-open/add:rotate-90" />Add to assembly</summary>
        <div class="flex min-w-0 flex-col gap-2 px-3 pb-3">
        <NativeSelect class="w-full min-w-0 max-w-full" aria-label="Module" value={library} onchange={event => chooseModule(event.currentTarget.value)}><option value="" disabled>Select a part</option>{#each robot.links as link}<option value={`link:${link.id}`}>{link.name}</option>{/each}{#each modules as module}<option value={module.id}>{module.name} · {module.links} links</option>{/each}</NativeSelect>
        {#if robot.links.length}
          <p class="truncate text-xs text-muted">Attach to: {selectedLink?.name || 'Select a link'}</p>
          <NativeSelect class="w-full min-w-0 max-w-full" aria-label="Parent mounting point" bind:value={parentMount}><option value="" disabled>Parent mounting point</option>{#each selectedLink?.mounts || [] as mount}<option value={mount.name}>{mount.name}</option>{/each}</NativeSelect>
          <NativeSelect class="w-full min-w-0 max-w-full" aria-label="Module mounting point" bind:value={childMount}><option value="" disabled>Module mounting point</option>{#each moduleRoot?.mounts || [] as mount}<option value={mount.name}>{mount.name}</option>{/each}</NativeSelect>
        {/if}
        <Button onclick={addModule} disabled={!moduleRobot.links.length || (!!robot.links.length && (!selectedLink || !parentMount || !childMount))}>Add module</Button>
        </div>
      </details>
      <details class="group/versions border-t border-border">
        <summary class="flex h-7 cursor-pointer list-none items-center gap-1 px-2 text-xs text-title [&::-webkit-details-marker]:hidden"><ChevronRight size={12} class="group-open/versions:rotate-90" />Saved versions<span class="ml-auto text-[10px] text-muted">{versions.length}</span></summary>
        <div class="pb-2" role="group" aria-label="Saved versions">
          {#each versions as version}
            <div class="group/version flex items-center {currentVersion === version.id ? 'bg-selection' : 'hover:bg-hover'}">
            <button class="flex min-w-0 flex-1 items-center gap-2 px-3 py-1.5 text-left text-xs {currentVersion === version.id ? 'bg-selection text-title' : 'text-muted hover:bg-hover'}" aria-pressed={currentVersion === version.id} title={`${version.name} · ${new Date(version.savedAt).toLocaleString()} · ${version.id}`} onclick={() => { if (currentVersion !== version.id || dirty) replaceSafely(() => readVersion(version.id)); }}>
              <span class="flex w-3 shrink-0 items-center">{#if currentVersion === version.id}<Check size={13} aria-hidden="true" />{/if}</span>
              <span class="min-w-0 flex-1"><span class="block truncate">{version.name}</span><span class="block truncate text-[10px] text-muted">{new Date(version.savedAt).toLocaleString()}</span></span>
            </button>
            <Button size="icon" variant="ghost" class="mr-1 size-6 shrink-0 opacity-0 group-hover/version:opacity-100 group-focus-within/version:opacity-100 hover:text-error" aria-label={`Delete version ${version.name}`} title="Delete saved version" onclick={() => deleteVersion(version)}><Trash2 size={13} /></Button>
            </div>
          {:else}<p class="px-3 py-2 text-xs text-muted">No saved versions yet.</p>{/each}
          {#if dirty && versions.length}<p class="px-3 pt-2 text-[10px] text-muted">Current assembly has unsaved changes.</p>{/if}
        </div>
      </details>
      </div>
    </fieldset>
  </StudioSidebar>
  <WorkspaceColumn active={studio.activeView === 'builder'}>
    <div class="workbench-panel flex min-h-0 flex-1 flex-col overflow-hidden">
      <div class="flex min-h-[35px] shrink-0 flex-wrap items-center gap-2 border-b border-border px-2 py-px">
        {#if section === 'Assembly'}
        {#if editingName}
          <Input class="h-7 w-40" aria-label="Robot name" bind:ref={nameInput} bind:value={nameDraft} onblur={commitName} onkeydown={(event) => {
            if (event.key === 'Enter') { event.preventDefault(); commitName(); }
            else if (event.key === 'Escape') { event.preventDefault(); editingName = false; }
          }} />
        {:else}
          <button class="h-7 w-40 truncate rounded px-2 text-left text-sm font-medium text-title hover:bg-selection focus-visible:outline-1 focus-visible:outline-ring" aria-label={`Rename robot ${robot.name}`} title="Double-click to rename" disabled={busy} ondblclick={editName} onkeydown={(event) => { if (event.key === 'Enter' || event.key === 'F2') { event.preventDefault(); void editName(); } }}>{robot.name}</button>
        {/if}
        <nav class="flex gap-1" aria-label="Builder modes">{#each ['Assemble', 'Configure', 'Test'] as const as tab}<Button size="sm" variant="ghost" class={mode === tab ? 'bg-selection text-title' : ''} aria-current={mode === tab ? 'page' : undefined} onclick={() => { mode = tab; if (tab === 'Test') explode = false; }}>{#if tab === 'Assemble'}<Wrench size={14} aria-hidden="true" />{:else if tab === 'Configure'}<SlidersHorizontal size={14} aria-hidden="true" />{:else}<Play size={14} aria-hidden="true" />{/if}{tab}</Button>{/each}</nav>
        <Button size="sm" variant="ghost" aria-pressed={collision} onclick={() => collision = !collision}>{#if collision}<Scan size={14} aria-hidden="true" />{:else}<Eye size={14} aria-hidden="true" />{/if}{collision ? 'Collision' : 'Visual'}</Button>
        <Button size="sm" variant="ghost" aria-pressed={explode} disabled={!robot.links.length} class={explode ? 'bg-selection text-title' : ''} onclick={() => { explode = !explode; if (explode && mode === 'Test') mode = 'Assemble'; }}><Move3d size={14} aria-hidden="true" />Explode</Button>
        <Button size="sm" class="ml-auto" variant="default" disabled={busy || !!issues.length || !dirty} onclick={() => save('versions')}><Save size={14} aria-hidden="true" />{busy ? 'Working…' : dirty ? 'Save version •' : 'Saved'}</Button>
        {:else}
          <Button size="sm" variant="ghost" onclick={() => section = 'Assembly'}><ArrowLeft size={14} />Assembly</Button>
          <span class="min-w-0 truncate text-xs font-medium text-title">{section === 'Catalog' ? 'Parts catalog' : moduleRobot.links.length ? moduleRobot.name : 'Part preview'}</span>
          {#if section === 'Parts'}<Button size="sm" variant="ghost" class="ml-auto" onclick={openCatalog}><LayoutGrid size={14} />All parts</Button>{/if}
        {/if}
      </div>
      {#if error}<p class="shrink-0 whitespace-pre-wrap border-b border-border px-3 py-2 text-xs text-error" role="alert">{error}</p>{/if}
      <div class="flex min-h-0 flex-1 overflow-x-auto">
        {#if section === 'Catalog'}
          <section class="flex min-w-0 flex-1 flex-col" aria-label="All parts catalog">
            <div class="flex flex-wrap items-center gap-3 border-b border-border p-4">
              <div class="mr-auto"><h2 class="text-sm font-medium text-title">Parts catalog</h2><p class="mt-1 text-xs text-muted">{catalog.length} parts · Select a part to view its 3D model and information.</p></div>
              <span class="text-xs text-muted">{visibleParts.length} results{query ? ` for “${partSearch.trim()}”` : ''}{partSource !== 'All' ? ` · ${partSource}` : ''}</span>
            </div>
            <div class="min-h-0 flex-1 overflow-auto p-4">
              <div class="grid grid-cols-[repeat(auto-fill,minmax(200px,1fr))] gap-3">
                {#each visibleParts as part (part.id)}<button class="group flex flex-col overflow-hidden rounded-lg border border-border bg-[#181818] text-left transition-colors hover:border-muted focus-visible:outline focus-visible:outline-ring" disabled={busy} aria-label={`Preview ${part.name}`} onclick={() => chooseModule(part.id)}>
                  <PartThumbnail name={part.name} load={() => loadPart(part.id)} />
                  <div class="w-full p-3"><span class="block truncate text-sm font-medium text-title" title={part.name}>{part.name}</span><span class="mt-1 block text-xs text-muted">{part.source}</span><span class="mt-2 block text-xs text-muted">{part.detail}</span><span class="mt-3 flex items-center gap-1 text-xs text-title">View 3D &amp; details<ChevronRight size={12} /></span></div>
                </button>{/each}
              </div>
              {#if !visibleParts.length}<div class="flex min-h-48 flex-col items-center justify-center gap-3 text-center text-sm text-muted"><Box size={32} /><p>{catalog.length ? 'No parts match your search.' : 'Import a robot to start your parts catalog.'}</p>{#if catalog.length}<Button size="sm" onclick={clearFilters}>Clear filters</Button>{:else}<Button size="sm" disabled={busy} onclick={() => folderInput.click()}><FolderOpen size={14} />Import robot folder</Button>{/if}</div>{/if}
            </div>
          </section>
        {:else if section === 'Parts'}
          {#if !moduleRobot.links.length}
            <div class="flex flex-1 items-center justify-center p-6 text-sm text-muted">Select a part from the catalog to preview it.</div>
          {:else}
          <BuilderViewport robot={moduleRobot} selected="" onselect={() => {}} />
          <aside aria-label="Part information" class="w-64 shrink-0 overflow-y-auto border-l border-border bg-surface p-4 text-xs">
            <Button size="sm" variant="ghost" class="mb-3 -ml-2" onclick={openCatalog}><ArrowLeft size={14} />Back to catalog</Button>
            <p class="mb-1 text-muted">Part catalog · 3D preview</p><h2 class="mb-4 text-sm font-medium text-title">{moduleRobot.name}</h2>
            <dl class="grid grid-cols-2 gap-2"><dt class="text-muted">Links</dt><dd>{moduleRobot.links.length}</dd><dt class="text-muted">Joints</dt><dd>{moduleRobot.joints.length}</dd><dt class="text-muted">Total mass</dt><dd>{moduleRobot.links.reduce((sum, link) => sum + link.mass, 0).toFixed(3)} kg</dd><dt class="text-muted">Mesh assets</dt><dd>{Object.keys(moduleRobot.assets).length}</dd></dl>
            {#each moduleRobot.links as link}
              <div class="mt-4 border-t border-border pt-3"><h3 class="mb-2 break-words font-medium">{link.name}</h3><p class="text-muted">Mounts: {link.mounts.map(mount => mount.name).join(', ') || 'None'}</p>
              {#each link.visuals as shape}<p class="mt-1 text-muted">{shape.kind}{#if shape.kind === 'box'} · {shape.size.join(' × ')} m{:else if shape.kind === 'sphere'} · radius {shape.size[0]} m{:else if shape.kind === 'cylinder'} · radius {shape.size[0]} m · length {shape.size[1]} m{/if}</p>{/each}
              {#each link.sensors as sensor}<p class="mt-2">{sensor.name} · {sensor.type} · {sensor.rate} Hz</p>{/each}</div>
            {/each}
            <p class="mt-4 text-muted">Preview this part, then choose a mounting point and add it to the assembly. Sensor metadata does not include a separate 3D model.</p>
          </aside>
          {/if}
        {:else}
        <BuilderViewport {robot} {selected} {collision} {explode} allowGroundPlacement={mode !== 'Test'} onplacement={position => { if (position.every(v => Math.abs(v) < 1e-8)) delete robot.placement; else robot.placement = position; }} positions={mode === 'Test' ? positions : {}} onselect={id => selected = id} />
        <aside aria-label={mode === 'Test' ? 'Assembly checks' : 'Component properties'} class="w-64 min-w-0 shrink-0 overflow-y-auto border-l border-border bg-surface">
          {#if mode === 'Test'}
            <div class="flex flex-col gap-3 p-3 text-xs"><h3 class="font-medium text-title">Assembly checks</h3><p class="text-muted">Preview joint movement here. Launch physics from a saved version in Simulation.</p>
            {#each physicsChecks as issue}<button class="text-left {issue.severity === 'error' ? 'text-error' : 'text-warning'}" onclick={() => { if (issue.linkId) { selected = issue.linkId; mode = 'Configure'; } }}>{issue.message}</button>{:else}<p class="text-success">Assembly is ready for simulation.</p>{/each}
            <Button size="sm" disabled={busy || dirty || !currentVersion} onclick={() => { studio.simulationVersion = currentVersion; studio.activeView = 'simulation'; }}>Open in Simulation</Button>
            {#if dirty || !currentVersion}<p class="text-muted">Save a version before opening Simulation.</p>{/if}
            {#each robot.joints.filter(j => j.type !== 'fixed' && !j.mimic) as joint}<label class="flex flex-col gap-2">{joint.name} · {(positions[joint.id] || 0).toFixed(2)} {joint.type === 'prismatic' ? 'm' : 'rad'}<input type="range" min={joint.type === 'continuous' ? -Math.PI : joint.lower} max={joint.type === 'continuous' ? Math.PI : joint.upper} step="0.001" value={positions[joint.id] || 0} oninput={event => positions[joint.id] = event.currentTarget.valueAsNumber} /></label>{/each}
            <Button size="sm" onclick={() => positions = {}}>Reset joint preview</Button>
            </div>
          {:else if selectedLink}
            <fieldset class="w-full min-w-0" disabled={busy}><BuilderInspector bind:robot bind:link={robot.links[selectedIndex]} onremove={() => confirm('Remove link?', 'This removes the selected link and its descendants from the current draft. Saved versions are unchanged.', () => { removeLink(robot, selected); selected = robot.links[0]?.id || ''; })} /></fieldset>
          {:else}<p class="p-4 text-xs text-muted">Select a link to configure its geometry, physical properties, mounting points, and sensors.</p>{/if}
        </aside>
        {/if}
      </div>
      {#if section === 'Assembly' && warnings.length}<details class="max-h-32 shrink-0 overflow-auto border-t border-border px-3 py-2 text-xs text-warning"><summary class="cursor-pointer">Import notes ({warnings.length})</summary>{#each warnings as warning}<p class="mt-2">{warning}</p>{/each}</details>{/if}
      {#if section === 'Assembly' && issues.length && robot.links.length && mode !== 'Test'}<button class="shrink-0 border-t border-border px-3 py-2 text-left text-xs text-warning" onclick={() => { mode = 'Test'; explode = false; }}>{issues.length} assembly issue{issues.length === 1 ? '' : 's'} · Review in Test</button>{/if}
    </div>
  </WorkspaceColumn>
</div>

<Dialog.Root bind:open={robotPickerOpen}>{#if robotPickerOpen}<Dialog.Content class="max-w-4xl">
  <Dialog.Title>Choose a robot</Dialog.Title>
  <Dialog.Description>Browse project URDFs and preview their assemblies. Your current draft is preserved when you switch.</Dialog.Description>
  <div class="mt-4"><RobotPicker entries={robotEntries} bind:value={pickedRobotId} active={activeRobotId} disabled={busy} load={previewProjectRobot} /></div>
  {#if error}<p class="mt-3 text-xs text-error" role="alert">{error}</p>{/if}
  <div class="mt-4 flex items-center justify-between gap-3 border-t border-border pt-4">
    <p class="min-w-0 truncate text-xs text-muted">{robotEntries.find(entry => entry.id === pickedRobotId)?.path || 'Select a robot to open'}</p>
    <div class="flex shrink-0 gap-2"><Button disabled={busy} onclick={() => robotPickerOpen = false}>Cancel</Button><Button variant="default" disabled={busy || !robotEntries.some(entry => entry.id === pickedRobotId)} onclick={openPickedRobot}>{busy ? 'Opening…' : 'Open robot'}</Button></div>
  </div>
</Dialog.Content>{/if}</Dialog.Root>
<Dialog.Root bind:open={createOpen}>{#if createOpen}<Dialog.Content><Dialog.Title>Create blank robot</Dialog.Title><Dialog.Description>Create a URDF with a base link inside this project's robots folder.</Dialog.Description>
  <label for="create-robot-name" class="mt-3 block text-xs">Robot folder name</label>
  <Input id="create-robot-name" class="mt-1" bind:value={createName} disabled={busy} />
  <p class="mt-2 break-all text-xs text-muted">robots/{createName.trim() || '…'}/robot.urdf</p>
  {#if createError}<p class="mt-2 whitespace-pre-wrap text-xs text-error" role="alert">{createError}</p>{/if}
  <div class="mt-4 flex justify-end gap-2"><Button disabled={busy} onclick={() => createOpen = false}>Cancel</Button><Button variant="default" disabled={busy || !createName.trim()} onclick={createRobot}>Create robot</Button></div>
</Dialog.Content>{/if}</Dialog.Root>
<Dialog.Root bind:open={importOpen}>{#if importOpen}<Dialog.Content class="max-w-4xl"><Dialog.Title>Import robot</Dialog.Title><Dialog.Description>Preview the included robots and choose one to open. All selected files will be copied into this project.</Dialog.Description>
  <label for="import-robot-name" class="mt-3 block text-xs">Robot folder name</label>
  <Input id="import-robot-name" class="mt-1" bind:value={destinationName} disabled={busy} />
  <p class="mt-2 break-all text-xs text-muted">Destination: robots/{destinationName.trim() || '…'}/</p>
  <div class="mt-4"><RobotPicker
    entries={importFiles.filter(file => /\.urdf$/i.test(file.name)).map(file => ({ id: file.webkitRelativePath || file.name, name: file.name, path: file.webkitRelativePath || file.name }))}
    bind:value={chosenUrdf} disabled={busy}
    load={async id => (await importUrdf(importFiles, id)).robot} /></div>
  <p class="mt-2 text-xs text-muted">{importFiles.length} files selected. All included URDFs will appear in Project robot.</p>
  {#if importError}<p class="mt-2 whitespace-pre-wrap text-xs text-error" role="alert">{importError}</p>{/if}
  <div class="mt-4 flex justify-end gap-2"><Button disabled={busy} onclick={() => importOpen = false}>Cancel</Button><Button variant="default" disabled={busy || !chosenUrdf || !destinationName.trim()} onclick={runImport}>Import</Button></div>
</Dialog.Content>{/if}</Dialog.Root>
<Dialog.Root bind:open={confirmOpen}>{#if confirmOpen}<Dialog.Content><Dialog.Title>{confirmTitle}</Dialog.Title><Dialog.Description>{confirmText}</Dialog.Description><div class="mt-4 flex justify-end gap-2"><Button onclick={() => { confirmOpen = false; pending = null; }}>Cancel</Button><Button variant="destructive" onclick={() => { const action = pending; pending = null; confirmOpen = false; void action?.(); }}>Continue</Button></div></Dialog.Content>{/if}</Dialog.Root>

