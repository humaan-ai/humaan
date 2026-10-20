<script lang="ts">
  import * as Dialog from "$lib/components/ui/dialog/index.js";
  import { DropdownMenu } from "bits-ui";
  import Filter from "@lucide/svelte/icons/filter";
  import Check from "@lucide/svelte/icons/check";
  import FileText from "@lucide/svelte/icons/file-text";
  import FileCode from "@lucide/svelte/icons/file-code";
  import ProjectOverview from "./ProjectOverview.svelte";
  import ProjectFilePreview from "./ProjectFilePreview.svelte";
  import ProjectCodeEditor from "./ProjectCodeEditor.svelte";
  import RefreshCw from "@lucide/svelte/icons/refresh-cw";
  import FilePlus from "@lucide/svelte/icons/file-plus";
  import FolderPlus from "@lucide/svelte/icons/folder-plus";
  import { onDestroy, tick } from "svelte";
  import ProjectFileTree from "./ProjectFileTree.svelte";
  import ChevronRight from "@lucide/svelte/icons/chevron-right";
  import X from "@lucide/svelte/icons/x";
  import { invoke } from "@tauri-apps/api/core";
  import { studio, type LocalProject } from "$lib/studio.svelte";
  import { Button } from "$lib/components/ui/button/index.js";
  import { Input } from "$lib/components/ui/input/index.js";
  import LayoutDashboard from "@lucide/svelte/icons/layout-dashboard";
  import Files from "@lucide/svelte/icons/files";
  import Boxes from "@lucide/svelte/icons/boxes";
  import Folder from "@lucide/svelte/icons/folder";
  import File from "@lucide/svelte/icons/file";
  import StudioSidebar from "./StudioSidebar.svelte";
  import WorkspaceColumn from "./WorkspaceColumn.svelte";

  let { project }: { project: LocalProject } = $props();
  type Entry = { name: string; path: string; directory: boolean; size: number; kind: string };
  const sections = [{ id: 'overview', label: 'Overview', icon: LayoutDashboard }, { id: 'files', label: 'Files', icon: Files }, { id: 'assets', label: 'Assets', icon: Boxes }] as const;
  let section = $state<'overview' | 'files' | 'assets'>('overview');
  let treeRevision = $state(0);
  let rootExpanded = $state(true);
  let creating = $state<'file' | 'folder' | null>(null);
  let newName = $state('');
  let createError = $state('');
  let createBusy = $state(false);
  let nameInput = $state<HTMLInputElement | null>(null);
  async function startCreate(kind: 'file' | 'folder') {
    creating = kind; newName = ''; createError = ''; rootExpanded = true;
    await tick(); nameInput?.focus();
  }
  async function createEntry() {
    if (!creating || createBusy || !newName.trim()) return;
    const kind = creating; const name = newName.trim();
    createBusy = true; createError = '';
    try {
      await invoke('project_create_entry', { root: project.root, name, directory: kind === 'folder' });
      creating = null; treeRevision += 1;
      if (kind === 'file') await openFile({ name, path: name, directory: false, size: 0, kind: 'File' });
    } catch (cause) { createError = String(cause); }
    finally { createBusy = false; }
  }
  let query = $state('');
  let assetType = $state('All');
  const assetTypes = [
    { value: 'All', label: 'All asset types' },
    { value: 'Robot / scene', label: 'Robots / scenes' },
    { value: 'Mesh', label: 'Meshes' },
    { value: 'Image', label: 'Images' },
    { value: 'Recording', label: 'Recordings' },
  ];
  let entries = $state<Entry[]>([]);
  let busy = $state(false);
  let error = $state('');
  let truncated = $state(false);
  let request = 0;
  const filtered = $derived(entries.filter(entry =>
    (assetType === 'All' || entry.kind === assetType) && entry.path.toLowerCase().includes(query.trim().toLowerCase())));
  async function load() {
    const id = ++request;
    if (section !== 'assets') { busy = false; return; }
    busy = true; error = ''; entries = []; truncated = false;
    try {
      const result = await invoke<{ entries: Entry[]; truncated: boolean }>('project_entries', { root: project.root, directory: '', assets: true });
      if (id === request) { entries = result.entries; truncated = result.truncated; }
    } catch (cause) { if (id === request) error = String(cause); }
    finally { if (id === request) busy = false; }
  }
  $effect(() => { section; project.root; void load(); });
  function navigate(next: typeof section) { section = next; query = ''; assetType = 'All'; }
  type Editor = { path: string; name: string; content: string; saved: string; readOnly: boolean; loading: boolean; saving: boolean; error: string; failed: boolean; crlf: boolean };
  let tabs = $state<Editor[]>([]);
  let activePath = $state('');
  let closing = $state('');
  let preview = $state(false);
  const active = $derived(tabs.find(tab => tab.path === activePath));
  const dirty = $derived(tabs.some(tab => tab.content !== tab.saved));
  const working = $derived(tabs.some(tab => tab.saving || tab.loading));
  $effect(() => { studio.editorDirty = dirty; studio.editorBusy = working || createBusy; });
  onDestroy(() => { studio.editorDirty = false; studio.editorBusy = false; });
  async function openFile(entry: Entry) {
    activePath = entry.path;
    if (tabs.some(tab => tab.path === entry.path)) return;
    tabs.push({ path: entry.path, name: entry.name, content: '', saved: '', readOnly: false, loading: true, saving: false, error: '', failed: false, crlf: false });
    const tab = tabs.find(tab => tab.path === entry.path)!;
    try {
      const result = await invoke<{ content: string; readOnly: boolean }>('project_read_file', { root: project.root, path: entry.path });
      tab.content = tab.saved = result.content; tab.readOnly = result.readOnly; tab.crlf = result.content.includes('\r\n');
    } catch (cause) { tab.error = String(cause); tab.failed = true; }
    finally { tab.loading = false; }
  }
  async function save(tab: Editor) {
    if (tab.readOnly || tab.failed || tab.loading || tab.saving || tab.content === tab.saved) return;
    const content = tab.content;
    tab.saving = true; tab.error = '';
    try {
      await invoke('project_save_file', { root: project.root, path: tab.path, content, expected: tab.saved });
      tab.saved = content;
    } catch (cause) { tab.error = String(cause); }
    finally { tab.saving = false; }
  }
  function close(path: string, discard = false) {
    const index = tabs.findIndex(tab => tab.path === path);
    const tab = tabs[index];
    if (!tab || tab.saving || tab.loading) return;
    if (!discard && tab.content !== tab.saved) { closing = path; return; }
    tabs.splice(index, 1); closing = '';
    if (activePath === path) activePath = tabs[Math.min(index, tabs.length - 1)]?.path ?? '';
  }

