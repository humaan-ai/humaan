<script lang="ts">
  import "../app.css";
  import { layout } from "$lib/layout.svelte";
  import TitleBar from "$lib/components/TitleBar.svelte";
  import ActivityBar from "$lib/components/ActivityBar.svelte";
  import SideBar from "$lib/components/SideBar.svelte";
  import RightPanel from "$lib/components/RightPanel.svelte";
  import BottomPanel from "$lib/components/BottomPanel.svelte";
  import StatusBar from "$lib/components/StatusBar.svelte";
  import { projectState } from "$lib/project.svelte";

  let { children } = $props();

  $effect(() => {
    const file = projectState.selectedFile;
    if (file) {
      const ext = file.split(".").pop()?.toLowerCase() || "";
      const isCAD = ["gltf", "glb", "step", "obj", "urdf", "stl"].includes(ext);
      layout.isImmersive3D = isCAD;
    } else {
      layout.isImmersive3D = false;
    }
  });

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

  <div class="flex min-h-0 flex-1 relative">
    <ActivityBar />

    {#if layout.sidebarOpen}
      <SideBar />
    {/if}

    <div class="flex min-w-0 flex-1 flex-col relative">
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
