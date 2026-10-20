<script lang="ts">
  import { Button } from "$lib/components/ui/button/index.js";
  import IconButton from "$lib/components/ui/icon-button.svelte";
  import { studio, type StudioView } from "$lib/studio.svelte";
  import Layers from "@lucide/svelte/icons/layers";
  import Folder from "@lucide/svelte/icons/folder";
  import Box from "@lucide/svelte/icons/box";
  import Play from "@lucide/svelte/icons/play";
  import Plug from "@lucide/svelte/icons/plug";
  import Server from "@lucide/svelte/icons/server";
  import Blocks from "@lucide/svelte/icons/blocks";
  import Settings from "@lucide/svelte/icons/settings";
  let { busy, reviewDirty, loopDirty, settingsActive, onsettings, onview }: { busy: boolean; reviewDirty: boolean; loopDirty: boolean; settingsActive: boolean; onsettings: () => void; onview: (view: StudioView) => void } = $props();
  const items = [
    { id: "project", label: "Project", icon: Folder },
    { id: "builder", label: "Builder", icon: Box },
    { id: "simulation", label: "Simulation", icon: Play },
    { id: "experiences", label: "Experiences", icon: Layers },
    { id: "deployments", label: "Deployments", icon: Plug },
  ] as const;
</script>

<nav aria-label="Studio views" class="flex w-10 shrink-0 flex-col gap-1 pb-1 bg-surface">
  {#each items as item}
    {@const dirty = item.id === "builder" ? studio.builderDirty : item.id === "experiences" && (reviewDirty || loopDirty)}
    {@const active = studio.activeView === item.id || (item.id === "experiences" && studio.activeView === "loop")}
    <Button variant="ghost" size="icon-lg"
      class="relative flex h-10 w-10 shrink-0 items-center justify-center transition-colors {active ? 'bg-selection text-foreground' : 'text-muted hover:text-foreground'}"
      aria-label={item.label + (dirty ? " (unsaved draft)" : "")}
      aria-current={active ? "page" : undefined}
      title={item.label + (dirty ? " · Unsaved draft" : "")}
      disabled={busy}
      onclick={() => onview(item.id)}
    >
      <item.icon size={20} strokeWidth={1.5} aria-hidden="true" />
      {#if dirty}<span class="absolute right-2 top-2 h-1.5 w-1.5 rounded-full bg-accent"></span>{/if}
    </Button>
  {/each}
  <div class="mt-auto flex flex-col gap-1">
  <IconButton size="icon-lg" class={studio.activeView === "cloud" ? "bg-selection text-foreground" : "text-muted"} aria-label="Compute" title="Compute" aria-current={studio.activeView === "cloud" ? "page" : undefined} disabled={busy} onclick={() => onview("cloud")}>
    <Server size={20} strokeWidth={1.5} aria-hidden="true" />
  </IconButton>
  <IconButton size="icon-lg" class={studio.activeView === "catalog" ? "bg-selection text-foreground" : "text-muted"} aria-label="Catalog" title="Catalog" aria-current={studio.activeView === "catalog" ? "page" : undefined} disabled={busy} onclick={() => onview("catalog")}>
    <Blocks size={20} strokeWidth={1.5} aria-hidden="true" />
  </IconButton>
  <IconButton size="icon-lg" id="studio-settings-trigger" class="flex   shrink-0 items-center justify-center {settingsActive ? 'bg-selection text-foreground' : 'text-muted hover:text-foreground'}" aria-label="Settings" aria-haspopup="dialog" aria-expanded={settingsActive} aria-controls="studio-settings-dialog" title="Settings" disabled={busy} onclick={onsettings}>
    <Settings size={20} strokeWidth={1.5} aria-hidden="true" />
  </IconButton>
  </div>
</nav>