</script>

<svelte:window onbeforeunload={(event) => { if (dirty || working || createBusy) { event.preventDefault(); event.returnValue = ''; } }} onkeydown={(event) => { if (studio.activeView === 'project' && !(studio.chatOpen && studio.chatExpanded) && (event.metaKey || event.ctrlKey) && event.key.toLowerCase() === 's' && active && section !== 'overview') { event.preventDefault(); void save(active); } }} />

<div class="flex h-full min-h-0 gap-1">
  <StudioSidebar title="Project">
    <nav aria-label="Project sections" class="flex shrink-0 flex-col gap-1 border-b border-border px-3 pb-3">
      {#each sections as item}
        <Button variant="ghost" class="h-8 w-full justify-start {section === item.id ? 'bg-selection text-title' : ''}" aria-current={section === item.id ? 'page' : undefined} onclick={() => navigate(item.id)}>
          <item.icon size={16} strokeWidth={1.5} aria-hidden="true" />{item.label}
        </Button>
      {/each}
    </nav>
    {#if section === 'files'}
      <div class="group/explorer flex min-h-0 flex-1 flex-col py-2">
        <div class="flex h-7 shrink-0 items-center pr-2">
        <button class="flex h-7 min-w-0 flex-1 items-center gap-1 px-2 text-left text-xs font-semibold uppercase text-title hover:bg-selection" aria-expanded={rootExpanded} onclick={() => rootExpanded = !rootExpanded}>
          <ChevronRight size={14} class="shrink-0 {rootExpanded ? 'rotate-90' : ''}" aria-hidden="true" /><span class="truncate">{project.project.name}</span>
        </button>
          <div class="flex shrink-0 opacity-0 group-hover/explorer:opacity-100 group-focus-within/explorer:opacity-100">
            <button class="rounded p-1 text-muted hover:bg-selection hover:text-title" aria-label="New file" title="New File" disabled={createBusy} onclick={() => startCreate('file')}><FilePlus size={15} /></button>
            <button class="rounded p-1 text-muted hover:bg-selection hover:text-title" aria-label="New folder" title="New Folder" disabled={createBusy} onclick={() => startCreate('folder')}><FolderPlus size={15} /></button>
            <button class="rounded p-1 text-muted hover:bg-selection hover:text-title" aria-label="Refresh files" title="Refresh" onclick={() => treeRevision += 1}><RefreshCw size={15} /></button>
          </div>
        </div>
        {#if creating}
          <form class="px-3 py-1" onsubmit={(event) => { event.preventDefault(); void createEntry(); }}>
            <Input bind:ref={nameInput} bind:value={newName} class="h-6 text-xs" aria-label={creating === 'file' ? 'New file name' : 'New folder name'} placeholder={creating === 'file' ? 'File name' : 'Folder name'} disabled={createBusy} onblur={() => { if (!createBusy) { creating = null; newName = ''; createError = ''; } }} onkeydown={(event) => { if (event.key === 'Escape' && !createBusy) { creating = null; createError = ''; } }} />
            <p class="mt-1 text-xs text-muted">{createBusy ? 'Creating…' : 'Enter to create · Esc to cancel'}</p>
            {#if createError}<p role="alert" class="mt-1 text-xs text-destructive">{createError}</p>{/if}
          </form>
        {/if}
        <div class="min-h-0 flex-1 overflow-auto">
          {#if rootExpanded}{#key treeRevision}<ProjectFileTree root={project.root} {activePath} onopen={openFile} />{/key}{/if}
        </div>
      </div>
    {:else if section === 'assets'}
        <div class="flex shrink-0 flex-wrap items-center gap-2 border-b border-border p-3">
          <span class="flex-1 text-xs text-muted">Asset files across this project</span>
          <button class="rounded p-1 text-muted hover:bg-selection" aria-label="Refresh assets" title="Refresh" disabled={busy} onclick={load}><RefreshCw size={15} /></button>
          <div class="relative w-full min-w-0">
            <Input class="w-full pr-9" aria-label="Search assets" placeholder="Search assets…" bind:value={query} />
            <DropdownMenu.Root>
              <DropdownMenu.Trigger aria-label="Filter assets by type" title={assetTypes.find(type => type.value === assetType)?.label}
                class="absolute inset-y-0 right-1 my-auto flex size-6 items-center justify-center rounded hover:bg-hover focus-visible:outline focus-visible:outline-ring {assetType !== 'All' ? 'bg-selection text-title' : 'text-muted'}">
                <Filter size={14} />
                {#if assetType !== 'All'}<span class="absolute right-0.5 top-0.5 size-1 rounded-full bg-current" aria-hidden="true"></span>{/if}
              </DropdownMenu.Trigger>
              <DropdownMenu.Portal>
                <DropdownMenu.Content align="end" sideOffset={6} class="z-50 w-48 rounded-md border border-border bg-surface p-1 shadow-xl outline-none">
                  <DropdownMenu.Group>
                    <DropdownMenu.GroupHeading class="px-2 py-1.5 text-[10px] font-medium uppercase text-muted">Asset type</DropdownMenu.GroupHeading>
                    <DropdownMenu.RadioGroup bind:value={assetType}>
                      {#each assetTypes as type}
                        <DropdownMenu.RadioItem value={type.value} class="flex cursor-default items-center gap-2 rounded px-2 py-1.5 text-xs outline-none data-[highlighted]:bg-selection">
                          <span class="flex w-3 items-center">{#if assetType === type.value}<Check size={12} />{/if}</span>{type.label}
                        </DropdownMenu.RadioItem>
                      {/each}
                    </DropdownMenu.RadioGroup>
                  </DropdownMenu.Group>
                </DropdownMenu.Content>
              </DropdownMenu.Portal>
            </DropdownMenu.Root>
          </div>
        </div>
        <p class="shrink-0 px-4 pt-3 text-xs text-muted">{assetType === 'All' ? 'Robot, mesh, image, and recording files.' : `${assetTypes.find(type => type.value === assetType)?.label} · ${filtered.length} matching files`}</p>
        {#if truncated}<p role="status" class="px-4 pt-3 text-xs text-muted">Scan limited to 20,000 entries. Open a smaller folder in Files to browse further.</p>{/if}
        <div class="min-h-0 flex-1 overflow-auto p-3" aria-busy={busy}>
          {#if error}
            <p role="alert" class="p-3 text-sm text-destructive">{error}</p>
          {:else if busy}<p role="status" class="p-3 text-sm text-muted">Loading {section}…</p>
          {:else if !filtered.length}<p class="p-3 text-sm text-muted">{query || assetType !== 'All' ? 'No matching files.' : 'No asset files found. Add robot models, meshes, images, or recordings to your project folder, then refresh.'}</p>
          {:else}
            <ul class="space-y-0.5">
              {#each filtered as entry (entry.path)}
                <li><button class="flex w-full items-center gap-3 rounded-md px-3 py-2 text-left hover:bg-selection focus-visible:outline-2 focus-visible:outline-ring {activePath === entry.path ? 'bg-selection' : ''}" onclick={() => void openFile(entry)}>
                  {#if entry.directory}<Folder size={16} class="shrink-0 text-muted" />{:else}<File size={16} class="shrink-0 text-muted" />{/if}
                  <span class="min-w-0 flex-1"><span class="block truncate text-sm text-title">{entry.name}</span>{#if section === 'assets'}<span class="block truncate text-xs text-muted">{entry.path}</span>{/if}</span>
                  
                </button></li>
              {/each}
            </ul>
          {/if}
        </div>
    {/if}
  </StudioSidebar>
  <WorkspaceColumn active={studio.activeView === 'project'}>
    <section class="workbench-panel flex min-h-0 flex-1 flex-col overflow-hidden" style:background={section === 'overview' ? undefined : '#181818'} aria-label="Project workspace">
      {#if section !== 'files' || tabs.length > 0}
      <header class="flex h-[35px] shrink-0 items-center bg-[#222222] pr-2 shadow-[inset_0_-1px_0_var(--color-border)]">
        {#if section !== 'files'}<h1 class="flex h-full w-28 shrink-0 items-center border-r border-border px-4 text-sm font-medium text-title">{sections.find(item => item.id === section)?.label}</h1>{/if}
        {#if section !== 'overview' && tabs.length}
          <div class="flex h-full min-w-0 flex-1 items-stretch overflow-x-auto" aria-label="Open files">
            {#each tabs as tab, index (tab.path)}
              <div class="group/tab flex h-full shrink-0 items-center rounded-tr-lg {index === 0 ? 'rounded-tl-none border-l-0' : '-ml-px rounded-tl-lg'} border border-y-0 {activePath === tab.path ? 'relative z-10 border-border bg-[#181818] text-[#e0e0e0]' : 'border-transparent bg-[#222222] text-muted shadow-[inset_0_-1px_0_var(--color-border)] hover:bg-[#292929]'}">
                <button class="flex h-full max-w-64 min-w-0 items-center gap-2 px-3 py-2 text-xs" title={tab.path} aria-pressed={activePath === tab.path} onclick={() => activePath = tab.path}><FileCode size={14} class="shrink-0 text-[#75bad3]" aria-hidden="true" /><span class="truncate">{tab.name}</span></button>
                <div class="relative mr-2 h-5 w-5 shrink-0">
                  {#if activePath !== tab.path && tab.content !== tab.saved}
                    <span class="pointer-events-none absolute inset-0 flex items-center justify-center group-hover/tab:invisible group-focus-within/tab:invisible"><span class="h-2.5 w-2.5 rounded-full bg-white" aria-label="Unsaved changes"></span></span>
                  {/if}
                  <button class="flex h-5 w-5 items-center justify-center rounded hover:bg-background {activePath === tab.path ? '' : 'opacity-0 group-hover/tab:opacity-100 group-focus-within/tab:opacity-100'}" aria-label={`Close ${tab.name}`} title={tab.content !== tab.saved ? 'Unsaved changes · Close file' : 'Close file'} disabled={tab.saving || tab.loading} onclick={() => close(tab.path)}>
                    {#if activePath === tab.path && tab.content !== tab.saved}<span class="h-2.5 w-2.5 rounded-full bg-white" aria-label="Unsaved changes"></span>{:else}<X size={12} />{/if}
                  </button>
                </div>
              </div>
            {/each}
          </div>
        {/if}
        {#if section !== 'overview'}
          <button class="ml-auto flex h-7 shrink-0 items-center gap-1.5 rounded px-2 text-xs hover:bg-selection {preview ? 'bg-selection text-title' : 'text-muted'}" title={preview ? 'Back to editor' : 'Preview file'} aria-label="Preview mode" aria-pressed={preview} disabled={!active || active.failed || active.loading} onclick={() => preview = !preview}>{#if preview}<FileCode size={15} aria-hidden="true" />{:else}<FileText size={15} aria-hidden="true" />{/if}<span>{preview ? 'Editor' : 'Preview'}</span></button>
        {/if}
      </header>
      {/if}
      {#if section === 'overview'}
        <ProjectOverview {project} onnavigate={navigate} />
      {:else}
        {#if active}
          {#if active.error}<p role="alert" class="shrink-0 border-b border-border p-3 text-sm text-destructive">{active.error}</p>{/if}
          {#if active.loading}<p role="status" class="p-4 text-muted">Opening file…</p>
          {:else if !active.failed}
            {#if preview}<ProjectFilePreview name={active.name} content={active.content} />{:else}
            {#key active.path}
              <ProjectCodeEditor name={active.name} content={active.content} readOnly={active.readOnly} onchange={(value) => { if (active) active.content = active.crlf ? value.replace(/\r?\n/g, '\r\n') : value; }} />
            {/key}
            {/if}
            <div class="shrink-0 border-t border-border px-4 py-1 text-xs text-muted">UTF-8 · {active.crlf ? 'CRLF' : 'LF'} · {active.content.split('\n').length} lines</div>
          {/if}
        {:else}<div class="flex flex-1 items-center justify-center text-sm text-muted">Select a file from the left panel to open an editor.</div>{/if}

      {/if}
    </section>
  </WorkspaceColumn>
</div>

<Dialog.Root open={!!closing} onOpenChange={(open) => { if (!open) closing = ''; }}>
  {#if closing}
    <Dialog.Content>
      <Dialog.Title>Discard unsaved changes?</Dialog.Title>
      <Dialog.Description>Changes to {tabs.find(tab => tab.path === closing)?.name} will be lost if you close this file without saving.</Dialog.Description>
      <div class="mt-4 flex justify-end gap-2">
        <Button variant="outline" onclick={() => closing = ''}>Keep editing</Button>
        <Button variant="destructive" onclick={() => close(closing, true)}>Discard changes</Button>
      </div>
    </Dialog.Content>
  {/if}
</Dialog.Root>
