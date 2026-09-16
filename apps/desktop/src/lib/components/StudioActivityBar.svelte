<script lang="ts">
  import { studio } from "$lib/studio.svelte";
  import Layers from "@lucide/svelte/icons/layers";
  import Workflow from "@lucide/svelte/icons/workflow";
  import Settings from "@lucide/svelte/icons/settings";
  let { busy, reviewDirty, loopDirty, settingsActive, onsettings, onview }: { busy: boolean; reviewDirty: boolean; loopDirty: boolean; settingsActive: boolean; onsettings: () => void; onview: (view: "experiences" | "loop") => void } = $props();
  const items = [
    { id: "experiences", label: "Experiences", icon: Layers },
    { id: "loop", label: "Loop", icon: Workflow },
  ] as const;
</script>

<nav aria-label="Studio views" class="flex w-12 shrink-0 flex-col border-r border-border bg-surface">
  {#each items as item}
    {@const dirty = item.id === "experiences" ? reviewDirty : item.id === "loop" ? loopDirty : false}
    <button
      class="relative flex h-12 w-12 shrink-0 items-center justify-center border-l-2 transition-colors hover:bg-hover focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-accent disabled:opacity-45 {studio.activeView === item.id ? 'border-accent text-foreground' : 'border-transparent text-muted hover:text-foreground'}"
      aria-label={item.label + (dirty ? " (unsaved draft)" : "")}
      aria-current={studio.activeView === item.id ? "page" : undefined}
      title={item.label + (dirty ? " · Unsaved draft" : "")}
      disabled={busy}
      onclick={() => onview(item.id)}
    >
      <item.icon size={22} strokeWidth={1.5} aria-hidden="true" />
      {#if dirty}<span class="absolute right-2 top-2 h-1.5 w-1.5 rounded-full bg-accent"></span>{/if}
    </button>
  {/each}
  <button id="studio-settings-trigger" class="mt-auto flex h-12 w-12 shrink-0 items-center justify-center border-l-2 hover:bg-hover focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-accent disabled:opacity-45 {settingsActive ? 'border-accent text-foreground' : 'border-transparent text-muted hover:text-foreground'}" aria-label="Settings" aria-haspopup="dialog" aria-expanded={settingsActive} aria-controls="studio-settings-dialog" title="Settings" disabled={busy} onclick={onsettings}>
    <Settings size={22} strokeWidth={1.5} aria-hidden="true" />
  </button>
</nav>
