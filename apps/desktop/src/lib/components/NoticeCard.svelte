<script lang="ts">
  import X from "@lucide/svelte/icons/x";
  import CircleCheck from "@lucide/svelte/icons/circle-check";
  import TriangleAlert from "@lucide/svelte/icons/triangle-alert";
  import Info from "@lucide/svelte/icons/info";
  import { feedback, dismissAfterReading, type Notice } from "$lib/feedback";
  let { notice, toast = false }: { notice: Notice; toast?: boolean } = $props();
  let copyStatus = $state("");
  const actionClass = "rounded px-1 py-0.5 text-xs text-link hover:bg-hover focus-visible:outline-2 focus-visible:outline-accent";
  function expiry(node: HTMLElement) {
    if (toast && notice.kind !== "error" && notice.kind !== "warning") return dismissAfterReading(node, () => feedback.dismiss(notice.id), 4000);
  }
  async function copy() {
    try { await navigator.clipboard.writeText(notice.copyText ?? notice.detail ?? notice.title); copyStatus = "Copied"; }
    catch { copyStatus = "Could not copy. Select the text in Details to copy manually."; }
  }
</script>

<article use:expiry class="rounded-lg border border-border-strong bg-raised p-3 text-sm shadow-lg" aria-label={notice.title}>
  <div class="flex items-start gap-2">
    {#if notice.kind === "error" || notice.kind === "warning"}<TriangleAlert size={16} class="mt-0.5 shrink-0 {notice.kind === 'error' ? 'text-error' : 'text-warning'}" aria-hidden="true" />
    {:else if notice.kind === "success"}<CircleCheck size={16} class="mt-0.5 shrink-0 text-success" aria-hidden="true" />
    {:else}<Info size={16} class="mt-0.5 shrink-0 text-muted" aria-hidden="true" />{/if}
    <div class="min-w-0 flex-1"><p class="font-medium text-title">{notice.title}</p>
      {#if !toast}<p class="mt-1 text-xs text-muted">{notice.kind} · {new Date(notice.createdAt).toLocaleTimeString()}</p>{/if}
    </div>
    {#if toast}<button class="rounded p-0.5 text-muted hover:bg-hover focus-visible:outline-2 focus-visible:outline-accent" aria-label="Dismiss notification" onclick={() => feedback.dismiss(notice.id)}><X size={14} aria-hidden="true" /></button>{/if}
  </div>
  {#if notice.detail}
    <details class="mt-2 text-xs text-muted" open={!toast}>
      <summary class="cursor-pointer rounded focus-visible:outline-2 focus-visible:outline-accent">Details</summary>
      <p class="mt-2 max-h-48 overflow-auto whitespace-pre-wrap break-words select-text">{notice.detail}</p>
    </details>
  {/if}
  <div class="mt-2 flex flex-wrap gap-3">
    {#if notice.action}<button class={actionClass} onclick={() => { feedback.dismiss(notice.id); void notice.action?.run(); }}>{notice.action.label}</button>{/if}
    {#if notice.copyText || notice.kind === "error"}<button class={actionClass} onclick={copy}>{notice.copyText ? "Copy path" : "Copy details"}</button>{/if}
    {#if toast}<button class={actionClass} onclick={() => feedback.openPanel()}>View history</button>{/if}
  </div>
  {#if copyStatus}<p class="mt-2 text-xs text-muted" role="status">{copyStatus}</p>{/if}
</article>
