<script lang="ts">
  import type { RecentProject } from '$lib/recent-projects';
  import FolderOpen from '@lucide/svelte/icons/folder-open';
  import X from '@lucide/svelte/icons/x';
  let { entries, busy, onopen, onremove, onclear }: { entries: RecentProject[]; busy: boolean; onopen: (root: string) => void; onremove: (root: string) => void; onclear: () => void } = $props();
</script>
<section aria-label="Recent projects" class="min-w-0">
  <div class="mb-2 flex items-center justify-between gap-2">
    <h2 class="text-xs font-semibold text-title">Recent projects</h2>
    {#if entries.length}<button class="rounded px-2 py-1 text-xs text-muted hover:bg-hover hover:text-title disabled:opacity-50" disabled={busy} onclick={onclear}>Clear recent</button>{/if}
  </div>
  {#each entries as entry (entry.root)}
    <div class="group flex min-w-0 items-center rounded-md hover:bg-hover">
      <button class="flex min-w-0 flex-1 items-center gap-3 rounded-md px-2 py-2.5 text-left focus-visible:outline focus-visible:outline-ring disabled:opacity-50" disabled={busy} title={entry.root} aria-label={`Open recent project ${entry.name}`} onclick={() => onopen(entry.root)}>
        <FolderOpen size={17} class="shrink-0 text-muted" aria-hidden="true" />
        <span class="min-w-0"><span class="block truncate text-sm text-title">{entry.name}</span><span class="mt-0.5 block truncate text-xs text-muted">{entry.root}</span></span>
      </button>
      <button class="mr-1 flex size-7 shrink-0 items-center justify-center rounded text-muted hover:bg-selection hover:text-title focus-visible:outline focus-visible:outline-ring disabled:opacity-50" disabled={busy} aria-label={`Remove ${entry.name} from recent projects`} title="Remove from recent projects" onclick={() => onremove(entry.root)}><X size={13} aria-hidden="true" /></button>
    </div>
  {:else}<p class="py-3 text-sm text-muted">Projects you open will appear here.</p>{/each}
</section>
