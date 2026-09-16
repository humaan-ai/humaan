<script lang="ts">
  import FolderOpen from "@lucide/svelte/icons/folder-open";
  import { readRerunPath } from "$lib/tool-settings";
  import VerificationStatus from "./VerificationStatus.svelte";
  import type { VerificationReport } from "$lib/verification";
  import RecordsTable from "./RecordsTable.svelte";
  import ExperienceTabs from "./ExperienceTabs.svelte";
  import ExperienceOverview from "./ExperienceOverview.svelte";
  import ExperienceStatus from "./ExperienceStatus.svelte";
  import Eye from "@lucide/svelte/icons/eye";
  import EyeOff from "@lucide/svelte/icons/eye-off";
  import ActionLabel from "./ActionLabel.svelte";
  import { feedback } from "$lib/feedback";
  import StudioSettings from "./StudioSettings.svelte";
  import StudioActivityBar from "./StudioActivityBar.svelte";
  import StudioSidebar from "./StudioSidebar.svelte";
  import LearningSets from "./LearningSets.svelte";
  import { studio, type LocalProject as Project } from "$lib/studio.svelte";
  import { invoke, isTauri } from "@tauri-apps/api/core";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { onMount, tick } from "svelte";
  import Welcome from "./Welcome.svelte";
  type Manifest = { verification?: VerificationReport; duration_s?: number | null; source_error?: string | null; id: string; robot: string; task: string; artifacts: { path: string; format: string; bytes: number; sha256: string }[] };
  type Source = { episode_index: number; frames: number; fps: number; duration_s: number; videos: Record<string, { from_timestamp: number; to_timestamp: number }>; features: Record<string, unknown> };
  type Inspection = { manifest: Manifest; source: Source | null; source_error: string | null; annotations: Annotations };
  type Review = { outcome: string; assistance: string; notes: string; reviewer: string; review_state: string };
  type Annotations = { revisions: { revision: number; saved_at_unix_ms: number; source: string; review: Review }[] };
  let reviews = $state<Record<string, Annotations>>({});
  let queue = $state("all");
  let curationBusy = $state(false);
  let curationDirty = $state(false);
  let reviewer = $state("");
  let draft = $state<Review>({ outcome: "unknown", assistance: "unknown", notes: "", reviewer: "", review_state: "reviewed" });
  let savedDraft = $state("");
  let reviewMessage = $state("");
  let reviewError = $state("");
  function latest(id: string) { return reviews[id]?.revisions.at(-1); }
  function resetDraft() {
    if (!inspection) return;
    const review = latest(inspection.manifest.id)?.review;
    draft = review ? { ...review } : { outcome: "unknown", assistance: "unknown", notes: "", reviewer, review_state: "reviewed" };
    savedDraft = JSON.stringify(draft);
  }
  async function loadInspection(id: string) {
    if (!project) return;
    reviewMessage = ""; reviewError = "";
    const next = await invoke<Inspection>("experience_inspect", { root: project.root, id });
    inspection = next;
    activeTab = "detail";
    reviews[id] = inspection.annotations;
    resetDraft();
    await tick(); document.getElementById("experience-tab-detail")?.focus();
  }
  async function saveReview(next: boolean) {
    if (busy) return;
    reviewMessage = ""; reviewError = "";
    const review = { ...draft, reviewer: draft.reviewer.trim() };
    if (!review.reviewer) { reviewError = "Enter your name in Reviewer to save this review."; return; }
    const bytes = new TextEncoder();
    if (bytes.encode(review.reviewer).length > 200 || bytes.encode(review.notes).length > 10000) {
      reviewError = "Reviewer must fit within 200 UTF-8 bytes and notes within 10,000 UTF-8 bytes.";
      return;
    }
    savingNext = next;
    await work("Saving review", async () => {
      if (!project || !inspection) return;
      const id = inspection.manifest.id;
      const ids = (reviewOrigin === "table" ? tableRecords : filtered).map(e => e.id);
      const nextId = ids[ids.indexOf(id) + 1] ?? ids.find(candidate => candidate !== id);
      reviews[id] = await invoke<Annotations>("experience_annotate", { root: project.root, id, expectedRevision: latest(id)?.revision ?? 0, review });
      reviewer = review.reviewer;
      resetDraft();
      reviewMessage = `Saved locally · revision ${latest(id)?.revision}`;
      if (next && nextId) feedback.publish({ title: "Review saved", kind: "success" });
      if (next && nextId) {
        try { await loadInspection(nextId); }
        catch (e) { reportError("Could not open the next experience", `Your review was saved. ${String(e)}`); }
      }
      else if (next) { inspection = null; activeTab = "records"; feedback.publish({ title: "Review queue complete", detail: "Your review was saved. No more experiences match this queue.", kind: "success" }); }
    });
  }
  let activeTab = $state<"overview" | "records" | "detail">("overview");
  let settingsOpen = $state(false);
  function selectView(view: "experiences" | "loop") {
    studio.activeView = view;
  }
  function openSettings() {
    settingsOpen = true;
  }
  async function closeExperience() {
    if (busy || dirty) return;
    inspection = null; activeTab = "records";
    await tick(); document.getElementById("experience-tab-records")?.focus();
  }
  function browseRecords(nextQueue: string) {
    if (!dirty && !busy && !curationBusy) { tableQueue = nextQueue; tableQuery = ""; verificationFilter = "all"; }
    activeTab = "records";
  }
  const project = $derived(studio.project);
  let experiences = $state<Manifest[]>([]);
  const mixedRobots = $derived(new Set(experiences.map(e => e.robot)).size > 1);
  const listIdentifiers = $derived.by(() => {
    const tasks = new Map<string, Manifest[]>();
    for (const experience of experiences) {
      const group = tasks.get(experience.task) ?? [];
      group.push(experience); tasks.set(experience.task, group);
    }
    const identifiers: Record<string, string> = {};
    for (const group of tasks.values()) {
      if (group.length < 2) continue;
      for (const experience of group) {
        let length = Math.min(8, experience.id.length);
        while (length < experience.id.length && group.some(other => other.id !== experience.id && other.id.slice(-length) === experience.id.slice(-length))) length++;
        identifiers[experience.id] = `${length < experience.id.length ? "…" : ""}${experience.id.slice(-length)}`;
      }
    }
    return identifiers;
  });
  let inspection = $state<Inspection | null>(null);
  const dirty = $derived(!!inspection && JSON.stringify(draft) !== savedDraft);
  let savingNext = $state(false);
  let verificationFilter = $state("all");
  let name = $state("");
  let query = $state("");
  let tableQuery = $state("");
  let tableQueue = $state("all");
  let reviewOrigin = $state<"sidebar" | "table">("sidebar");
  let busy = $state("");
  function reportError(title: string, detail: string) {
    feedback.publish({ title, detail, kind: "error", popup: true });
  }
  let loaded = $state(false);
  function matchesRecord(record: Manifest, reviewFilter: string, search: string) {
    const review = latest(record.id)?.review;
    const matchesReview = reviewFilter === "all" || (reviewFilter === "unreviewed" ? review?.review_state !== "reviewed" : reviewFilter === "reviewed" ? review?.review_state === "reviewed" : reviewFilter === "assisted" ? review?.assistance === "yes" : (review?.outcome ?? "unknown") === reviewFilter);
    return matchesReview && `${record.id} ${record.robot} ${record.task}`.toLowerCase().includes(search.trim().toLowerCase());
  }
  const filtered = $derived(experiences.filter(record => matchesRecord(record, queue, query)));
  const tableRecords = $derived(experiences.filter(record => matchesRecord(record, tableQueue, tableQuery) && (verificationFilter === "all" || (record.verification?.status ?? "not_checked") === verificationFilter)));
  onMount(() => {
    let disposed = false;
    const unlisteners: UnlistenFn[] = [];
    if (isTauri()) {
      for (const [event, action] of [["studio:open-project", () => { void open(); }], ["studio:new-project", showNewProject]] as const) {
        listen(event, action)
          .then((cleanup) => { if (disposed) cleanup(); else unlisteners.push(cleanup); })
          .catch((cause) => { if (!disposed) reportError("File menu unavailable", String(cause)); });
      }
    }
    return () => { disposed = true; unlisteners.forEach(cleanup => cleanup()); };
  });
  async function work(label: string, operation: () => Promise<void>) {
    if (busy) return;
    busy = label;
    const quiet = ["Opening project", "Creating project", "Loading experience metadata", "Refreshing experience metadata", "Saving review"].includes(label);
    const activity = quiet ? null : feedback.start(label);
    try { await operation(); }
    catch (e) { reportError(label === "Opening Rerun…" ? "Could not open Rerun" : `${label} failed`, String(e)); }
    finally { if (activity !== null) feedback.finish(activity); busy = ""; }
  }
  async function folder(title: string) { return invoke<string | null>("select_directory", { title }); }
  async function refresh() {
    if (!project) return;
    loaded = false; experiences = []; reviews = {};
    const catalog = await invoke<{ records: Manifest[]; warning: string | null }>("experience_list", { root: project.root });
    experiences = catalog.records;
    if (catalog.warning) feedback.publish({ title: "Verification history unavailable", detail: catalog.warning, kind: "warning", popup: true });
    reviews = await invoke<Record<string, Annotations>>("experience_reviews", { root: project.root });
    loaded = true;
  }
  let newProjectDialog: HTMLDialogElement;
  let projectLocation = $state("~/Humaan");
  let choosingLocation = $state(false);
  async function browseProjectLocation() {
    if (choosingLocation || busy) return;
    choosingLocation = true;
    try {
      const path = await folder("Choose project location");
      if (path) projectLocation = path;
    } catch (cause) { reportError("Could not choose project location", String(cause)); }
    finally { choosingLocation = false; }
  }
  function showNewProject() {
    if (busy || curationBusy || dirty || curationDirty) {
      reportError("Cannot create a project yet", "Finish running operations and save or discard your review and learning-set drafts first.");
      return;
    }
    if (newProjectDialog.open) return;
    name = "";
    projectLocation = "~/Humaan";
    newProjectDialog.showModal();
  }
  async function createFromMenu() {
    if (!name.trim() || !projectLocation.trim() || choosingLocation) return;
    const previousProject = studio.project;
    await open(true);
    if (studio.project !== previousProject) newProjectDialog.close();
  }
  async function open(create = false) {
    if (busy || curationBusy) {
      feedback.publish({ title: "An operation is still running", detail: "Wait for it to finish before opening another project.", kind: "warning", popup: true });
      return;
    }
    if (dirty || curationDirty) {
      feedback.publish({ title: "Unsaved drafts", detail: "Save or discard your review and learning-set drafts before opening another project.", kind: "warning", popup: true });
      return;
    }
    await work(create ? "Creating project" : "Opening project", async () => {
      const path = create ? projectLocation.trim() : await folder("Open Humaan project"); if (!path) return;
      const next = await invoke<Project>("experience_project", { path, name: create ? name.trim() : null });
      feedback.reset();
      activeTab = "overview"; queue = "all"; tableQueue = "all"; tableQuery = ""; reviewOrigin = "sidebar";
      studio.project = next; studio.activeView = "experiences"; inspection = null; verificationFilter = "all"; query = "";
      await refresh();
    });
  }
  async function importRecording() {
    await work("Importing and verifying recordings", async () => {
      const source = await folder("Import recording or LeRobot dataset"); if (!source || !project) return;
      const imported = await invoke<Manifest[]>("experience_import", { root: project.root, source });
      verificationFilter = "all";
      try { await refresh(); } catch (cause) { throw new Error(`Recordings were imported, but the catalog could not refresh. Use Refresh to try again. ${String(cause)}`); }
      feedback.publish({ title: `Imported ${imported.length} experience${imported.length === 1 ? "" : "s"}`, kind: "success", popup: true, detail: "Recordings are available in Experiences. Clear the review filter if they are not visible." });
    });
  }
  async function inspect(id: string, origin: "sidebar" | "table" = "sidebar") {
    if (inspection?.manifest.id === id) { activeTab = "detail"; return; }
    if (dirty || busy || curationBusy) return;
    await work("Loading experience metadata", async () => {
      if (!project) return;
      await loadInspection(id);
      reviewOrigin = origin;
    });
  }
  async function verify() {
    await work("Verifying all recordings", async () => {
      if (!project) return;
      const results = await invoke<VerificationReport[]>("experience_verify", { root: project.root });
      for (const result of results) {
        const record = experiences.find(record => record.id === result.id);
        if (record) record.verification = result;
        else experiences.push({ id: result.id, task: "Unreadable recording", robot: "Unknown", artifacts: [], verification: result });
      }
      const failed = results.filter(result => result.status === "failed").length;
      const changed = results.filter(result => result.status === "needs_recheck").length;
      const passed = results.filter(result => result.status === "verified").length;
      feedback.publish({
        title: !results.length ? "No records to verify" : failed || changed ? `Verification complete · ${passed} passed, ${failed} failed${changed ? `, ${changed} need recheck` : ""}` : `All ${passed} records verified`,
        detail: "Results and check times are saved locally. Use the info icon in the Verification column for details.",
        kind: failed || changed ? "warning" : "success", popup: true,
        action: failed || changed ? { label: failed ? "Show failed" : "Show changed", run: async () => {
          studio.activeView = "experiences"; activeTab = "records";
          tableQueue = "all"; tableQuery = ""; verificationFilter = failed ? "failed" : "needs_recheck";
          feedback.closePanel(); await tick(); document.getElementById("verification-filter")?.focus();
        } } : undefined,
      });
    });
  }
  async function exportRecording() {
    await work("Exporting and verifying experience", async () => {
      const parent = await folder("Choose parent folder for export"); if (!parent || !project || !inspection) return;
      const destination = await invoke<string>("experience_export", { root: project.root, id: inspection.manifest.id, parent });
      feedback.publish({ title: "Experience exported", detail: destination, copyText: destination, kind: "success", popup: true });
    });
  }
  async function replay() {
    await work("Opening Rerun…", async () => {
      if (!project || !inspection?.source) return;
      await invoke("experience_replay", { root: project.root, id: inspection.manifest.id, viewerPath: readRerunPath() || null });
      feedback.publish({ title: "Opened in Rerun", detail: "Continue in the separate viewer.", kind: "info", popup: true });
    });
  }
