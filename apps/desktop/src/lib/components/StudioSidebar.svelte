<script lang="ts">
  import type { Snippet } from "svelte";
  import { studio } from "$lib/studio.svelte";
  let { title, children }: { title: string; children: Snippet } = $props();
  let dragging = $state(false);
  let startX = 0;
  let startWidth = 260;
  function resize(width: number) { studio.sidebarWidth = Math.max(200, Math.min(420, width)); }
</script>

<aside class="relative flex h-full shrink-0 flex-col border-r border-border bg-surface" style:width="{studio.sidebarWidth}px" aria-label={title}>
  <h2 class="flex h-10 shrink-0 items-center px-4 text-xs font-semibold tracking-wide text-title uppercase">{title}</h2>
  {@render children()}
  <!-- A focusable separator is an adjustable window splitter with arrow-key support. -->
  <!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
  <div
    role="separator" tabindex="0" aria-label="Resize sidebar" aria-orientation="vertical"
    aria-valuemin={200} aria-valuemax={420} aria-valuenow={studio.sidebarWidth}
    class="absolute inset-y-0 -right-0.5 z-10 w-1 cursor-col-resize touch-none hover:bg-accent/60 focus-visible:bg-accent focus-visible:outline-none {dragging ? 'bg-accent' : ''}"
    onpointerdown={(event) => { dragging = true; startX = event.clientX; startWidth = studio.sidebarWidth; event.currentTarget.setPointerCapture(event.pointerId); }}
    onpointermove={(event) => { if (dragging) resize(startWidth + event.clientX - startX); }}
    onpointerup={() => dragging = false} onlostpointercapture={() => dragging = false}
    onkeydown={(event) => {
      if (!["ArrowLeft", "ArrowRight", "Home", "End"].includes(event.key)) return;
      event.preventDefault();
      resize(event.key === "Home" ? 200 : event.key === "End" ? 420 : studio.sidebarWidth + (event.key === "ArrowRight" ? 20 : -20));
    }}
  ></div>
</aside>
