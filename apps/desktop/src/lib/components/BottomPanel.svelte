<script lang="ts">
  import { layout, type BottomTab } from "$lib/layout.svelte";
  import X from "@lucide/svelte/icons/x";

  const tabs: { id: BottomTab; label: string }[] = [
    { id: "problems", label: "Problems" },
    { id: "output", label: "Output" },
    { id: "debug", label: "Debug Console" },
    { id: "terminal", label: "Terminal" },
  ];

  let resizing = $state(false);

  function onResize(e: PointerEvent) {
    if (!resizing) return;
    // 22px = status bar below the panel
    const max = Math.round(window.innerHeight * 0.7);
    layout.bottomPanelHeight = Math.min(max, Math.max(80, window.innerHeight - 22 - e.clientY));
  }
</script>

<section
  class="relative flex shrink-0 flex-col border-t border-border bg-background"
  style="height: {layout.bottomPanelHeight}px"
>
  <div
    role="separator"
    aria-orientation="horizontal"
    class="absolute inset-x-0 -top-0.5 z-10 h-1 cursor-row-resize transition-colors {resizing
      ? 'bg-accent'
      : 'hover:bg-accent/50'}"
    onpointerdown={(e) => {
      resizing = true;
      e.currentTarget.setPointerCapture(e.pointerId);
    }}
    onpointermove={onResize}
    onpointerup={() => (resizing = false)}
  ></div>
  <div class="flex h-9 shrink-0 items-center justify-between px-3 select-none">
    <div class="flex h-full items-center gap-2">
      {#each tabs as tab}
        <button
          class="flex items-center border-b text-xs px-2 py-1 rounded-md tracking-wide font-semibold {layout.bottomTab ===
          tab.id
            ? 'bg-muted/10 border-transparent  text-white'
            : 'border-transparent text-muted hover:text-foreground'}"
          onclick={() => (layout.bottomTab = tab.id)}
        >
          {tab.label}
        </button>
      {/each}
    </div>
    <button
      class="flex h-[22px] w-[22px] items-center justify-center rounded-sm text-muted hover:bg-hover hover:text-foreground"
      title="Close Panel (⌘J)"
      onclick={() => (layout.bottomPanelOpen = false)}
    >
      <X size={14} strokeWidth={1.6} />
    </button>
  </div>

  <div class="min-h-0 flex-1 overflow-y-auto pt-2 px-4 pb-3 font-mono text-xs text-muted">
    {#if layout.bottomTab === "problems"}
      <p>No problems have been detected in the workspace.</p>
    {:else if layout.bottomTab === "output"}
      <p>[00:00:01] humaan-kernel: runtime initialized</p>
      <p>[00:00:01] desktop: webview ready on http://localhost:1420</p>
      <p>[00:00:02] agents: 0 sessions restored</p>
    {:else if layout.bottomTab === "debug"}
      <p class="mb-2">Debug session not active. Start an agent to attach the debugger.</p>
      <div class="flex items-center gap-1 text-foreground/80">
        <span class="text-link">›</span>
        <input
          type="text"
          placeholder="Evaluate expression"
          class="flex-1 bg-transparent placeholder:text-muted/50 focus:outline-none"
        />
      </div>
    {:else}
      <p>
        <span class="text-success">anonymous@humaan</span>
        <span class="text-foreground/80">~/Humaan/humaan %</span>
        <span class="cursor"></span>
      </p>
    {/if}
  </div>
</section>

<style>
  .cursor {
    display: inline-block;
    width: 7px;
    height: 14px;
    margin-left: 2px;
    vertical-align: text-bottom;
    background: currentColor;
    animation: blink 1.1s step-end infinite;
  }

  @keyframes blink {
    50% {
      opacity: 0;
    }
  }
</style>
