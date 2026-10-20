<script lang="ts">
  import IconButton from "$lib/components/ui/icon-button.svelte";
  import { studio } from "$lib/studio.svelte";
  import { onMount } from "svelte";
  import { invoke, isTauri } from "@tauri-apps/api/core";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { toast } from "svelte-sonner";

  let customTitlebar = $state(false);
  let showIcon = $state(false);
  let maximized = $state(false);
  let menuLabels = $state<string[]>([]);

  async function windowAction(action: () => Promise<unknown>) {
    try { await action(); }
    catch (error) { toast.error(`Window action failed: ${String(error)}`); }
  }

  onMount(() => {
    if (!isTauri()) return;
    let disposed = false;
    let unlisten: (() => void) | undefined;
    void windowAction(async () => {
      const [custom, icon] = await Promise.all([
        invoke<boolean>("uses_custom_titlebar"),
        invoke<boolean>("show_titlebar_icon"),
      ]);
      if (disposed) return;
      showIcon = icon;
      if (!custom) return;
      const labels = await invoke<string[]>("app_menu_labels");
      if (disposed) return;
      menuLabels = labels;
      customTitlebar = true;
      const window = getCurrentWindow();
      const updateMaximized = async () => {
        const value = await window.isMaximized();
        if (!disposed) maximized = value;
      };
      unlisten = await window.onResized(() => { void windowAction(updateMaximized); });
      if (disposed) { unlisten(); return; }
      await updateMaximized();
    });
    return () => { disposed = true; unlisten?.(); };
  });
</script>

<header data-tauri-drag-region class="relative flex h-10 shrink-0 items-center gap-3 bg-surface select-none {showIcon ? 'pl-3' : 'pl-20'} {customTitlebar ? 'pr-56' : 'pr-20'}">
  {#if showIcon}
    <img src="/favicon.svg" alt="Humaan Studio" class="pointer-events-none size-5 shrink-0 object-contain" />
  {/if}
  {#if customTitlebar}
    <nav aria-label="Application menu" class="flex shrink-0 items-center">
      {#each menuLabels as label}
        <button type="button" class="rounded px-2 py-1 text-xs text-muted hover:bg-white/10 hover:text-title focus-visible:outline focus-visible:outline-ring"
          aria-haspopup="menu"
          onpointerdown={(event) => event.preventDefault()}
          onclick={(event) => {
            const x = event.currentTarget.getBoundingClientRect().left;
            void windowAction(() => invoke("show_app_menu", { label, x }));
          }}>{label}</button>
      {/each}
    </nav>
  {/if}
  <span data-tauri-drag-region class="min-w-0 flex-1 truncate text-center text-xs text-muted">{studio.project?.project.name ?? "Humaan Studio"}</span>
  <div class="absolute top-0 flex h-10 w-20 items-center {customTitlebar ? 'right-36' : 'right-0'}">
  <IconButton id="terminal-toggle" class="absolute right-11 {studio.terminalOpen ? 'text-title' : 'text-muted'}"
    aria-label={studio.terminalOpen ? "Hide bottom panel" : "Show bottom panel"} aria-expanded={studio.terminalOpen} aria-controls="terminal-panel" title={studio.terminalOpen ? "Hide bottom panel" : "Show bottom panel"}
    onclick={() => studio.terminalOpen = !studio.terminalOpen}>
    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
      {#if studio.terminalOpen}<path d="M3 15h18v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2Z" fill="currentColor" stroke="none" />{/if}
      <rect x="3" y="3" width="18" height="18" rx="2" />
      <path d="M3 15h18" />
    </svg>
  </IconButton>
  <IconButton id="chat-toggle" class="absolute right-3 {studio.chatOpen ? 'text-title' : 'text-muted'}"
    aria-label={studio.chatOpen ? "Close right panel" : "Open right panel"} aria-expanded={studio.chatOpen} aria-controls="ai-chat-panel" title={studio.chatOpen ? "Close right panel" : "Open right panel"}
    onclick={() => { studio.chatOpen = !studio.chatOpen; if (!studio.chatOpen) studio.chatExpanded = false; }}>
    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
      {#if studio.chatOpen}<path d="M15 3h4a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2h-4Z" fill="currentColor" stroke="none" />{/if}
      <rect x="3" y="3" width="18" height="18" rx="2" />
      <path d="M15 3v18" />
    </svg>
  </IconButton>
  </div>
  {#if customTitlebar}
    <div class="absolute inset-y-0 right-0 flex items-stretch">
      <button type="button" class="w-12 text-muted hover:bg-white/10 hover:text-title" aria-label="Minimize" title="Minimize"
        onclick={() => windowAction(() => getCurrentWindow().minimize())}>
        <svg class="mx-auto" width="12" height="12" viewBox="0 0 12 12" fill="none" stroke="currentColor" aria-hidden="true"><path d="M1 6h10" /></svg>
      </button>
      <button type="button" class="w-12 text-muted hover:bg-white/10 hover:text-title" aria-label={maximized ? "Restore" : "Maximize"} title={maximized ? "Restore" : "Maximize"}
        onclick={() => windowAction(() => getCurrentWindow().toggleMaximize())}>
        <svg class="mx-auto" width="12" height="12" viewBox="0 0 12 12" fill="none" stroke="currentColor" aria-hidden="true">
          {#if maximized}<path d="M4 3V1.5h6.5V8H9" /><rect x="1.5" y="4" width="6.5" height="6.5" />{:else}<rect x="1.5" y="1.5" width="9" height="9" />{/if}
        </svg>
      </button>
      <button type="button" class="w-12 text-muted hover:bg-red-600 hover:text-white" aria-label="Close window" title="Close window"
        onclick={() => windowAction(() => getCurrentWindow().close())}>
        <svg class="mx-auto" width="12" height="12" viewBox="0 0 12 12" fill="none" stroke="currentColor" aria-hidden="true"><path d="m1.5 1.5 9 9m0-9-9 9" /></svg>
      </button>
    </div>
  {/if}
</header>
