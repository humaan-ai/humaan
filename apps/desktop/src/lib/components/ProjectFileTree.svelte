<script lang="ts">
  import { onDestroy } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import ChevronRight from '@lucide/svelte/icons/chevron-right';
  import Folder from '@lucide/svelte/icons/folder';
  import FolderOpen from '@lucide/svelte/icons/folder-open';
  import File from '@lucide/svelte/icons/file';
  import FileCode from '@lucide/svelte/icons/file-code';
  import FileImage from '@lucide/svelte/icons/file-image';
  import ProjectFileTree from './ProjectFileTree.svelte';
  import ProjectFileMenu from './ProjectFileMenu.svelte';
  import type { ProjectEntry as Entry } from '$lib/project-files';

  let { root, directory = '', depth = 0, activePath, onopen, ondelete, revision, deleting = false }: {
    root: string; directory?: string; depth?: number; activePath: string; onopen: (entry: Entry) => void;
    ondelete: (entry: Entry, trigger: HTMLButtonElement | null) => void; revision: number; deleting?: boolean;
  } = $props();
  let entries = $state<Entry[]>([]);
  let expanded = $state<Record<string, boolean>>({});
  let busy = $state(true);
  let error = $state('');
  let truncated = $state(false);
  let request = 0;
  async function load() {
    const id = ++request;
    error = '';
    try {
      const result = await invoke<{ entries: Entry[]; truncated: boolean }>('project_entries', { root, directory, assets: false });
      if (id === request) { entries = result.entries; truncated = result.truncated; }
    } catch (cause) { if (id === request) error = String(cause); }
    finally { if (id === request) busy = false; }
  }
  $effect(() => { root; directory; revision; void load(); });
  onDestroy(() => { request += 1; });
  function icon(entry: Entry) {
    if (entry.directory) return expanded[entry.path] ? FolderOpen : Folder;
    if (/\.(png|jpe?g|webp|svg|exr|hdr)$/i.test(entry.name)) return FileImage;
    if (/\.(urdf|xacro|xml|json|ya?ml|toml|py|rs|[cm]?[jt]sx?|svelte|css|html|sh)$/i.test(entry.name)) return FileCode;
    return File;
  }
</script>

<ul aria-label={directory || 'Project files'} class="m-0 list-none p-0">
  {#if busy}<li class="py-1 text-xs text-muted" style:padding-left="{depth * 14 + 24}px">Loading…</li>
  {:else if error}<li class="p-2 text-xs text-destructive" role="alert">{error}<button class="ml-2 underline" onclick={load}>Retry</button></li>
  {:else if !entries.length}<li class="py-1 text-xs text-muted" style:padding-left="{depth * 14 + 24}px">Empty folder</li>
  {:else}
    {#each entries as entry (entry.path)}
      {@const Icon = icon(entry)}
      <li>
        <ProjectFileMenu {entry} disabled={deleting} {ondelete}>
          <button class="flex h-[22px] w-full items-center gap-1.5 pr-3 text-left text-[13px] hover:bg-selection focus-visible:outline-1 focus-visible:outline-ring focus-visible:-outline-offset-1 {activePath === entry.path ? 'bg-selection text-title' : 'text-foreground'}" style:padding-left="{depth * 14 + 8}px" title={entry.path} aria-expanded={entry.directory ? !!expanded[entry.path] : undefined} aria-current={!entry.directory && activePath === entry.path ? 'true' : undefined}
            onclick={() => entry.directory ? expanded[entry.path] = !expanded[entry.path] : onopen(entry)}
            onkeydown={(event) => {
              if (entry.directory && (event.key === 'ArrowRight' || event.key === 'ArrowLeft')) {
                event.preventDefault(); expanded[entry.path] = event.key === 'ArrowRight';
              }
            }}>
            {#if entry.directory}<ChevronRight size={12} class="shrink-0 transition-transform {expanded[entry.path] ? 'rotate-90' : ''}" aria-hidden="true" />{:else}<span class="w-3 shrink-0"></span>{/if}
            <Icon size={15} strokeWidth={1.5} class="shrink-0 {entry.directory ? 'text-muted' : 'text-[#9cb9d1]'}" aria-hidden="true" />
            <span class="truncate">{entry.name}</span>
          </button>
        </ProjectFileMenu>
        {#if entry.directory && expanded[entry.path]}
          <ProjectFileTree {root} directory={entry.path} depth={depth + 1} {activePath} {onopen} {ondelete} {revision} {deleting} />
        {/if}
      </li>
    {/each}
  {/if}
  {#if truncated}<li class="p-2 text-xs text-muted">Folder listing limited to 20,000 entries.</li>{/if}
</ul>
