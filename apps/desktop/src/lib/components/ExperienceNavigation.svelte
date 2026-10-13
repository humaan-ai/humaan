<script lang="ts">
  import { Button } from "$lib/components/ui/button/index.js";
  import { studio } from "$lib/studio.svelte";
  import Layers from "@lucide/svelte/icons/layers";
  import Workflow from "@lucide/svelte/icons/workflow";

  let { busy, reviewDirty, loopDirty }: { busy: boolean; reviewDirty: boolean; loopDirty: boolean } = $props();
</script>

<nav aria-label="Experiences sections" class="flex shrink-0 flex-col gap-1 border-b border-border px-3 pb-3">
  {#each [{ id: "experiences", label: "Experiences", icon: Layers, dirty: reviewDirty }, { id: "loop", label: "Loops", icon: Workflow, dirty: loopDirty }] as const as item}
    <Button variant="ghost" class="h-8 w-full min-w-0 justify-start {studio.activeView === item.id ? 'bg-selection text-title' : ''}" title={item.label} aria-current={studio.activeView === item.id ? "page" : undefined} disabled={busy} onclick={() => studio.activeView = item.id}>
      <item.icon size={16} strokeWidth={1.5} class="shrink-0" aria-hidden="true" />
      <span class="min-w-0 truncate">{item.label}</span>
      {#if item.dirty}<span class="ml-auto h-1.5 w-1.5 shrink-0 rounded-full bg-accent" aria-label="Unsaved draft"></span>{/if}
    </Button>
  {/each}
</nav>
