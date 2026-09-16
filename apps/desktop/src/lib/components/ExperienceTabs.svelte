<script lang="ts">
  import LayoutDashboard from "@lucide/svelte/icons/layout-dashboard";
  import Table from "@lucide/svelte/icons/table";
  import Play from "@lucide/svelte/icons/play";
  import X from "@lucide/svelte/icons/x";
  let { active, detail, dirty, busy, onselect, onclose }: {
    active: "overview" | "records" | "detail"; detail?: string; dirty: boolean; busy: boolean;
    onselect: (tab: "overview" | "records" | "detail") => void; onclose: () => void;
  } = $props();
  const tabs = $derived([{ id: "overview" as const, label: "Overview" }, { id: "records" as const, label: "Records" }, ...(detail ? [{ id: "detail" as const, label: detail }] : [])]);
  function navigate(event: KeyboardEvent) {
    if (!["ArrowLeft", "ArrowRight", "Home", "End"].includes(event.key)) return;
    event.preventDefault();
    const buttons = Array.from((event.currentTarget as HTMLElement).querySelectorAll<HTMLButtonElement>('[role="tab"]'));
    const index = buttons.indexOf(event.target as HTMLButtonElement);
    if (index < 0) return;
    const next = event.key === "Home" ? 0 : event.key === "End" ? buttons.length - 1 : (index + (event.key === "ArrowRight" ? 1 : -1) + buttons.length) % buttons.length;
    buttons[next]?.focus(); onselect(tabs[next].id);
  }
</script>

<div class="flex shrink-0 items-center border-b border-border bg-surface">
  <div role="tablist" tabindex="-1" aria-label="Workspace tabs" class="flex min-w-0 overflow-x-auto" onkeydown={navigate}>
    {#each tabs as tab}
      <div role="presentation" class="flex h-10 min-w-24 max-w-64 shrink-0 items-center border-r border-border {active === tab.id ? 'bg-background text-title' : 'text-muted hover:bg-hover'}">
        <button role="tab" id="experience-tab-{tab.id}" aria-controls="experience-panel-{tab.id}" aria-selected={active === tab.id} tabindex={active === tab.id ? 0 : -1}
          class="flex h-full min-w-0 flex-1 items-center gap-2 px-4 text-xs focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-title/40"
          title={tab.label} onclick={() => onselect(tab.id)}>
          {#if tab.id === "overview"}<LayoutDashboard size={14} class="shrink-0" aria-hidden="true" />
          {:else if tab.id === "records"}<Table size={14} class="shrink-0" aria-hidden="true" />
          {:else if tab.id === "detail"}<Play size={14} class="shrink-0" aria-hidden="true" />{/if}
          <span class="truncate">{tab.label}</span>{#if tab.id === "detail" && dirty}<span aria-label="Unsaved review" class="h-1.5 w-1.5 shrink-0 rounded-full bg-accent"></span>{/if}
        </button>
        {#if tab.id === "detail"}
          <button class="mr-2 shrink-0 rounded p-1 text-muted hover:bg-hover disabled:opacity-45 focus-visible:outline-2 focus-visible:outline-title/40" aria-label="Close experience tab" title={dirty ? "Save or discard your review before closing" : "Close experience tab"} disabled={dirty || busy} onclick={onclose}><X size={14} aria-hidden="true" /></button>
        {/if}
      </div>
    {/each}
  </div>

</div>
