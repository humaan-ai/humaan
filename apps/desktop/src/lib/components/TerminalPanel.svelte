<script lang="ts">
  import IconButton from "$lib/components/ui/icon-button.svelte";
  import Terminal from "@lucide/svelte/icons/terminal";
  import X from "@lucide/svelte/icons/x";
  import { studio } from "$lib/studio.svelte";

  let drag: { y: number; height: number } | null = null;
  function resize(height: number) {
    studio.terminalHeight = Math.max(100, Math.min(500, height));
  }
  function close() {
    studio.terminalOpen = false;
    document.getElementById("terminal-toggle")?.focus();
  }
</script>

<!-- A focusable separator is an interactive window splitter with keyboard resizing. -->
<!-- svelte-ignore a11y_no_noninteractive_tabindex a11y_no_noninteractive_element_interactions -->
<div role="separator" aria-label="Resize terminal" aria-controls="terminal-panel" aria-orientation="horizontal" aria-valuemin={100} aria-valuemax={500} aria-valuenow={studio.terminalHeight} tabindex={0}
  class="flex h-1 shrink-0 cursor-row-resize touch-none items-center justify-center gap-0.5 focus-visible:outline focus-visible:outline-accent"
  onpointerdown={(event) => { if (event.button !== 0) return; drag = { y: event.clientY, height: studio.terminalHeight }; event.currentTarget.setPointerCapture(event.pointerId); event.preventDefault(); }}
  onpointermove={(event) => { if (drag) resize(drag.height + drag.y - event.clientY); }}
  onpointerup={() => drag = null} onpointercancel={() => drag = null} onlostpointercapture={() => drag = null}
  onkeydown={(event) => { if (["ArrowUp", "ArrowDown", "Home", "End"].includes(event.key)) { event.preventDefault(); resize(event.key === "Home" ? 100 : event.key === "End" ? 500 : studio.terminalHeight + (event.key === "ArrowUp" ? 10 : -10)); } }}>
  {#each [0, 1, 2] as dot}<span class="h-0.5 w-0.5 rounded-full bg-muted" aria-hidden="true"></span>{/each}
</div>
<section id="terminal-panel" aria-label="Terminal" class="workbench-panel flex min-h-0 shrink-0 flex-col overflow-hidden" style:height={`${studio.terminalHeight}px`} style:max-height="60%">
  <header class="flex h-[35px] shrink-0 items-center gap-2 px-3">
    <Terminal size={14} strokeWidth={1.5} aria-hidden="true" />
    <h2 class="text-xs font-medium text-title">Terminal</h2>
    <IconButton class="ml-auto" aria-label="Close terminal" title="Close terminal" onclick={close}><X size={14} aria-hidden="true" /></IconButton>
  </header>
  <div class="min-h-0 flex-1 overflow-auto px-4 py-2 font-mono text-xs text-muted">
    No terminal session connected.
  </div>
</section>
