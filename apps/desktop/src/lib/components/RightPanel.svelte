<script lang="ts">
  import { layout } from "$lib/layout.svelte";
  import X from "@lucide/svelte/icons/x";

  let temperature = $state(0.7);

  let resizing = $state(false);

  function onResize(e: PointerEvent) {
    if (!resizing) return;
    layout.rightPanelWidth = Math.min(520, Math.max(200, window.innerWidth - e.clientX));
  }
</script>

<aside
  class="transition-all duration-300 ease-in-out flex shrink-0 flex-col select-none {layout.isImmersive3D ? 'absolute right-0 top-[58px] bottom-0 z-20 bg-surface/45 border-l border-white/5 backdrop-blur-md shadow-2xl' : 'relative bg-surface border-l border-border'}"
  style="width: {layout.rightPanelWidth}px"
>
  <div
    role="separator"
    aria-orientation="vertical"
    class="absolute inset-y-0 -left-0.5 z-10 w-1 cursor-col-resize transition-colors {resizing
      ? 'bg-accent'
      : 'hover:bg-accent/50'}"
    onpointerdown={(e) => {
      resizing = true;
      e.currentTarget.setPointerCapture(e.pointerId);
    }}
    onpointermove={onResize}
    onpointerup={() => (resizing = false)}
  ></div>
  <div class="flex h-[35px] shrink-0 items-center justify-between pr-2 pl-5">
    <span class="text-[11px] tracking-wider text-muted uppercase">Configuration</span>
    <button
      class="flex h-[22px] w-[22px] items-center justify-center rounded-sm text-muted hover:bg-hover hover:text-foreground"
      title="Close Panel"
      onclick={() => (layout.rightPanelOpen = false)}
    >
      <X size={14} strokeWidth={1.6} />
    </button>
  </div>

  <div class="min-h-0 flex-1 space-y-5 overflow-y-auto px-5 py-3 text-[13px]">
    <label class="block space-y-1.5">
      <span class="text-foreground/90">Model</span>
      <select
        class="w-full rounded-sm border border-border-strong bg-raised px-2 py-1 focus:border-accent focus:outline-none"
      >
        <option>claude-fable-5</option>
        <option>claude-haiku-4-5</option>
        <option>llama-3.3-70b</option>
      </select>
    </label>

    <label class="block space-y-1.5">
      <span class="flex justify-between text-foreground/90">
        Temperature <span class="text-muted">{temperature.toFixed(1)}</span>
      </span>
      <input type="range" min="0" max="1" step="0.1" bind:value={temperature} class="w-full accent-accent" />
    </label>

    <label class="block space-y-1.5">
      <span class="text-foreground/90">Max output tokens</span>
      <input
        type="number"
        value="4096"
        class="w-full rounded-sm border border-border-strong bg-raised px-2 py-1 focus:border-accent focus:outline-none"
      />
    </label>

    <label class="block space-y-1.5">
      <span class="text-foreground/90">System prompt</span>
      <textarea
        rows="4"
        placeholder="You are a helpful agent…"
        class="w-full resize-none rounded-sm border border-border-strong bg-raised px-2 py-1 placeholder:text-muted/60 focus:border-accent focus:outline-none"
      ></textarea>
    </label>

    <label class="flex items-center gap-2">
      <input type="checkbox" checked class="accent-accent" />
      <span class="text-foreground/90">Stream responses</span>
    </label>
  </div>
</aside>
