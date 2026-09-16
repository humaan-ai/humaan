<script lang="ts">
  import LayoutDashboard from "@lucide/svelte/icons/layout-dashboard";
  import X from "@lucide/svelte/icons/x";
  import { tick } from "svelte";
  import Layers from "@lucide/svelte/icons/layers";
  import ActionLabel from "./ActionLabel.svelte";
  import { feedback } from "$lib/feedback";
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
  const focusClass = "focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-accent";
  const buttonClass = "rounded-md border border-border-strong px-3 py-2 text-xs disabled:opacity-45 " + focusClass;
  const inputClass = "mt-1 block w-full rounded-md border border-border-strong bg-raised p-2.5 text-sm " + focusClass;
  let activeOperation = $state("");
  let selectedSet = $state<string | null>(null);
  let sets = $state<LearningSet[]>([]);
  let openTabs = $state<string[]>([]);
  let history: (string | null)[] = [];
  let discardDialog: HTMLDialogElement;
  const tabs = $derived([{ id: null, label: "Overview" }, ...openTabs.map(id => ({ id, label: id === "draft" ? "New learning set" : sets.find(set => set.id === id)?.name ?? "Learning set" }))]);
  async function selectTab(id: string | null) {
    if (id !== null && !openTabs.includes(id)) openTabs = [...openTabs, id];
    if (selectedSet !== id) history = [...history.filter(value => value !== selectedSet), selectedSet];
    selectedSet = id;
    await tick();
    document.getElementById(`loop-tab-${id ?? "overview"}`)?.focus();
  }
  async function closeTab(id: string) {
    if (busy) return;
    if (id === "draft" && dirty) { discardDialog.showModal(); return; }
    openTabs = openTabs.filter(value => value !== id);
    history = history.filter(value => value !== id);
    if (selectedSet === id) {
      selectedSet = history.reverse().find(value => value === null || openTabs.includes(value)) ?? null;
      history.reverse();
    }
    await tick();
    document.getElementById(`loop-tab-${selectedSet ?? "overview"}`)?.focus();
  }
  async function discardDraft() {
    discard();
    dirty = false;
    discardDialog.close();
    await closeTab("draft");
  }
  function navigateTabs(event: KeyboardEvent) {
    if (!["ArrowLeft", "ArrowRight", "Home", "End"].includes(event.key)) return;
    const index = tabs.findIndex(tab => tab.id === selectedSet);
    const next = event.key === "Home" ? 0 : event.key === "End" ? tabs.length - 1 : (index + (event.key === "ArrowRight" ? 1 : -1) + tabs.length) % tabs.length;
    event.preventDefault();
    void selectTab(tabs[next].id);
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
      openTabs = openTabs.map(id => id === "draft" ? set.id : id);
      history = history.map(id => id === "draft" ? set.id : id);
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

<div class="flex h-full min-h-0 overflow-x-auto">
<StudioSidebar title="Loop">
  <div class="px-3 pb-3">
    <button class="{buttonClass} w-full bg-raised hover:bg-selection" aria-current={selectedSet === "draft" ? "page" : undefined} disabled={busy} onclick={() => selectTab("draft")}>{dirty ? "Continue draft · Unsaved" : "Create learning set"}</button>
  </div>
  <div class="flex items-center justify-between px-4 py-2 text-xs text-muted"><span>Learning sets ({sets.length})</span><button class="{buttonClass} bg-raised hover:bg-selection" disabled={busy} onclick={() => work(refresh)}><ActionLabel active={activeOperation === "Loading learning sets"} label="Refresh" pending="Refreshing…" /></button></div>
  <div class="min-h-0 flex-1 overflow-y-auto">
    {#each sets as set (set.id)}
      <button class="w-full px-4 py-3 text-left text-sm hover:bg-hover disabled:opacity-45 {focusClass} {selectedSet === set.id ? 'bg-selection' : 'bg-transparent'}" aria-current={selectedSet === set.id ? "page" : undefined} disabled={busy} onclick={() => selectTab(set.id)}>
        <span class="block truncate">{set.name}</span><span class="mt-1 block text-xs text-muted">{set.members.length} experiences</span>
      </button>
    {:else}<p class="px-4 py-3 text-xs text-muted">{loaded ? "No saved learning sets yet." : "Loading learning sets…"}</p>{/each}
  </div>
</StudioSidebar>
<div class="flex min-h-0 min-w-80 flex-1 flex-col">
  <div class="flex shrink-0 items-center border-b border-border bg-surface">
    <div role="tablist" aria-label="Loop tabs" tabindex="-1" class="flex min-w-0 overflow-x-auto" onkeydown={navigateTabs}>
      {#each tabs as tab (tab.id)}
        <div role="presentation" class="flex h-10 max-w-64 shrink-0 items-center border-r border-border {selectedSet === tab.id ? 'bg-background text-title' : 'text-muted hover:bg-hover'}">
          <button role="tab" id="loop-tab-{tab.id ?? 'overview'}" aria-controls="loop-panel-learning-sets" aria-selected={selectedSet === tab.id} tabindex={selectedSet === tab.id ? 0 : -1} class="flex h-full min-w-0 items-center gap-2 px-4 text-xs focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-title/40" onclick={() => selectTab(tab.id)} title={tab.label}>
            {#if tab.id === null}<LayoutDashboard size={14} class="shrink-0" aria-hidden="true" />{:else}<Layers size={14} class="shrink-0" aria-hidden="true" />{/if}
            <span class="truncate">{tab.label}</span>
            {#if tab.id === "draft" && dirty}<span aria-label="Unsaved draft" class="h-1.5 w-1.5 shrink-0 rounded-full bg-accent"></span>{/if}
          </button>
          {#if tab.id !== null}<button class="mr-2 rounded p-1 text-muted hover:bg-hover disabled:opacity-45 {focusClass}" aria-label="Close {tab.label} tab" disabled={busy} onclick={() => closeTab(tab.id!)}><X size={14} aria-hidden="true" /></button>{/if}
        </div>
      {/each}
    </div>
  </div>
<div id="loop-panel-learning-sets" role="tabpanel" aria-labelledby="loop-tab-{selectedSet ?? 'overview'}" tabindex="0" class="min-h-0 flex-1 overflow-y-auto px-7 py-6" aria-busy={busy}>
  {#if blocked}<p class="mb-4 rounded border border-warning/40 p-3 text-warning">Save or discard your experience review before curating. Your learning-set draft is preserved.</p>{/if}
  {#if selectedSet === null}
    <h2 class="text-xl font-medium">Overview</h2>
    <p class="mt-3 max-w-lg leading-relaxed text-muted">Select useful experiences, preserve their evidence, and prepare training datasets.</p>
    <p class="mt-5 text-sm">{loaded ? `${sets.length} saved learning sets` : "Loading learning sets…"}</p>
    {#if loaded && !sets.length}<p class="mt-2 text-sm text-muted">No learning sets yet. Create your first set from reviewed experiences.</p>{/if}
    <button class="{buttonClass} mt-4 bg-raised hover:bg-selection" disabled={busy || blocked} onclick={() => selectTab("draft")}>{dirty ? "Continue draft" : "Create learning set"}</button>
    <div class="mt-6 space-y-2">
      {#each sets as set (set.id)}
        <button class="block w-full rounded border border-border p-4 text-left hover:bg-hover {focusClass}" onclick={() => selectTab(set.id)}>
          <span class="block font-medium">{set.name}</span><span class="mt-1 block text-xs text-muted">{set.members.length} experiences · {set.curator}</span>
        </button>
      {/each}
    </div>
  {/if}
  {#if error}<p class="mt-4 text-error" role="alert">{error}</p>{/if}
  <div hidden={selectedSet !== "draft"}>
  <h2 class="text-xl font-medium">Select useful experience</h2>
  <p class="mt-2 max-w-3xl text-muted">Choose reviewed attempts that address a learning goal. A success can provide an example; a failure can reveal what needs improvement. Assistance alone does not identify a corrected trajectory.</p>

  <form class="mt-5" onsubmit={(e) => { e.preventDefault(); save(); }}>
    <fieldset class="disabled:opacity-50" disabled={busy || blocked}>
      <div class="grid gap-4 md:grid-cols-2">
        <label>Learning set name<input class={inputClass} bind:value={name} required maxlength="200" placeholder="Recover from grasp slips" /></label>
        <label>Curator<input class={inputClass} bind:value={curator} required maxlength="200" placeholder="Your name" /></label>
        <label class="md:col-span-2">Learning goal<textarea class={inputClass} bind:value={goal} required maxlength="4000" rows="2" placeholder="What should the robot learn from these attempts?"></textarea></label>
      </div>
      <div class="my-5 flex flex-wrap items-end gap-3">
        <label class="flex-1">Find reviewed experiences<input class={inputClass} bind:value={query} placeholder="Search task, robot, or ID" /></label>
        <label>Evidence filter<select class={inputClass} bind:value={filter}><option value="all">All reviewed</option><option value="success">Success</option><option value="assisted">Human assisted</option><option value="failure">Failure</option><option value="timeout">Timeout</option><option value="aborted">Aborted</option><option value="unknown">Unknown outcome</option></select></label>
        <span class="pb-2 text-xs text-muted">{candidates.length} matching · {selectionIds.length} selected</span>
      </div>
      <div class="max-h-72 overflow-y-auto rounded border border-border-strong">
        {#each candidates as experience (experience.id)}
          <label class="flex items-start gap-3 border-b border-border p-3">
            <input class="mt-1 accent-accent {focusClass}" type="checkbox" checked={!!selected[experience.id]} onchange={() => toggle(experience.id)} />
            <span class="min-w-0"><span class="block">{experience.task} · {experience.robot}</span><span class="block break-all text-xs text-muted">{experience.id} · {latest(experience.id)?.review.outcome} · Assistance: {latest(experience.id)?.review.assistance}</span><span class="block whitespace-pre-wrap text-xs text-muted">{latest(experience.id)?.review.notes}</span></span>
          </label>
        {:else}<p class="p-4 text-muted">No reviewed experiences match. Review attempts in Experiences first, or change the filter.</p>{/each}
      </div>
      {#if selectionIds.length}
        <h3 class="mt-5 font-medium">Selection rationale</h3>
        <p class="mt-1 text-xs text-muted">Selections remain when filters change. Include only evidence relevant to your goal.</p>
        {#each selectionIds as id (id)}
          <div class="mt-3 rounded border border-border-strong p-3">
            <div class="mb-2 flex items-center justify-between gap-3"><span class="break-all text-xs">{id} · review revision {selected[id].review_revision}</span><button class="{buttonClass} bg-raised hover:bg-selection" type="button" onclick={() => toggle(id)}>Remove</button></div>
            <label>Why include this attempt?<textarea class={inputClass} bind:value={selected[id].reason} required maxlength="4000" rows="2" placeholder="Useful example, failure evidence, or recovery worth studying"></textarea></label>
          </div>
        {/each}
      {/if}
      <div class="mt-4 flex gap-3"><button class="{buttonClass} bg-accent font-semibold text-white hover:bg-accent-hover" type="submit" disabled={!selectionIds.length}><ActionLabel active={activeOperation === "Saving learning set"} label="Save learning set" pending="Saving…" /></button>{#if dirty}<button class="{buttonClass} bg-raised hover:bg-selection" type="button" onclick={discard}>Discard selection</button>{/if}</div>
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
        <button class="{buttonClass} bg-raised hover:bg-selection" disabled={busy || blocked} onclick={() => preparingSet = preparingSet === set.id ? "" : set.id}>Prepare training dataset</button>
        <button class="{buttonClass} bg-accent font-semibold text-white hover:bg-accent-hover" disabled={busy || blocked} onclick={() => exportSet(set.id, true)}><ActionLabel active={activeOperation === "Exporting learning-set files"} label="Export experience files" pending="Exporting…" /></button>
        <button class="{buttonClass} bg-raised hover:bg-selection" disabled={busy || blocked} onclick={() => exportSet(set.id)}><ActionLabel active={activeOperation === "Exporting selection manifest"} label="Export selection manifest only" pending="Exporting…" /></button>
      </div>
      {#if preparingSet === set.id}<fieldset disabled={blocked}><DatasetPreparation {root} {set} bind:busy /></fieldset>{/if}
      <p class="mt-2 text-xs text-muted">Experience export includes original files and review history. Shared video/data chunks remain whole; dataset preparation is still required before training.</p>
    </section>
  {/each}
</div>
</div>
</div>

<dialog bind:this={discardDialog} aria-labelledby="discard-learning-set-title" class="m-auto max-w-md rounded-xl border border-border-strong bg-background p-6 text-foreground shadow-xl backdrop:bg-black/60" onclose={() => document.getElementById(`loop-tab-${selectedSet ?? "overview"}`)?.focus()}>
  <h2 id="discard-learning-set-title" class="text-lg font-medium">Discard this draft?</h2>
  <p class="mt-3 text-sm text-muted">The learning set details and experience selections have not been saved.</p>
  <div class="mt-5 flex justify-end gap-3">
    <button class="{buttonClass} bg-raised" onclick={() => discardDialog.close()}>Keep editing</button>
    <button class="{buttonClass} bg-raised text-error" onclick={discardDraft}>Discard draft</button>
  </div>
</dialog>
