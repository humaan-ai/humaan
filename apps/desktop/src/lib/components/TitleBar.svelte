<script lang="ts">
  import IconButton from "$lib/components/ui/icon-button.svelte";
  import { studio } from "$lib/studio.svelte";
</script>

<header data-tauri-drag-region class="relative flex h-10 shrink-0 items-center justify-center bg-surface px-20 select-none">
  <span class="pointer-events-none truncate text-xs text-muted">{studio.project?.project.name ?? "Humaan Studio"}</span>
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
</header>
