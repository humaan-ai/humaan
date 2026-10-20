<script lang="ts">
  import { tick } from 'svelte';
  import { studio } from '$lib/studio.svelte';
  import Layers from '@lucide/svelte/icons/layers';
  import FlaskConical from '@lucide/svelte/icons/flask-conical';

  let { instance, learningSetDirty = false }: { instance: 'learning-sets' | 'training'; learningSetDirty?: boolean } = $props();
  const items = [
    { id: 'learning-sets', label: 'Learning sets', icon: Layers },
    { id: 'training', label: 'Training experiments', icon: FlaskConical },
  ] as const;
  async function select(id: typeof instance) {
    studio.loopSection = id;
    await tick();
    document.getElementById(`loop-section-${id}-${id}`)?.focus();
  }
  function navigate(event: KeyboardEvent) {
    if (!['ArrowLeft', 'ArrowRight', 'Home', 'End'].includes(event.key)) return;
    event.preventDefault();
    const index = items.findIndex(item => item.id === studio.loopSection);
    const next = event.key === 'Home' ? 0 : event.key === 'End' ? items.length - 1 : (index + 1) % items.length;
    void select(items[next].id);
  }
</script>

<div class="flex h-[35px] shrink-0 items-center bg-[#222222] shadow-[inset_0_-1px_0_var(--color-border)]">
  <div role="tablist" tabindex={-1} aria-label="Loop sections" class="flex h-full min-w-0 overflow-x-auto" onkeydown={navigate}>
    {#each items as item, index}
      {@const active = studio.loopSection === item.id}
      {@const dirty = item.id === 'learning-sets' ? learningSetDirty : studio.trainingDirty}
      <div role="presentation" class="flex h-full max-w-64 shrink-0 items-center rounded-tr-lg {index === 0 ? 'rounded-tl-none border-l-0' : '-ml-px rounded-tl-lg'} border border-y-0 {active ? 'relative z-10 border-border bg-background text-title' : 'border-transparent bg-[#222222] text-muted shadow-[inset_0_-1px_0_var(--color-border)] hover:bg-[#292929]'}">
        <button role="tab" id="loop-section-{instance}-{item.id}" aria-controls="loop-workspace-{item.id}" aria-selected={active} tabindex={active ? 0 : -1} class="flex h-full min-w-0 items-center gap-2 px-3 text-xs focus-visible:outline-1 focus-visible:outline-ring focus-visible:-outline-offset-2" title={item.label} onclick={() => select(item.id)}>
          <item.icon size={14} class="shrink-0" aria-hidden="true" />
          <span class="truncate">{item.label}</span>
          {#if dirty}<span class="h-1.5 w-1.5 shrink-0 rounded-full bg-accent" aria-label="Unsaved draft"></span>{/if}
        </button>
      </div>
    {/each}
  </div>
</div>
