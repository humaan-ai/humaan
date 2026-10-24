<script lang="ts">
  import PanelResizeHandle from "./PanelResizeHandle.svelte";
  import type { Snippet } from "svelte";
  import { studio } from "$lib/studio.svelte";
  let { title, children }: { title: string; children: Snippet } = $props();
</script>

<aside class="workbench-panel relative flex h-full shrink-0 flex-col bg-surface" style:width="{studio.sidebarWidth}px" aria-label={title}>
  <h2 class="flex h-[35px] shrink-0 items-center px-4 text-xs font-semibold tracking-wide text-title uppercase">{title}</h2>
  {@render children()}
  <PanelResizeHandle label="Resize sidebar" width={studio.sidebarWidth} min={200} max={420} onresize={(width) => studio.sidebarWidth = width} />
</aside>
