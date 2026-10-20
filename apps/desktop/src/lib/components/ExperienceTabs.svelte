<script lang="ts">
  import IconButton from "$lib/components/ui/icon-button.svelte";
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

<div class="flex h-[35px] shrink-0 items-center bg-[#222222] shadow-[inset_0_-1px_0_var(--color-border)]">
  <div role="tablist" tabindex={-1} aria-label="Workspace tabs" class="flex h-full min-w-0 overflow-x-auto" onkeydown={navigate}>
    {#each tabs as tab, index}
      <div role="presentation" class="group/tab flex h-full max-w-64 shrink-0 items-center rounded-tr-lg {index === 0 ? 'rounded-tl-none border-l-0' : '-ml-px rounded-tl-lg'} border border-y-0 {active === tab.id ? 'relative z-10 border-border bg-background text-title' : 'border-transparent bg-[#222222] text-muted shadow-[inset_0_-1px_0_var(--color-border)] hover:bg-[#292929]'}">
        <button role="tab" id="experience-tab-{tab.id}" aria-controls="experience-panel-{tab.id}" aria-selected={active === tab.id} tabindex={active === tab.id ? 0 : -1}
          class="flex h-full min-w-0 flex-1 items-center gap-2 px-3 text-xs focus-visible:outline-1 focus-visible:outline-ring focus-visible:-outline-offset-2"
          title={tab.label} onclick={() => onselect(tab.id)}>
          {#if tab.id === "overview"}<LayoutDashboard size={14} class="shrink-0" aria-hidden="true" />
          {:else if tab.id === "records"}<Table size={14} class="shrink-0" aria-hidden="true" />
          {:else if tab.id === "detail"}<Play size={14} class="shrink-0" aria-hidden="true" />{/if}
          <span class="truncate">{tab.label}</span>{#if tab.id === "detail" && dirty}<span aria-label="Unsaved review" class="h-1.5 w-1.5 shrink-0 rounded-full bg-accent"></span>{/if}
        </button>
        {#if tab.id === "detail"}
          <IconButton class="mr-2 shrink-0 text-muted {active === tab.id ? '' : 'opacity-0 group-hover/tab:opacity-100 group-focus-within/tab:opacity-100'}" aria-label="Close experience tab" title={dirty ? "Save or discard your review before closing" : "Close experience tab"} disabled={dirty || busy} onclick={onclose}><X size={14} aria-hidden="true" /></IconButton>
        {/if}
      </div>
    {/each}
  </div>

</div>
