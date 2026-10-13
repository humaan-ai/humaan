<script lang="ts">
  import { Button } from "$lib/components/ui/button/index.js";
  import { studio } from "$lib/studio.svelte";
  import ExperienceStatus from "./ExperienceStatus.svelte";
  let { name, experiences, reviews, loaded, onrecords }: {
    name: string; experiences: { id: string; robot: string; task: string }[];
    reviews: Record<string, { revisions: { saved_at_unix_ms: number; review: { review_state: string; outcome: string } }[] }>;
    loaded: boolean; onrecords: (queue: string) => void;
  } = $props();
  const reviewed = $derived(experiences.filter(e => reviews[e.id]?.revisions.at(-1)?.review.review_state === "reviewed").length);
  const recent = $derived(experiences.flatMap(e => {
    const revision = reviews[e.id]?.revisions.at(-1);
    return revision ? [{ ...e, revision }] : [];
  }).sort((a, b) => b.revision.saved_at_unix_ms - a.revision.saved_at_unix_ms).slice(0, 5));
</script>

<div class="mx-auto w-full max-w-5xl p-7">
  <p class="text-xs uppercase tracking-widest text-muted">Project overview</p>
  <h1 class="mt-2 text-2xl font-medium">{name}</h1>
  {#if !loaded}<p class="mt-5 text-muted">Catalog not loaded. Open Records and refresh to load the project summary.</p>
  {:else}
    <dl class="mt-6 grid gap-4 sm:grid-cols-3">
      {#each [{label: "Records", value: experiences.length}, {label: "Reviewed", value: reviewed}, {label: "Awaiting review", value: experiences.length - reviewed}] as metric}
        <div class="rounded-lg border border-border-strong bg-surface p-5"><dt class="text-xs text-muted">{metric.label}</dt><dd class="mt-2 text-3xl text-title">{metric.value}</dd></div>
      {/each}
    </dl>
    <section class="mt-6 rounded-lg border border-border-strong p-5" aria-label="Review progress">
      <h2 class="font-medium">Review progress</h2>
      <progress class="mt-3 h-2 w-full accent-accent" aria-label="Reviewed records" value={reviewed} max={Math.max(1, experiences.length)}></progress>
      <p class="mt-2 text-xs text-muted">{reviewed} of {experiences.length} reviewed</p>
      {#if experiences.length > reviewed}<Button class="mt-4" onclick={() => onrecords("unreviewed")}>Review remaining records</Button>{/if}
      {#if !experiences.length}<p class="mt-3 text-sm text-muted">Import recordings from the Records tab to get started.</p>{/if}
    </section>
    <section class="mt-6" aria-label="Outcome breakdown"><h2 class="font-medium">Final outcomes</h2><div class="mt-3 grid gap-3 sm:grid-cols-3">
      {#each ["success", "failure", "timeout", "aborted", "unknown"] as outcome}
        <div class="flex items-center justify-between rounded border border-border-strong p-3 text-sm"><ExperienceStatus kind="outcome" value={outcome} /><span>{experiences.filter(e => (reviews[e.id]?.revisions.at(-1)?.review.outcome ?? "unknown") === outcome).length}</span></div>
      {/each}
    </div></section>
    <section class="mt-6" aria-label="Recent reviews"><h2 class="font-medium">Recent reviews</h2>
      {#each recent as item}<div class="mt-3 flex flex-wrap justify-between gap-2 border-b border-border pb-3 text-sm"><span>{item.task}</span><time class="text-xs text-muted" datetime={new Date(item.revision.saved_at_unix_ms).toISOString()}>{new Date(item.revision.saved_at_unix_ms).toLocaleString()}</time></div>
      {:else}<p class="mt-3 text-sm text-muted">No reviews yet.</p>{/each}
    </section>
  {/if}
  <section class="mt-6 rounded-lg border border-border-strong p-5"><h2 class="font-medium">Physical robot sessions</h2><p class="mt-2 text-sm text-muted">Inspect measured joint motion with its saved robot version. Session evaluation and learning-set integration are planned.</p><Button class="mt-4" onclick={() => { studio.deploymentSection = "sessions"; studio.activeView = "deployments"; }}>Browse robot sessions</Button></section>
  <Button class="mt-6" onclick={() => onrecords("all")}>Browse records</Button>
</div>
