<script lang="ts">
  import { layout } from "$lib/layout.svelte";
  import PanelLeft from "@lucide/svelte/icons/panel-left";
  import PanelBottom from "@lucide/svelte/icons/panel-bottom";
  import PanelRight from "@lucide/svelte/icons/panel-right";
  import Search from "@lucide/svelte/icons/search";
  import { getCurrentWindow } from "@tauri-apps/api/window";

  const appWindow = getCurrentWindow();

  function onPointerDown(e: PointerEvent) {
    const target = e.target as HTMLElement;
    // Only initiate dragging on left click, and not when clicking buttons or input fields
    if (e.button === 0 && !target.closest("button") && !target.closest("input")) {
      appWindow.startDragging();
    }
  }

  const toggles = [
    {
      icon: PanelLeft,
      get active() {
        return layout.sidebarOpen;
      },
      toggle: () => (layout.sidebarOpen = !layout.sidebarOpen),
      title: "Toggle Primary Side Bar (⌘B)",
    },
    {
      icon: PanelBottom,
      get active() {
        return layout.bottomPanelOpen;
      },
      toggle: () => (layout.bottomPanelOpen = !layout.bottomPanelOpen),
      title: "Toggle Panel (⌘J)",
    },
    {
      icon: PanelRight,
      get active() {
        return layout.rightPanelOpen;
      },
      toggle: () => (layout.rightPanelOpen = !layout.rightPanelOpen),
      title: "Toggle Configuration Panel (⌥⌘B)",
    },
  ];
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<header
  onpointerdown={onPointerDown}
  class="relative flex h-[35px] shrink-0 items-center justify-end border-b border-border bg-surface pr-2 pl-20 select-none"
>
  <div class="pointer-events-none absolute inset-x-0 flex justify-center">
    <div
      class="pointer-events-auto flex h-6 w-2/5 max-w-md items-center gap-1.5 rounded-md border border-border-strong bg-raised/60 px-2 text-xs text-muted focus-within:border-accent"
    >
      <Search size={13} strokeWidth={1.8} class="shrink-0" />
      <input
        type="text"
        placeholder="Search"
        class="w-full bg-transparent placeholder:text-muted/70 focus:outline-none"
      />
    </div>
  </div>

  {#each toggles as t}
    <button
      class="flex h-[26px] w-[30px] items-center justify-center rounded-sm hover:bg-hover {t.active
        ? 'text-foreground'
        : 'text-muted'}"
      title={t.title}
      onclick={t.toggle}
    >
      <t.icon size={16} strokeWidth={1.6} />
    </button>
  {/each}
</header>
