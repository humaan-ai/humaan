<script lang="ts">
  import * as Tooltip from "$lib/components/ui/tooltip/index.js";
  import * as Dialog from "$lib/components/ui/dialog/index.js";
  import IconButton from "$lib/components/ui/icon-button.svelte";
  import { Button } from "$lib/components/ui/button/index.js";
  import CircleCheck from "@lucide/svelte/icons/circle-check";
  import CircleX from "@lucide/svelte/icons/circle-x";
  import Circle from "@lucide/svelte/icons/circle";
  import RefreshCw from "@lucide/svelte/icons/refresh-cw";
  import Info from "@lucide/svelte/icons/info";
  import type { VerificationReport } from "$lib/verification";
  let { report, id }: { report?: VerificationReport; id: string } = $props();
  let detailsOpen = $state(false);
  let detailsButton = $state<HTMLButtonElement | null>(null);
  const status = $derived(report?.status ?? "not_checked");
  const label = $derived(status === "verified" ? "Verified" : status === "failed" ? "Failed" : status === "needs_recheck" ? "Needs recheck" : "Not checked");
  const color = $derived(status === "verified" ? "text-success" : status === "failed" ? "text-error" : status === "needs_recheck" ? "text-warning" : "text-muted");
  const explanation = $derived(status === "verified" ? "Files passed size and SHA-256 checks at the time shown. This is not a live integrity guarantee." : status === "failed" ? report?.error ?? "Files failed integrity checks." : status === "needs_recheck" ? "A file or manifest changed, or changed during verification. Run Verify all again." : "No saved verification result. Use Verify all in Records to check file integrity.");
  const timestamp = $derived(report ? new Date(report.checked_at_unix_ms).toLocaleString() : "Never checked");
</script>

<span class="inline-flex items-center gap-1.5 whitespace-nowrap {color}">
  {#if status === "verified"}<CircleCheck size={13} aria-hidden="true" />{:else if status === "failed"}<CircleX size={13} aria-hidden="true" />{:else if status === "needs_recheck"}<RefreshCw size={13} aria-hidden="true" />{:else}<Circle size={13} aria-hidden="true" />{/if}
  <span>{label}</span>
  <Tooltip.Root>
    <Tooltip.Trigger>
      {#snippet child({ props })}
        <IconButton {...props} bind:ref={detailsButton} aria-label="Verification details for {id}" onclick={(event) => { event.stopPropagation(); detailsOpen = true; }}><Info size={13} aria-hidden="true" /></IconButton>
      {/snippet}
    </Tooltip.Trigger>
    <Tooltip.Content side="bottom" sideOffset={6} class="block w-64 whitespace-normal text-left"><span class="block text-title">{timestamp}</span><span class="mt-1 block">{explanation}</span></Tooltip.Content>
  </Tooltip.Root>
</span>
<Dialog.Root bind:open={detailsOpen}>
{#if detailsOpen}
<Dialog.Content class="max-w-lg" showCloseButton={false} onCloseAutoFocus={(event) => { event.preventDefault(); detailsButton?.focus(); }}>
  <Dialog.Description class="sr-only">Saved integrity verification details for this recording.</Dialog.Description>
  <Dialog.Title>Verification: {label}</Dialog.Title>
  <p class="mt-3 break-all font-mono text-xs text-muted">{id}</p>
  <p class="mt-3 text-xs text-muted">Last check: {timestamp}</p>
  <p class="mt-4 whitespace-pre-wrap break-words">{explanation}</p>
  {#if report?.status === "needs_recheck" && report.error}<p class="mt-3 whitespace-pre-wrap break-words text-error">Previous check: {report.error}</p>{/if}
  <p class="mt-4 text-xs text-muted">Checks cover file integrity, not task outcome or review quality. Refresh detects ordinary file size and modification-time changes; Verify all rechecks hashes.</p>
  <div class="mt-5 flex justify-end"><Button onclick={(event) => { event.stopPropagation(); detailsOpen = false; }}>Close</Button></div>
</Dialog.Content>
{/if}
</Dialog.Root>
