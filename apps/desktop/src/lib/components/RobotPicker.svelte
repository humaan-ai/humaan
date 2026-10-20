<script lang="ts">
  import Search from '@lucide/svelte/icons/search';
  import LayoutGrid from '@lucide/svelte/icons/layout-grid';
  import List from '@lucide/svelte/icons/list';
  import Check from '@lucide/svelte/icons/check';
  import Box from '@lucide/svelte/icons/box';
  import { Input } from '$lib/components/ui/input/index.js';
  import { Button } from '$lib/components/ui/button/index.js';
  import PartThumbnail from './PartThumbnail.svelte';
  import type { Robot } from '$lib/builder/model';

  type Entry = { id: string; name: string; path: string; missing?: boolean };
  let { entries, value = $bindable(''), active = '', disabled = false, load }: {
    entries: Entry[];
    value?: string;
    active?: string;
    disabled?: boolean;
    load: (id: string) => Promise<Robot>;
  } = $props();
  let query = $state(''), view = $state<'grid' | 'list'>('grid');
  let details = $state<Record<string, { links: number; joints: number }>>({});
  const visible = $derived(entries.filter(entry => `${entry.name} ${entry.path}`.toLowerCase().includes(query.trim().toLowerCase())));
  const previews = new Map<string, Promise<Robot>>();
  function preview(id: string) {
    let result = previews.get(id);
    if (!result) {
      result = load(id).then(robot => {
        details[id] = { links: robot.links.length, joints: robot.joints.length };
        return robot;
      });
      previews.set(id, result);
    }
    return result;
  }
</script>

<div class="min-w-0 space-y-3">
  <div class="flex items-center gap-2">
    <div class="relative min-w-0 flex-1">
      <Search size={14} class="pointer-events-none absolute left-3 top-1/2 -translate-y-1/2 text-muted" />
      <Input class="pl-9" aria-label="Search robots" placeholder="Search robots or file paths…" bind:value={query} />
    </div>
    <div class="flex shrink-0 gap-0.5 rounded-md border border-border p-0.5" role="group" aria-label="Robot view">
      <Button size="icon" variant="ghost" class="size-7 {view === 'grid' ? 'bg-selection text-title' : 'text-muted'}" aria-label="Grid view" aria-pressed={view === 'grid'} onclick={() => view = 'grid'}><LayoutGrid size={15} /></Button>
      <Button size="icon" variant="ghost" class="size-7 {view === 'list' ? 'bg-selection text-title' : 'text-muted'}" aria-label="List view" aria-pressed={view === 'list'} onclick={() => view = 'list'}><List size={15} /></Button>
    </div>
  </div>
  <div class="flex items-center justify-between text-xs text-muted">
    <span aria-live="polite">{visible.length} {visible.length === 1 ? 'robot' : 'robots'}{query ? ` matching “${query}”` : ''}</span>
    <span>Select a robot to continue</span>
  </div>
  <div class="max-h-[48dvh] min-h-44 overflow-y-auto p-1" role="group" aria-label="Robot files">
    <div class={view === 'grid' ? 'grid grid-cols-1 gap-3 min-[480px]:grid-cols-2 min-[760px]:grid-cols-3' : 'flex flex-col gap-2'}>
      {#each visible as entry (entry.id)}
        <button type="button" {disabled} aria-label={`Select ${entry.path || entry.name}`} aria-pressed={value === entry.id} data-robot-id={entry.id}
          class="group relative min-w-0 overflow-hidden rounded-lg border text-left transition-colors hover:border-border-strong focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ring disabled:opacity-50 {value === entry.id ? 'border-ring bg-selection' : 'border-border bg-surface'} {view === 'list' ? 'flex items-center gap-3 pr-3' : ''}"
          onclick={() => value = entry.id}>
          <div class={view === 'list' ? 'w-24 shrink-0 overflow-hidden border-r border-border' : 'overflow-hidden border-b border-border'}>
            <PartThumbnail name={entry.name} compact={view === 'list'} load={() => preview(entry.id)} />
          </div>
          <div class="min-w-0 flex-1 {view === 'grid' ? 'p-3' : 'py-2'}">
            <div class="flex items-center gap-2">
              <span class="truncate text-xs font-medium text-title">{entry.name}</span>
              {#if active === entry.id}<span class="shrink-0 rounded bg-raised px-1.5 py-0.5 text-[9px] text-muted">Current</span>{/if}
            </div>
            <p class="mt-1 truncate font-mono text-[10px] text-muted" title={entry.path}>{entry.path || 'Saved assembly'}</p>
            <p class="mt-2 text-[10px] {entry.missing ? 'text-warning' : 'text-muted'}">
              {#if entry.missing}Source missing · saved draft available if previously opened
              {:else if details[entry.id]}{details[entry.id].links} links · {details[entry.id].joints} joints
              {:else}URDF robot{/if}
            </p>
          </div>
          <span class="flex size-5 shrink-0 items-center justify-center rounded-full border {value === entry.id ? 'border-ring bg-selection text-title' : 'border-border-strong bg-surface text-muted'} {view === 'grid' ? 'absolute right-2 top-2' : ''}">
            {#if value === entry.id}<Check size={12} />{/if}
          </span>
        </button>
      {:else}
        <div class="col-span-full flex min-h-44 flex-col items-center justify-center gap-3 px-4 text-center text-sm text-muted">
          <Box size={28} />
          <p>{entries.length ? 'No robots match your search.' : 'No robots in this project yet.'}</p>
          {#if query}<Button size="sm" variant="ghost" onclick={() => query = ''}>Clear search</Button>{:else}<p class="text-xs">Add a robot or place URDF files in robots/.</p>{/if}
        </div>
      {/each}
    </div>
  </div>
</div>
