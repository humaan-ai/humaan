<script lang="ts">
  import { layout, type ViewId } from "$lib/layout.svelte";
  import FolderOpen from "@lucide/svelte/icons/folder-open";
  import Server from "@lucide/svelte/icons/server";
  import Layers from "@lucide/svelte/icons/layers";
  import Database from "@lucide/svelte/icons/database";
  import Boxes from "@lucide/svelte/icons/boxes";
  import Blocks from "@lucide/svelte/icons/blocks";
  import CircleUser from "@lucide/svelte/icons/circle-user";
  import Settings from "@lucide/svelte/icons/settings";

  const items: { id: ViewId; label: string; icon: typeof FolderOpen }[] = [
    { id: "project", label: "Project", icon: FolderOpen },
    { id: "assembly", label: "Assembly", icon: Blocks },
    { id: "models", label: "Models", icon: Layers },
    { id: "data", label: "Data", icon: Database },
    { id: "simulation", label: "Simulation", icon: Boxes },
    { id: "compute", label: "Compute", icon: Server },
  ];

  function activate(id: ViewId) {
    if (layout.activeView === id && layout.sidebarOpen) {
      layout.sidebarOpen = false;
    } else {
      layout.activeView = id;
      layout.sidebarOpen = true;
    }
  }
</script>

<nav class="flex w-12 shrink-0 flex-col items-center border-r border-border bg-surface select-none">
  {#each items as item}
    {@const active = layout.activeView === item.id && layout.sidebarOpen}
    <button
      class="relative flex h-12 w-12 items-center justify-center border-l-2 {active
        ? 'border-accent text-foreground'
        : 'border-transparent text-muted hover:text-foreground'}"
      title={item.label}
      onclick={() => activate(item.id)}
    >
      <item.icon size={22} strokeWidth={1.4} />
    </button>
  {/each}

  <div class="flex-1"></div>

  <button class="flex h-12 w-12 items-center justify-center text-muted hover:text-foreground" title="Account">
    <CircleUser size={22} strokeWidth={1.4} />
  </button>

  <button
    class="flex h-12 w-12 items-center justify-center {layout.rightPanelOpen
      ? 'text-foreground'
      : 'text-muted hover:text-foreground'}"
    title="Settings — Configuration (⌥⌘B)"
    onclick={() => (layout.rightPanelOpen = !layout.rightPanelOpen)}
  >
    <Settings size={22} strokeWidth={1.4} />
  </button>
</nav>
