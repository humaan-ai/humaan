<script lang="ts">
  import { studio } from '$lib/studio.svelte';
  import Blocks from '@lucide/svelte/icons/blocks';
  import StudioSidebar from './StudioSidebar.svelte';
  import WorkspaceColumn from './WorkspaceColumn.svelte';
  let kind = $state('all');
  const kinds = [{ id: 'all', label: 'All items' }, { id: 'part', label: 'Parts' }, { id: 'component', label: 'Components' }, { id: 'module', label: 'Modules' }, { id: 'robot', label: 'Robots' }];
</script>

<div class="flex h-full min-h-0 gap-1">
  <StudioSidebar title="Catalog">
    <nav aria-label="Catalog categories" class="space-y-1 px-2 py-2">
      {#each kinds as category}
        <button class="flex h-8 w-full items-center justify-between rounded px-2 text-xs hover:bg-hover {kind === category.id ? 'bg-selection text-title' : 'text-muted'}" aria-pressed={kind === category.id} onclick={() => kind = category.id}>
          <span>{category.label}</span><span class="text-[10px] text-muted">0</span>
        </button>
      {/each}
    </nav>
    <div class="mt-auto border-t border-border p-3 text-xs text-muted">
      <p class="font-medium text-title">Humaan Cloud</p>
      <p class="mt-2 leading-relaxed">Catalog connection coming soon.</p>
    </div>
  </StudioSidebar>
  <WorkspaceColumn active={studio.activeView === 'catalog'}>
    <section class="workbench-panel relative flex min-h-0 flex-1 flex-col" aria-label="Catalog workspace">
      <header class="flex h-[35px] shrink-0 items-center gap-2 border-b border-border px-3 text-xs">
        <Blocks size={15} aria-hidden="true" /><span class="font-medium text-title">Catalog</span>
        <span class="ml-auto text-muted">0 items</span>
      </header>
      <div class="flex min-h-0 flex-1 flex-col items-center justify-center overflow-auto p-8 text-center">
        <Blocks size={38} strokeWidth={1} class="text-muted" aria-hidden="true" />
        <h1 class="mt-5 text-lg font-medium text-title">No catalog items yet</h1>
        <p class="mt-3 max-w-sm text-sm leading-relaxed text-muted">Catalog browsing and downloads will be available when Humaan Cloud is connected.</p>
      </div>
    </section>
  </WorkspaceColumn>
</div>
