<script lang="ts">
  import { Checkbox } from "$lib/components/ui/checkbox/index.js";
  import * as Dialog from "$lib/components/ui/dialog/index.js";
  import { Button } from "$lib/components/ui/button/index.js";
  import IconButton from "$lib/components/ui/icon-button.svelte";
  import { Input } from "$lib/components/ui/input/index.js";
  import { Textarea } from "$lib/components/ui/textarea/index.js";
  import { NativeSelect } from "$lib/components/ui/native-select/index.js";
  import ArrowLeft from "@lucide/svelte/icons/arrow-left";
  import { tick } from "svelte";
  import Plus from "@lucide/svelte/icons/plus";
  import RefreshCw from "@lucide/svelte/icons/refresh-cw";
  import ActionLabel from "./ActionLabel.svelte";
  import { feedback } from "$lib/feedback";
  import ExperienceNavigation from "./ExperienceNavigation.svelte";
  import LoopNavigation from "./LoopNavigation.svelte";
  import WorkspaceColumn from "./WorkspaceColumn.svelte";
  import { studio } from "$lib/studio.svelte";
  import StudioSidebar from "./StudioSidebar.svelte";
  import DatasetPreparation from "./DatasetPreparation.svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";
  type Review = { outcome: string; assistance: string; review_state: string; reviewer: string; notes: string };
  type Revision = { revision: number; review: Review };
  type Experience = { id: string; task: string; robot: string };
  type LearningSet = { id: string; name: string; learning_goal: string; curator: string; created_at_unix_ms: number; members: { experience_id: string; reason: string; review: Revision }[] };
  let { root, experiences, reviews, busy = $bindable(false), dirty = $bindable(false), blocked = false }: {
    root: string; experiences: Experience[]; reviews: Record<string, { revisions: Revision[] }>;
    busy?: boolean; dirty?: boolean; blocked?: boolean;
  } = $props();
  const inputClass = "mt-1 block w-full";
  let activeOperation = $state("");
  let selectedSet = $state<string | null>(null);
  let sets = $state<LearningSet[]>([]);
  let discardOpen = $state(false);
  let setQuery = $state("");
  let setFilter = $state("all");
  const visibleSets = $derived(sets.filter(set =>
    `${set.name} ${set.learning_goal} ${set.curator}`.toLowerCase().includes(setQuery.trim().toLowerCase()) &&
    (setFilter === "all" || set.members.some(member => member.review.review.outcome === setFilter))
  ));
  async function selectTab(id: string | null) {
    selectedSet = id;
    await tick();
    document.getElementById("loop-panel-learning-sets")?.focus();
  }
  async function closeTab(id: string) {
    if (busy) return;
    if (id === "draft" && dirty) { discardOpen = true; return; }
    await selectTab(null);
  }
  async function discardDraft() {
    discard();
    dirty = false;
    discardOpen = false;
    await selectTab(null);
  }

  let selected = $state<Record<string, { review_revision: number; reason: string }>>({});
  let name = $state("");
  let goal = $state("");
  let curator = $state("");
  let query = $state("");
  let filter = $state("all");
  let error = $state("");
  let preparingSet = $state("");
  let loaded = $state(false);
  function latest(id: string) { return reviews[id]?.revisions.at(-1); }
  const candidates = $derived(experiences.filter(e => {
    const review = latest(e.id)?.review;
    return review?.review_state === "reviewed" &&
      (filter === "all" || (filter === "assisted" ? review.assistance === "yes" : review.outcome === filter)) &&
      `${e.id} ${e.robot} ${e.task}`.toLowerCase().includes(query.toLowerCase());
  }));
  const selectionIds = $derived(Object.keys(selected));
  $effect(() => { dirty = !!(name || goal || curator || selectionIds.length); });
  async function work(operation: () => Promise<void>, label = "Loading learning sets") {
    if (busy) return;
    busy = true; activeOperation = label; error = "";
    const activity = label === "Loading learning sets" ? null : feedback.start(label);
    try { await operation(); }
    catch (e) { feedback.publish({ title: `${label} failed`, detail: String(e), kind: "error", popup: true }); }
    finally { if (activity !== null) feedback.finish(activity); busy = false; activeOperation = ""; }
  }
  async function refresh() {
    sets = await invoke<LearningSet[]>("learning_set_list", { root }); loaded = true;
  }
  onMount(() => { work(refresh); });
  function toggle(id: string) {
    if (selected[id]) { delete selected[id]; return; }
    const revision = latest(id);
    if (revision?.review.review_state === "reviewed") selected[id] = { review_revision: revision.revision, reason: "" };
  }
  function discard() { selected = {}; name = ""; goal = ""; curator = ""; error = ""; }
  async function save() {
    if (blocked || busy) return;
    if (!selectionIds.length) { error = "Select at least one reviewed experience."; return; }
    if (selectionIds.some(id => !selected[id].reason.trim())) { error = "Explain why each selected experience is useful."; return; }
    await work(async () => {
      const set = await invoke<LearningSet>("learning_set_create", { root, input: {
        name, learning_goal: goal, curator,
        selections: selectionIds.map(experience_id => ({ experience_id, ...selected[experience_id] }))
      } });
      sets = [set, ...sets];
      discard();
      await selectTab(set.id);
      feedback.publish({ title: "Learning set saved", detail: `“${set.name}” · ${set.members.length} experiences`, kind: "success" });
    }, "Saving learning set");
  }
  async function exportSet(id: string, files = false) {
    if (blocked || busy) return;
    const label = files ? "Exporting learning-set files" : "Exporting selection manifest";
    await work(async () => {
      const parent = await invoke<string | null>("select_directory", { title: files ? "Choose parent folder for experience files" : "Export learning set selection manifest" });
      if (!parent) return;
      const destination = await invoke<string>(files ? "learning_set_export_files" : "learning_set_export", { root, id, parent });
      feedback.publish({ title: files ? "Learning-set files exported" : "Selection manifest exported", detail: destination, copyText: destination, kind: "success", popup: true });
    }, label);
  }
