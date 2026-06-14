<script lang="ts">
  import "../app.css";
  import { layout } from "$lib/layout.svelte";
  import TitleBar from "$lib/components/TitleBar.svelte";
  import ActivityBar from "$lib/components/ActivityBar.svelte";
  import SideBar from "$lib/components/SideBar.svelte";
  import RightPanel from "$lib/components/RightPanel.svelte";
  import BottomPanel from "$lib/components/BottomPanel.svelte";
  import StatusBar from "$lib/components/StatusBar.svelte";

  let { children } = $props();

  function onKeydown(e: KeyboardEvent) {
    if (!(e.metaKey || e.ctrlKey)) return;
    if (e.code === "KeyB" && e.altKey) {
      layout.rightPanelOpen = !layout.rightPanelOpen;
    } else if (e.code === "KeyB") {
      layout.sidebarOpen = !layout.sidebarOpen;
    } else if (e.code === "KeyJ") {
      layout.bottomPanelOpen = !layout.bottomPanelOpen;
    } else {
      return;
    }
    e.preventDefault();
  }
</script>

<svelte:window onkeydown={onKeydown} />

<div class="flex h-screen flex-col overflow-hidden">
  <TitleBar />

  <div class="flex min-h-0 flex-1">
    <ActivityBar />

    {#if layout.sidebarOpen}
      <SideBar />
    {/if}

    <div class="flex min-w-0 flex-1 flex-col">
      <main class="min-h-0 flex-1 overflow-auto">
        {@render children()}
      </main>

      {#if layout.bottomPanelOpen}
        <BottomPanel />
      {/if}
    </div>

    {#if layout.rightPanelOpen}
      <RightPanel />
    {/if}
  </div>

  <StatusBar />
</div>
