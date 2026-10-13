<script lang="ts">
  let { label, width, min, max, edge = "right", onresize }: {
    label: string; width: number; min: number; max: number;
    edge?: "left" | "right"; onresize: (width: number) => void;
  } = $props();
  let dragging = $state(false);
  let startX = 0;
  let startWidth = 0;
  function resize(value: number) { onresize(Math.max(min, Math.min(max, value))); }
</script>

<!-- A focusable separator supports both pointer and keyboard resizing. -->
<!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
<div
  role="separator" tabindex={0} aria-label={label} aria-orientation="vertical"
  aria-valuemin={min} aria-valuemax={max} aria-valuenow={width}
  aria-valuetext={`${Math.round(width)} pixels`} title={`${label} · Drag or use arrow keys`}
  class="resize-handle {edge} {dragging ? 'dragging' : ''}"
  onpointerdown={(event) => {
    if (event.button !== 0) return;
    event.preventDefault();
    event.currentTarget.focus();
    dragging = true; startX = event.clientX; startWidth = width;
    event.currentTarget.setPointerCapture(event.pointerId);
  }}
  onpointermove={(event) => { if (dragging) resize(startWidth + (event.clientX - startX) * (edge === "right" ? 1 : -1)); }}
  onpointerup={() => dragging = false} onpointercancel={() => dragging = false} onlostpointercapture={() => dragging = false}
  onkeydown={(event) => {
    if (!["ArrowLeft", "ArrowRight", "Home", "End"].includes(event.key)) return;
    event.preventDefault();
    resize(event.key === "Home" ? min : event.key === "End" ? max : width + (event.key === "ArrowRight" ? 20 : -20) * (edge === "right" ? 1 : -1));
  }}
>
  <span class="grip" aria-hidden="true"><i></i><i></i><i></i></span>
</div>

<style>
  .resize-handle { position: absolute; top: 0; bottom: 0; z-index: 10; width: 4px; display: flex; align-items: center; justify-content: center; cursor: col-resize; touch-action: none; color: var(--color-muted); }
  .right { right: -5px; }
  .left { left: -5px; }
  .grip { display: flex; flex-direction: column; align-items: center; gap: 3px; padding: 8px 0; }
  i { display: block; width: 2px; height: 2px; border-radius: 50%; background: currentColor; }
  .resize-handle:hover, .resize-handle:focus-visible, .dragging { background: var(--color-selection); color: var(--color-title); outline: none; }
  .resize-handle:focus-visible .grip { outline: 1px solid var(--color-accent); outline-offset: 1px; border-radius: 2px; }
</style>