</script>

<div class="flex h-full min-h-0 gap-1 overflow-x-auto">
<StudioSidebar title="Experiences">
  <ExperienceNavigation {busy} reviewDirty={blocked} loopDirty={dirty} />
  <div class="px-4 py-3 text-xs text-muted">Learning sets ({visibleSets.length} of {sets.length})</div>
  <div class="min-h-0 flex-1 overflow-y-auto">
    {#each visibleSets as set (set.id)}
      <Button variant="ghost" class="block h-auto w-full whitespace-normal rounded-none px-4 py-3 text-left {selectedSet === set.id ? 'bg-selection' : 'bg-transparent'}" aria-current={selectedSet === set.id ? "page" : undefined} disabled={busy} onclick={() => selectTab(set.id)}>
        <span class="block truncate">{set.name}</span><span class="mt-1 block text-xs text-muted">{set.members.length} experiences</span>
      </Button>
    {:else}<p class="px-4 py-3 text-xs text-muted">{loaded ? (sets.length ? "No matching learning sets." : "No saved learning sets yet.") : "Loading learning sets…"}</p>{/each}
  </div>
</StudioSidebar>
<WorkspaceColumn active={studio.activeView === "loop"}>
<div class="workbench-panel flex min-h-0 min-w-80 flex-1 flex-col overflow-hidden">
  <LoopNavigation instance="learning-sets" learningSetDirty={dirty} />
  <div id="loop-workspace-learning-sets" role="tabpanel" aria-labelledby="loop-section-learning-sets-learning-sets" class="flex min-h-0 flex-1 flex-col">
  <div role="group" aria-label="Learning set actions" class="flex min-h-[57px] shrink-0 flex-wrap items-center gap-3 border-b border-border px-4 py-3">
    <Button variant="default" aria-pressed={selectedSet === 'draft'} disabled={busy} onclick={() => selectTab("draft")}><Plus size={14} />{dirty ? 'Continue draft' : 'Create learning set'}</Button>
    <Input class="min-w-32 max-w-56 flex-1" aria-label="Search learning sets" placeholder="Search learning sets" bind:value={setQuery} />
    <NativeSelect class="w-auto" aria-label="Filter learning sets" bind:value={setFilter}><option value="all">All learning sets</option><option value="failure">Includes failures</option><option value="success">Includes successes</option></NativeSelect>
    <IconButton variant="outline" size="icon" aria-label="Refresh learning sets" title="Refresh learning sets" disabled={busy} onclick={() => work(refresh)}><RefreshCw size={14} class={activeOperation === 'Loading learning sets' ? 'animate-spin motion-reduce:animate-none' : ''} /></IconButton>
    {#if selectedSet !== null}<Button class="ml-auto" disabled={busy} onclick={() => closeTab(selectedSet!)}><ArrowLeft size={14} />Back to learning sets</Button>{/if}
  </div>
<div id="loop-panel-learning-sets" role="region" aria-label="Learning set content" tabindex="-1" class="min-h-0 flex-1 overflow-y-auto px-7 py-6" aria-busy={busy}>
  {#if blocked}<p class="mb-4 rounded border border-warning/40 p-3 text-warning">Save or discard your experience review before curating. Your learning-set draft is preserved.</p>{/if}
  {#if selectedSet === null}
    {#if !loaded}
      <p class="text-muted" role="status">Loading learning sets…</p>
    {:else if !sets.length}
      <div class="flex min-h-48 flex-col items-center justify-center gap-3 text-center">
        <h2 class="text-lg font-medium">No learning sets yet</h2>
        <p class="max-w-sm text-muted">Create a learning set from reviewed experiences.</p>
        <Button variant="default" disabled={busy || blocked} onclick={() => selectTab("draft")}><Plus size={14} />{dirty ? 'Continue draft' : 'Create learning set'}</Button>
      </div>
    {:else}
      <div class="overflow-x-auto">
        <table class="w-full text-left text-sm" aria-label="Learning sets">
          <thead class="border-b border-border text-xs text-muted"><tr><th class="px-3 py-3 font-medium">Name</th><th class="px-3 py-3 font-medium">Learning goal</th><th class="px-3 py-3 font-medium">Experiences</th><th class="px-3 py-3 font-medium">Curator</th><th class="px-3 py-3 font-medium">Created</th></tr></thead>
          <tbody>
            {#each visibleSets as set (set.id)}
              <tr class="border-b border-border hover:bg-hover">
                <td class="px-3 py-3"><button class="text-left font-medium text-title hover:underline focus-visible:outline focus-visible:outline-ring disabled:opacity-45" disabled={busy} onclick={() => selectTab(set.id)}>{set.name}</button></td>
                <td class="max-w-80 px-3 py-3"><p class="line-clamp-2" title={set.learning_goal}>{set.learning_goal}</p></td>
                <td class="px-3 py-3 tabular-nums">{set.members.length}</td>
                <td class="px-3 py-3">{set.curator}</td>
                <td class="whitespace-nowrap px-3 py-3 text-xs text-muted">{new Date(set.created_at_unix_ms).toLocaleString()}</td>
              </tr>
            {:else}
              <tr><td colspan="5" class="px-3 py-8 text-center text-muted">No matching learning sets. <Button variant="ghost" onclick={() => { setQuery = ''; setFilter = 'all'; }}>Clear filters</Button></td></tr>
            {/each}
          </tbody>
        </table>
      </div>
    {/if}
  {/if}
  {#if error}<p class="mt-4 text-error" role="alert">{error}</p>{/if}
  <div hidden={selectedSet !== "draft"}>
  <h2 class="text-xl font-medium">Select useful experience</h2>
  <p class="mt-2 max-w-3xl text-muted">Choose reviewed attempts that address a learning goal. A success can provide an example; a failure can reveal what needs improvement. Assistance alone does not identify a corrected trajectory.</p>

  <form class="mt-5" onsubmit={(e) => { e.preventDefault(); save(); }}>
    <fieldset class="disabled:opacity-50" disabled={busy || blocked}>
      <div class="grid gap-4 md:grid-cols-2">
        <label>Learning set name<Input class={inputClass} bind:value={name} required maxlength={200} placeholder="Recover from grasp slips" /></label>
        <label>Curator<Input class={inputClass} bind:value={curator} required maxlength={200} placeholder="Your name" /></label>
        <label class="md:col-span-2">Learning goal<Textarea class={inputClass} bind:value={goal} required maxlength={4000} rows={2} placeholder="What should the robot learn from these attempts?"></Textarea></label>
      </div>
      <div class="my-5 flex flex-wrap items-end gap-3">
        <label class="flex-1">Find reviewed experiences<Input class={inputClass} bind:value={query} placeholder="Search task, robot, or ID" /></label>
        <label>Evidence filter<NativeSelect class={inputClass} bind:value={filter}><option value="all">All reviewed</option><option value="success">Success</option><option value="assisted">Human assisted</option><option value="failure">Failure</option><option value="timeout">Timeout</option><option value="aborted">Aborted</option><option value="unknown">Unknown outcome</option></NativeSelect></label>
        <span class="pb-2 text-xs text-muted">{candidates.length} matching · {selectionIds.length} selected</span>
      </div>
      <div class="max-h-72 overflow-y-auto rounded border border-border-strong">
        {#each candidates as experience (experience.id)}
          <label class="flex items-start gap-3 border-b border-border p-3">
            <Checkbox class="mt-1" checked={!!selected[experience.id]} onCheckedChange={() => toggle(experience.id)} aria-label={`Include ${experience.task}`} />
            <span class="min-w-0"><span class="block">{experience.task} · {experience.robot}</span><span class="block break-all text-xs text-muted">{experience.id} · {latest(experience.id)?.review.outcome} · Assistance: {latest(experience.id)?.review.assistance}</span><span class="block whitespace-pre-wrap text-xs text-muted">{latest(experience.id)?.review.notes}</span></span>
          </label>
        {:else}<p class="p-4 text-muted">No reviewed experiences match. Review attempts in Experiences first, or change the filter.</p>{/each}
      </div>
      {#if selectionIds.length}
        <h3 class="mt-5 font-medium">Selection rationale</h3>
        <p class="mt-1 text-xs text-muted">Selections remain when filters change. Include only evidence relevant to your goal.</p>
        {#each selectionIds as id (id)}
          <div class="mt-3 rounded border border-border-strong p-3">
            <div class="mb-2 flex items-center justify-between gap-3"><span class="break-all text-xs">{id} · review revision {selected[id].review_revision}</span><Button type="button" onclick={() => toggle(id)}>Remove</Button></div>
            <label>Why include this attempt?<Textarea class={inputClass} bind:value={selected[id].reason} required maxlength={4000} rows={2} placeholder="Useful example, failure evidence, or recovery worth studying"></Textarea></label>
          </div>
        {/each}
      {/if}
      <div class="mt-4 flex gap-3"><Button variant="default" type="submit" disabled={!selectionIds.length}><ActionLabel active={activeOperation === "Saving learning set"} label="Save learning set" pending="Saving…" /></Button>{#if dirty}<Button type="button" onclick={discard}>Discard selection</Button>{/if}</div>
    </fieldset>
  </form>
  <p class="mt-4 text-xs text-muted">Saved sets preserve whole-attempt selections and review snapshots. Use dataset preparation to validate supported imitation examples. Correction segments are not yet supported.</p>
  </div>
  {#each sets as set (set.id)}
    <section hidden={selectedSet !== set.id} aria-label={set.name}>
      <h2 class="text-xl font-medium">{set.name}</h2>
      <p class="mt-2 text-sm text-muted">{set.members.length} experiences</p>
      <p class="mt-3 whitespace-pre-wrap">{set.learning_goal}</p>
      <p class="mt-2 text-xs text-muted">{set.curator} · {new Date(set.created_at_unix_ms).toLocaleString()} · Saved selection</p>
      {#each set.members as member}<div class="mt-3 border-t border-border pt-3"><p class="break-all text-xs">{member.experience_id} · {member.review.review.outcome} · review revision {member.review.revision}</p><p class="mt-1 whitespace-pre-wrap text-muted">{member.reason}</p></div>{/each}
      <div class="mt-4 flex flex-wrap gap-3">
        <Button disabled={busy || blocked} onclick={() => preparingSet = preparingSet === set.id ? "" : set.id}>Prepare training dataset</Button>
        <Button variant="default" disabled={busy || blocked} onclick={() => exportSet(set.id, true)}><ActionLabel active={activeOperation === "Exporting learning-set files"} label="Export experience files" pending="Exporting…" /></Button>
        <Button disabled={busy || blocked} onclick={() => exportSet(set.id)}><ActionLabel active={activeOperation === "Exporting selection manifest"} label="Export selection manifest only" pending="Exporting…" /></Button>
      </div>
      {#if preparingSet === set.id}<fieldset disabled={blocked}><DatasetPreparation {root} {set} bind:busy /></fieldset>{/if}
      <p class="mt-2 text-xs text-muted">Experience export includes original files and review history. Shared video/data chunks remain whole; dataset preparation is still required before training.</p>
    </section>
  {/each}
</div>
</div>
</div>
</WorkspaceColumn>
</div>

<Dialog.Root bind:open={discardOpen}>
{#if discardOpen}
<Dialog.Content showCloseButton={false} onCloseAutoFocus={(event) => { event.preventDefault(); document.getElementById("loop-panel-learning-sets")?.focus(); }}>
  <Dialog.Title>Discard this draft?</Dialog.Title>
  <Dialog.Description class="mt-3">The learning set details and experience selections have not been saved.</Dialog.Description>
  <div class="mt-5 flex justify-end gap-3">
    <Button onclick={() => discardOpen = false}>Keep editing</Button>
    <Button variant="destructive" onclick={discardDraft}>Discard draft</Button>
  </div>
</Dialog.Content>
{/if}
</Dialog.Root>
