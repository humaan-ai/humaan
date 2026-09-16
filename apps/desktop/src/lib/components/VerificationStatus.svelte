<script lang="ts">
  import CircleCheck from "@lucide/svelte/icons/circle-check";
  import CircleX from "@lucide/svelte/icons/circle-x";
  import Circle from "@lucide/svelte/icons/circle";
  import RefreshCw from "@lucide/svelte/icons/refresh-cw";
  import Info from "@lucide/svelte/icons/info";
  import type { VerificationReport } from "$lib/verification";
  let { report, id }: { report?: VerificationReport; id: string } = $props();
  const tooltipId = $props.id();
  let tooltipPosition = $state({ left: 0, top: 0 });
  function positionTooltip(event: Event) {
    const bounds = (event.currentTarget as HTMLElement).getBoundingClientRect();
    tooltipPosition = { left: Math.max(8, Math.min(bounds.left, window.innerWidth - 272)), top: Math.min(bounds.bottom + 6, window.innerHeight - 180) };
  }
  let dialog = $state<HTMLDialogElement>();
  const status = $derived(report?.status ?? "not_checked");
  const label = $derived(status === "verified" ? "Verified" : status === "failed" ? "Failed" : status === "needs_recheck" ? "Needs recheck" : "Not checked");
  const color = $derived(status === "verified" ? "text-success" : status === "failed" ? "text-error" : status === "needs_recheck" ? "text-warning" : "text-muted");
  const explanation = $derived(status === "verified" ? "Files passed size and SHA-256 checks at the time shown. This is not a live integrity guarantee." : status === "failed" ? report?.error ?? "Files failed integrity checks." : status === "needs_recheck" ? "A file or manifest changed, or changed during verification. Run Verify all again." : "No saved verification result. Use Verify all in Records to check file integrity.");
  const timestamp = $derived(report ? new Date(report.checked_at_unix_ms).toLocaleString() : "Never checked");
</script>

<span class="inline-flex items-center gap-1.5 whitespace-nowrap {color}">
  {#if status === "verified"}<CircleCheck size={13} aria-hidden="true" />{:else if status === "failed"}<CircleX size={13} aria-hidden="true" />{:else if status === "needs_recheck"}<RefreshCw size={13} aria-hidden="true" />{:else}<Circle size={13} aria-hidden="true" />{/if}
  <span>{label}</span>
  <span class="group relative inline-flex">
    <button class="rounded p-1 text-muted hover:text-title focus-visible:outline-2 focus-visible:outline-title/40" aria-label="Verification details for {id}" aria-describedby={tooltipId} onpointerenter={positionTooltip} onfocus={positionTooltip} onclick={(event) => { event.stopPropagation(); dialog?.showModal(); }}><Info size={13} aria-hidden="true" /></button>
    <span id={tooltipId} role="tooltip" style:left="{tooltipPosition.left}px" style:top="{tooltipPosition.top}px" class="pointer-events-none fixed z-40 hidden w-64 whitespace-normal rounded border border-border-strong bg-raised p-2 text-left text-xs text-foreground shadow-lg group-hover:block group-focus-within:block"><span class="block text-title">{timestamp}</span><span class="mt-1 block">{explanation}</span></span>
  </span>
</span>
<dialog bind:this={dialog} aria-label="Verification details" class="fixed m-auto w-[calc(100%_-_3rem)] max-w-lg rounded-xl border border-border-strong bg-background p-6 text-sm text-foreground shadow-2xl backdrop:bg-black/60">
  <h2 class="text-lg font-medium">Verification: {label}</h2>
  <p class="mt-3 break-all font-mono text-xs text-muted">{id}</p>
  <p class="mt-3 text-xs text-muted">Last check: {timestamp}</p>
  <p class="mt-4 whitespace-pre-wrap break-words">{explanation}</p>
  {#if report?.status === "needs_recheck" && report.error}<p class="mt-3 whitespace-pre-wrap break-words text-error">Previous check: {report.error}</p>{/if}
  <p class="mt-4 text-xs text-muted">Checks cover file integrity, not task outcome or review quality. Refresh detects ordinary file size and modification-time changes; Verify all rechecks hashes.</p>
  <div class="mt-5 flex justify-end"><button class="rounded border border-border-strong bg-raised px-3 py-2 hover:bg-hover focus-visible:outline-2 focus-visible:outline-title/40" onclick={(event) => { event.stopPropagation(); dialog?.close(); }}>Close</button></div>
</dialog>