</script>

<StudioSettings bind:open={settingsOpen} />

<div class="flex h-full min-w-0">
  {#if project}<StudioActivityBar busy={!!busy || curationBusy} reviewDirty={dirty} loopDirty={curationDirty} settingsActive={settingsOpen} onsettings={openSettings} onview={selectView} />{/if}
<section class="flex min-w-0 flex-1 flex-col bg-background text-sm" aria-label="Studio workspace" aria-busy={!!busy || curationBusy}>
  {#if !project}
    <Welcome busy={!!busy} onopen={() => open()} oncreate={showNewProject} />
  {:else}
    {#key project.root}
      <div class="min-h-0 flex-1 border-t border-border-strong" hidden={studio.activeView !== "loop"}>
        <LearningSets root={project.root} {experiences} {reviews} bind:busy={curationBusy} bind:dirty={curationDirty} blocked={dirty} />
      </div>
    {/key}
    <div class="min-h-0 flex-1 border-t border-border-strong" hidden={studio.activeView !== "experiences"}>
      <div class="flex h-full min-h-0 overflow-x-auto">
      <StudioSidebar title="Experiences">
        <div class="p-4 flex flex-col gap-3">
          <select aria-label="Review queue" bind:value={queue} disabled={!!busy || dirty || curationBusy}>
            <option value="all">All experiences</option><option value="unreviewed">Unreviewed</option><option value="reviewed">Reviewed</option><option value="unknown">Unknown outcome</option><option value="failure">Failure</option><option value="timeout">Timeout</option><option value="aborted">Aborted</option><option value="success">Success</option><option value="assisted">Human assisted</option>
          </select>
          <span class="text-xs text-muted">{filtered.length} in queue · {experiences.filter(e => latest(e.id)?.review.review_state === "reviewed").length} reviewed</span>
          <input class="w-full" aria-label="Search experiences" placeholder="Search task, robot, or ID" bind:value={query} disabled={!!busy || dirty || curationBusy} /></div>
        <div class="min-h-0 flex-1 overflow-y-auto">
          {#each filtered as experience (experience.id)}
            <button class="entry {inspection?.manifest.id === experience.id ? 'selected' : ''}" disabled={!!busy || curationBusy || (dirty && inspection?.manifest.id !== experience.id)} onclick={() => inspect(experience.id)}>
              <span class="block font-medium text-title">{experience.task}</span>
              {#if mixedRobots || listIdentifiers[experience.id]}
                <span class="mt-1 flex flex-wrap items-center gap-1.5 text-xs text-muted">
                  {#if mixedRobots}<span>{experience.robot}</span>{/if}
                  {#if mixedRobots && listIdentifiers[experience.id]}<span aria-hidden="true">·</span>{/if}
                  {#if listIdentifiers[experience.id]}<span class="break-all font-mono" title={experience.id}>{listIdentifiers[experience.id]}</span>{/if}
                </span>
              {/if}
              <span class="mt-2 flex flex-wrap items-center gap-1.5 text-xs">
                <ExperienceStatus kind="outcome" value={latest(experience.id)?.review.outcome} />
                <span aria-hidden="true">·</span>
                <span class="inline-flex items-center gap-1 {latest(experience.id)?.review.review_state === 'reviewed' ? 'text-success' : 'text-muted'}">
                  {#if latest(experience.id)?.review.review_state === "reviewed"}
                    <Eye size={13} class="shrink-0" aria-hidden="true" />
                  {:else}
                    <EyeOff size={13} class="shrink-0" aria-hidden="true" />
                  {/if}
                  {latest(experience.id)?.review.review_state ?? "unreviewed"}
                </span>
              </span>
              {#if latest(experience.id)?.review.assistance === "yes"}
                <span class="mt-1 block text-xs"><ExperienceStatus kind="assistance" value="yes" /></span>
              {/if}
            </button>
          {:else}<p class="p-5 text-muted">{!loaded ? "Refresh to load recordings. Use Verify all to diagnose damaged entries." : experiences.length ? "No matching experiences." : "No experiences yet. Import a complete LeRobot v3.0 dataset or a Humaan recording folder."}</p>{/each}
        </div>
      </StudioSidebar>
      <div class="flex min-h-0 min-w-80 flex-1 flex-col">
        <ExperienceTabs active={activeTab} detail={inspection?.manifest.task} {dirty} busy={!!busy || curationBusy} onselect={(tab) => activeTab = tab} onclose={closeExperience} />
        <div id="experience-panel-overview" role="tabpanel" aria-labelledby="experience-tab-overview" tabindex="0" class="min-h-0 flex-1 overflow-y-auto" hidden={activeTab !== "overview"}>
          <ExperienceOverview name={project.project.name} {experiences} {reviews} {loaded} onrecords={browseRecords} />
        </div>
        <div id="experience-panel-records" role="tabpanel" aria-labelledby="experience-tab-records" tabindex="0" class="min-h-0 flex-1" hidden={activeTab !== "records"}>
          <div class="flex h-full min-h-0 flex-col">
    <div class="flex flex-wrap items-center gap-3 border-b border-border px-4 py-3">
      <button class="text-white primary" disabled={!!busy || dirty || curationBusy || curationDirty} onclick={importRecording}><ActionLabel active={busy === "Importing and verifying recordings"} label="Import recordings" pending="Importing…" /></button>
      <button disabled={!!busy || dirty || curationBusy || curationDirty} onclick={() => work("Refreshing experience metadata", async () => { inspection = null; verificationFilter = "all"; await refresh(); })}><ActionLabel active={busy === "Refreshing experience metadata"} label="Refresh" pending="Refreshing…" /></button>
      <button disabled={!!busy || dirty || curationBusy || curationDirty} onclick={verify}><ActionLabel active={busy === "Verifying all recordings"} label="Verify all" pending="Verifying…" /></button>
      <label class="flex items-center gap-2 text-xs text-muted">Verification<select aria-label="Verification status filter" id="verification-filter" bind:value={verificationFilter}><option value="all">All statuses</option><option value="verified">Verified</option><option value="failed">Failed</option><option value="needs_recheck">Needs recheck</option><option value="not_checked">Not checked</option></select></label>
      <span class="ml-auto text-xs text-muted">{loaded ? `${tableRecords.length} of ${experiences.length} experiences` : "Catalog not loaded"}</span>
    </div>

            <div class="flex flex-wrap items-center gap-3 border-b border-border px-4 py-3">
              <input class="min-w-48 flex-1" aria-label="Search records table" placeholder="Search records by task, robot, or ID" bind:value={tableQuery} />
              <select aria-label="Records table review filter" bind:value={tableQueue}><option value="all">All experiences</option><option value="unreviewed">Unreviewed</option><option value="reviewed">Reviewed</option><option value="unknown">Unknown outcome</option><option value="failure">Failure</option><option value="timeout">Timeout</option><option value="aborted">Aborted</option><option value="success">Success</option><option value="assisted">Human assisted</option></select>
              {#if tableQuery || tableQueue !== "all" || verificationFilter !== "all"}<button onclick={() => { tableQuery = ""; tableQueue = "all"; verificationFilter = "all"; }}>Clear table filters</button>{/if}
            </div>
            {#if dirty}<div class="flex items-center gap-3 border-b border-border px-4 py-2 text-xs text-warning"><span>Save or discard your open review before opening another record.</span><button onclick={() => activeTab = "detail"}>Return to review</button></div>{/if}
            <div class="flex min-h-0 flex-1 overflow-x-auto">

              <div class="min-w-80 flex-1 overflow-y-auto px-7 py-6">
                <RecordsTable records={tableRecords} {reviews} {mixedRobots} selectedId={inspection?.manifest.id} busy={!!busy || curationBusy} {dirty} {loaded} onopen={(id) => inspect(id, "table")} />
              </div>
            </div>
          </div>
        </div>
        {#if inspection}
          <div id="experience-panel-detail" role="tabpanel" aria-labelledby="experience-tab-detail" tabindex="0" class="min-h-0 flex-1 overflow-y-auto px-7 py-6" hidden={activeTab !== "detail"}>
        {#if inspection}
          <div class="mb-5 flex items-start justify-between gap-4"><div><p class="text-xs uppercase tracking-widest text-muted">Experience</p><h2 class="mt-2 text-xl font-medium">{inspection.manifest.task}</h2></div><button disabled={!!busy || dirty || curationBusy || curationDirty} onclick={exportRecording}><ActionLabel active={busy === "Exporting and verifying experience"} label="Export" pending="Exporting…" /></button></div>
          <p class="mb-5 break-all font-mono text-xs text-muted">{inspection.manifest.id}</p>
          {#if inspection.source_error}<p class="mb-4 text-error" role="alert">Source metadata could not be loaded: {inspection.source_error}</p>{/if}
          {#if inspection.source}
            <button class="text-white primary mb-5" disabled={!!busy} onclick={replay}><ActionLabel active={busy === "Opening Rerun…"} label="Open in Rerun ↗" pending="Opening…" /></button>
          {/if}
          <dl class="grid grid-cols-2 gap-4 rounded-lg border border-border-strong bg-surface p-5">
            <div><dt>Robot</dt><dd>{inspection.manifest.robot}</dd></div><div><dt>Integrity</dt><dd><VerificationStatus id={inspection.manifest.id} report={experiences.find(record => record.id === inspection?.manifest.id)?.verification} /></dd></div>
            <div><dt>Outcome</dt><dd><ExperienceStatus kind="outcome" value={latest(inspection.manifest.id)?.review.outcome} /></dd></div><div><dt>Human assistance</dt><dd><ExperienceStatus kind="assistance" value={latest(inspection.manifest.id)?.review.assistance} /></dd></div>
            <div><dt>Policy</dt><dd>Unknown</dd></div>
            {#if inspection.source}<div><dt>Source</dt><dd>LeRobot v3.0 · episode {inspection.source.episode_index}</dd></div><div><dt>Duration</dt><dd>{inspection.source.duration_s.toFixed(2)}s · {inspection.source.frames} frames at {inspection.source.fps} FPS</dd></div>{/if}
          </dl>
          <form class="mt-5 rounded-lg border border-border-strong bg-surface p-5" onsubmit={(event) => { event.preventDefault(); saveReview(event.submitter instanceof HTMLButtonElement && event.submitter.value === "next"); }}>
            <h3 class="font-medium">Review outcome</h3>
            <p class="my-3 text-xs text-muted">Judge against the task’s success criteria. Keep the outcome unknown when evidence is insufficient. Assistance is separate from success.</p>
            <fieldset disabled={!!busy} class="grid grid-cols-2 gap-4">
              <label>Final outcome<select class="mt-1 w-full" bind:value={draft.outcome}><option value="unknown">Unknown</option><option value="success">Success</option><option value="failure">Failure</option><option value="timeout">Timeout</option><option value="aborted">Aborted</option></select></label>
              <label>Human assistance<select class="mt-1 w-full" bind:value={draft.assistance}><option value="unknown">Unknown</option><option value="yes">Yes</option><option value="no">No</option></select></label>
              <label>Reviewer<input class="mt-1 w-full" bind:value={draft.reviewer} required maxlength="200" placeholder="Your name" aria-describedby="reviewer-help" /></label>
              <label>Review state<select class="mt-1 w-full" bind:value={draft.review_state}><option value="reviewed">Reviewed</option><option value="unreviewed">Unreviewed</option></select></label>
              <label class="col-span-2">Notes<textarea class="mt-1 w-full" rows="3" bind:value={draft.notes} maxlength="10000" placeholder="Evidence, failure reason, or uncertainty"></textarea></label>
            </fieldset>
            <p id="reviewer-help" class="mt-3 text-xs text-muted">A reviewer name is required to save. It identifies who assigned this outcome.</p>
            {#if reviewError}<p class="mt-3 text-error" role="alert">{reviewError}</p>{/if}
            {#if reviewMessage && !dirty}<p class="mt-3 text-success" role="status">{reviewMessage}</p>{/if}
            <div class="mt-4 flex flex-wrap gap-2">
              <button class="text-white primary" type="submit" value="save" disabled={!!busy}><ActionLabel active={busy === "Saving review" && !savingNext} label="Save review" pending="Saving…" /></button>
              <button type="submit" value="next" disabled={!!busy}><ActionLabel active={busy === "Saving review" && savingNext} label="Save and next" pending="Saving…" /></button>
              {#if dirty}<button type="button" disabled={!!busy} onclick={resetDraft}>Discard changes</button><span class="self-center text-xs text-muted">Unsaved changes</span>{/if}
            </div>
            {#if latest(inspection.manifest.id)}
              <details class="mt-4 text-xs text-muted"><summary>Review history ({reviews[inspection.manifest.id].revisions.length})</summary>
                {#each reviews[inspection.manifest.id].revisions.toReversed() as revision}
                  <div class="mt-3 border-t border-border pt-3"><p>Revision {revision.revision} · {revision.review.reviewer} · {new Date(revision.saved_at_unix_ms).toLocaleString()} · {revision.source}</p><p>{revision.review.outcome} · Assistance: {revision.review.assistance} · {revision.review.review_state}</p><p class="whitespace-pre-wrap break-words">{revision.review.notes}</p></div>
                {/each}
              </details>
            {/if}
          </form>
          {#if inspection.source}
            <h3 class="mb-3 mt-6 font-medium">Cameras</h3>
            {#each Object.entries(inspection.source.videos) as [camera, video]}<div class="mb-2 rounded border border-border-strong p-3"><p class="break-all">{camera}</p><p class="mt-1 text-xs text-muted">Episode range: {video.from_timestamp.toFixed(2)}–{video.to_timestamp.toFixed(2)}s in the source video</p></div>{/each}
            <details class="my-5"><summary>Observation and action schema</summary><pre class="mt-3 overflow-auto rounded bg-surface p-4 text-xs">{JSON.stringify(inspection.source.features, null, 2)}</pre></details>
          {/if}
          <h3 class="mb-3 mt-6 font-medium">Artifacts</h3>
          {#each inspection.manifest.artifacts as artifact}<details class="mb-2 rounded border border-border-strong p-3"><summary class="break-all">{artifact.path} <span class="text-xs text-muted">· {artifact.bytes.toLocaleString()} bytes</span></summary><p class="mt-2 break-all text-xs text-muted">{artifact.format}</p><p class="mt-2 break-all font-mono text-xs text-muted">SHA-256 {artifact.sha256}</p></details>{/each}
          <p class="mt-6 text-xs leading-relaxed text-muted">Selecting an experience reads metadata only. Integrity results are saved from the last explicit check; use Verify all to check again. LeRobot replay opens the selected episode in the standalone Rerun viewer.</p>
        {:else}<div class="flex min-h-48 flex-col items-center justify-center gap-3 text-center"><h2 class="text-lg">Inspect an experience</h2><p class="max-w-sm leading-relaxed text-muted">Select an attempt to view its metadata and review. Use Verify all to check recording files. Outcomes stay unknown until reviewed.</p></div>{/if}
          </div>
        {/if}
      </div>
      </div>
    </div>
  {/if}
</section>
</div>
<style>
  button { display: inline-flex; align-items: center; justify-content: center; gap: 0.5rem; border-radius: 6px; padding: 0.5rem 0.85rem; font-size: 0.8rem; background: var(--color-raised); border: 1px solid var(--color-border-strong); }
  .primary { background: var(--color-accent); font-weight: 600; }
  .entry { display: block; width: 100%; border-radius: 0; border: 0; border-bottom: 1px solid var(--color-border); padding: 1rem; text-align: left; background: transparent; }
  .entry:hover, .entry.selected { background: var(--color-selection); }
  button:disabled { opacity: 0.45; cursor: default; }
  input, select, textarea { border: 1px solid var(--color-border-strong); border-radius: 5px; background: var(--color-raised); padding: 0.6rem; font-size: 0.8rem; }
  input:focus-visible, select:focus-visible, textarea:focus-visible, button:focus-visible, summary:focus-visible { outline: 2px solid var(--color-accent); outline-offset: 2px; }
  summary { cursor: pointer; }
  dt { color: var(--color-muted); font-size: 0.75rem; margin-bottom: 0.3rem; }
  dd { overflow-wrap: anywhere; }
</style>

<dialog bind:this={newProjectDialog} aria-labelledby="new-project-title" class="m-auto w-[calc(100%_-_2rem)] max-w-md rounded-xl border border-border-strong bg-background p-6 text-foreground shadow-xl backdrop:bg-black/60" oncancel={(event) => { if (busy || choosingLocation) event.preventDefault(); }}>
  <h2 id="new-project-title" class="text-lg font-medium">New Project</h2>
  <form class="mt-5" onsubmit={(event) => { event.preventDefault(); void createFromMenu(); }}>
    <label for="new-project-name" class="block text-sm">Project name</label>
    <div class="relative mt-2">
      <input id="new-project-name" class="w-full !pr-12" bind:value={name} placeholder="Pick and place" required disabled={!!busy} />
      <button type="button" class="!absolute right-1 top-1/2 -translate-y-1/2 !border-0 !bg-transparent !p-2 text-muted hover:text-foreground" aria-label="Browse project location" title="Browse project location" disabled={!!busy || choosingLocation} onclick={browseProjectLocation}><FolderOpen size={16} aria-hidden="true" /></button>
    </div>
    <div class="mt-5 flex justify-end gap-3">
      <button type="button" disabled={!!busy || choosingLocation} onclick={() => newProjectDialog.close()}>Cancel</button>
      <button type="submit" class="primary" disabled={!!busy || choosingLocation || !name.trim() || !projectLocation.trim()}><ActionLabel active={busy === "Creating project"} label="Create new project" pending="Creating…" /></button>
    </div>
  </form>
</dialog>
