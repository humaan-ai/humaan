<script lang="ts">
  import { Button, buttonVariants } from "$lib/components/ui/button/index.js";
  import IconButton from "$lib/components/ui/icon-button.svelte";
  import FolderOpen from "@lucide/svelte/icons/folder-open";
  import Bell from "@lucide/svelte/icons/bell";
  import LoaderCircle from "@lucide/svelte/icons/loader-circle";
  import X from "@lucide/svelte/icons/x";
  import { onMount, tick } from "svelte";
  import { studio } from "$lib/studio.svelte";
  import { feedback, dismissAfterReading } from "$lib/feedback";
  import NoticeCard from "./NoticeCard.svelte";
  import ProblemIcon from './ProblemIcon.svelte';
  import { problemCounts, showProblems } from '$lib/problems.svelte';
  let now = $state(Date.now());
  let bell = $state<HTMLButtonElement | null>(null);
  let panel = $state<HTMLElement>();
  const current = $derived($feedback.activities.at(-1));
  const unread = $derived($feedback.notices.filter(n => n.unread).length);
  const popups = $derived($feedback.notices.filter(n => n.popup).slice(0, 3));
  const problems = $derived(problemCounts());
  const buttonClass = buttonVariants({ variant: "ghost", size: "xs", class: "h-5" });
  onMount(() => { const timer = setInterval(() => now = Date.now(), 1000); return () => clearInterval(timer); });
  async function toggleHistory() {
    if ($feedback.panelOpen) { closeHistory(); return; }
    feedback.openPanel(); await tick(); panel?.querySelector<HTMLButtonElement>("button")?.focus();
  }
  function closeHistory() { feedback.closePanel(); bell?.focus(); }
</script>

<svelte:window onkeydown={(event) => { if (event.key === "Escape" && $feedback.panelOpen) { event.preventDefault(); closeHistory(); } }} onpointerdown={(event) => { if ($feedback.panelOpen && event.target instanceof Node && !panel?.contains(event.target) && !bell?.contains(event.target)) feedback.closePanel(); }} />

<div class="sr-only" role="status" aria-live="polite">{current?.title ?? ($feedback.status?.kind !== "error" ? $feedback.status?.title ?? "" : "")}</div>
<div class="sr-only" role="alert">{$feedback.status?.kind === "error" ? $feedback.status.title : ""}</div>

{#if $feedback.panelOpen}
  <section bind:this={panel} aria-label="Notifications" class="fixed right-3 bottom-9 z-50 flex max-h-[70vh] w-[calc(100%_-_1.5rem)] max-w-md flex-col rounded-xl border border-border-strong bg-surface text-foreground shadow-2xl">
    <header class="flex items-center justify-between border-b border-border px-4 py-3 text-sm"><h2 class="font-medium">Notifications</h2><div class="flex gap-2"><Button variant="ghost" size="xs" class={buttonClass} disabled={!$feedback.notices.length} onclick={() => feedback.clearHistory()}>Clear history</Button><IconButton class={buttonClass} aria-label="Close notifications" onclick={closeHistory}><X size={16} /></IconButton></div></header>
    <div class="min-h-0 space-y-3 overflow-y-auto p-3">
      {#each $feedback.activities as activity (activity.id)}
        <div class="rounded-lg border border-border-strong p-3 text-sm"><p class="flex items-center gap-2"><LoaderCircle size={14} class="animate-spin motion-reduce:animate-none" />{activity.title}</p><p class="mt-2 text-xs text-muted">{Math.max(0, Math.floor((now - activity.startedAt) / 1000))}s elapsed</p>{#if activity.detail}<p class="mt-2 whitespace-pre-wrap break-words text-xs text-muted">{activity.detail}</p>{/if}</div>
      {/each}
      {#each $feedback.notices as notice (notice.id)}<NoticeCard {notice} />{/each}
      {#if !$feedback.activities.length && !$feedback.notices.length}<p class="py-6 text-center text-sm text-muted">No notifications yet.</p>{/if}
    </div>
    <p class="border-t border-border px-4 py-2 text-xs text-muted">Latest 50 events · This project session</p>
  </section>
{:else}
  <div class="pointer-events-none fixed right-3 bottom-9 z-40 flex max-h-[60vh] w-[calc(100%_-_1.5rem)] max-w-sm flex-col gap-2 overflow-y-auto" aria-label="Recent notifications">
    {#each popups as notice (notice.id)}<div class="pointer-events-auto"><NoticeCard {notice} toast /></div>{/each}
  </div>
{/if}

<footer class="flex h-[22px] shrink-0 items-center gap-2 bg-surface px-2 text-[11px] text-foreground">
  <span class="flex min-w-0 max-w-[35%] shrink-0 items-center gap-1.5" title={studio.project?.root}>
    {#if studio.project}<FolderOpen size={12} class="shrink-0" aria-hidden="true" /><span class="truncate">{studio.project.project.name}</span>{:else}<span>No project open</span>{/if}
  </span>
  {#if studio.project}
    <Button variant="ghost" size="xs" class="{buttonClass} gap-2" aria-label={`Show Problems: ${problems.error} errors, ${problems.warning} warnings, ${problems.info} informational notes`} title="Show Problems" onclick={showProblems}>
      {#each ['error', 'warning', 'info'] as const as severity}<span class="flex items-center gap-1 tabular-nums"><ProblemIcon {severity} muted={!problems[severity]} />{problems[severity]}</span>{/each}
    </Button>
  {/if}
  <div class="min-w-0 flex-1">
    {#if current}
      <Button variant="ghost" size="xs" class="{buttonClass} flex max-w-full items-center gap-1.5" aria-label="Show operation details" onclick={toggleHistory}><LoaderCircle size={12} class="shrink-0 animate-spin motion-reduce:animate-none" aria-hidden="true" /><span class="truncate">{current.title}</span><span class="shrink-0 text-muted">{Math.max(0, Math.floor((now - current.startedAt) / 1000))}s</span>{#if $feedback.activities.length > 1}<span>+{$feedback.activities.length - 1}</span>{/if}</Button>
    {:else if $feedback.status}
      {#key $feedback.status.id}
        {@const status = $feedback.status}
        <button use:dismissAfterReading={() => feedback.clearStatus(status.id)} class="{buttonClass} max-w-full truncate text-left {status.kind === 'error' ? 'text-error' : status.kind === 'warning' ? 'text-warning' : ''}" title={status.title} onclick={toggleHistory}>{status.title}</button>
      {/key}
    {/if}
  </div>
  <span class="hidden shrink-0 text-muted sm:inline">Local</span>
  <Button variant="ghost" size="xs" bind:ref={bell} class="{buttonClass} flex shrink-0 items-center gap-1" aria-label={unread ? `Notifications (${unread} unread)` : "Notifications"} aria-expanded={$feedback.panelOpen} title="Notifications" onclick={toggleHistory}><Bell size={12} aria-hidden="true" />{#if unread}<span class={$feedback.notices.some(n => n.unread && n.kind === 'error') ? 'text-error' : 'text-muted'}>{unread}</span>{/if}</Button>
</footer>
