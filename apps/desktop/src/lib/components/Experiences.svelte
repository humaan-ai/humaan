<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";
  import LoaderCircle from "@lucide/svelte/icons/loader-circle";
  type Manifest = { id: string; robot: string; task: string; artifacts: { path: string; format: string; bytes: number; sha256: string }[] };
  type Project = { root: string; project: { id: string; name: string } };
  type Source = { episode_index: number; frames: number; fps: number; duration_s: number; videos: Record<string, { from_timestamp: number; to_timestamp: number }>; features: Record<string, unknown> };
  type Inspection = { manifest: Manifest; source: Source | null };
  type Verification = { id: string; valid: boolean; error?: string };
  type Review = { outcome: string; assistance: string; notes: string; reviewer: string; review_state: string };
  type Annotations = { revisions: { revision: number; saved_at_unix_ms: number; source: string; review: Review }[] };
  let reviews = $state<Record<string, Annotations>>({});
  let queue = $state("unreviewed");
  let reviewer = $state("");
  let draft = $state<Review>({ outcome: "unknown", assistance: "unknown", notes: "", reviewer: "", review_state: "reviewed" });
  let savedDraft = $state("");
  function latest(id: string) { return reviews[id]?.revisions.at(-1); }
  function resetDraft() {
    if (!inspection) return;
    const review = latest(inspection.manifest.id)?.review;
    draft = review ? { ...review } : { outcome: "unknown", assistance: "unknown", notes: "", reviewer, review_state: "reviewed" };
    savedDraft = JSON.stringify(draft);
  }
  async function loadInspection(id: string) {
    if (!project) return;
    inspection = null;
    inspection = await invoke<Inspection>("experience_inspect", { root: project.root, id });
    resetDraft();
  }
  async function saveReview(next: boolean) {
    await work("Saving review", async () => {
      if (!project || !inspection) return;
      const id = inspection.manifest.id;
      const ids = filtered.map(e => e.id);
      const nextId = ids[ids.indexOf(id) + 1] ?? ids.find(candidate => candidate !== id);
      reviews[id] = await invoke<Annotations>("experience_annotate", { root: project.root, id, expectedRevision: latest(id)?.revision ?? 0, review: draft });
      reviewer = draft.reviewer;
      resetDraft();
      message = "Review saved.";
      if (next && nextId) await loadInspection(nextId);
      else if (next) { inspection = null; message = "Review saved. No more experiences in this queue."; }
    });
  }
  let project = $state<Project | null>(null);
  let experiences = $state<Manifest[]>([]);
  let inspection = $state<Inspection | null>(null);
  const dirty = $derived(!!inspection && JSON.stringify(draft) !== savedDraft);
  let verification = $state<Verification[] | null>(null);
  let name = $state("");
  let query = $state("");
  let busy = $state("");
  let error = $state("");
  let message = $state("");
  let elapsed = $state(0);
  let started = 0;
  let loaded = $state(false);
  const filtered = $derived(experiences.filter(e =>
    (queue === "all" || (queue === "unreviewed" ? latest(e.id)?.review.review_state !== "reviewed" : queue === "reviewed" ? latest(e.id)?.review.review_state === "reviewed" : queue === "assisted" ? latest(e.id)?.review.assistance === "yes" : (latest(e.id)?.review.outcome ?? "unknown") === queue)) && `${e.id} ${e.robot} ${e.task}`.toLowerCase().includes(query.toLowerCase())));
  onMount(() => {
    const timer = setInterval(() => { if (busy) elapsed = Math.floor((Date.now() - started) / 1000); }, 1000);
    return () => clearInterval(timer);
  });
  async function work(label: string, operation: () => Promise<void>) {
    if (busy) return;
    busy = label; error = ""; message = ""; started = Date.now(); elapsed = 0;
    try { await operation(); } catch (e) { error = String(e); } finally { busy = ""; }
  }
  async function folder(title: string) { return invoke<string | null>("select_directory", { title }); }
  async function refresh() {
    if (!project) return;
    loaded = false; experiences = []; reviews = {};
    experiences = await invoke<Manifest[]>("experience_list", { root: project.root });
    reviews = await invoke<Record<string, Annotations>>("experience_reviews", { root: project.root });
    loaded = true;
  }
  async function open(create = false) {
    await work(create ? "Creating project" : "Opening project", async () => {
      const path = await folder(create ? "Choose folder for new project" : "Open Humaan project"); if (!path) return;
      const next = await invoke<Project>("experience_project", { path, name: create ? name.trim() : null });
      project = next; inspection = null; verification = null; query = "";
      await refresh(); message = `${create ? "Created" : "Opened"} ${next.project.name}.`;
    });
  }
  async function importRecording() {
    await work("Importing and verifying recordings", async () => {
      const source = await folder("Import recording or LeRobot dataset"); if (!source || !project) return;
      const imported = await invoke<Manifest[]>("experience_import", { root: project.root, source });
      verification = null;
      message = `Successfully imported ${imported.length} experience${imported.length === 1 ? "" : "s"}.`;
      await refresh();
    });
  }
  async function inspect(id: string) {
    await work("Inspecting and verifying experience", async () => {
      if (!project) return;
      await loadInspection(id);
    });
  }
  async function verify() {
    await work("Verifying all recordings", async () => {
      if (!project) return;
      verification = await invoke<Verification[]>("experience_verify", { root: project.root });
      const failed = verification.filter(v => !v.valid).length;
      if (inspection && verification.some(v => v.id === inspection?.manifest.id && !v.valid)) inspection = null;
      message = `Checked ${verification.length} experiences; ${failed} failed.`;
    });
  }
  async function exportRecording() {
    await work("Exporting and verifying experience", async () => {
      const parent = await folder("Choose parent folder for export"); if (!parent || !project || !inspection) return;
      const destination = await invoke<string>("experience_export", { root: project.root, id: inspection.manifest.id, parent });
      message = `Exported to ${destination}.`;
    });
  }
  async function replay() {
    await work("Preparing selected experience for Rerun", async () => {
      if (!project || !inspection?.source) return;
      await invoke("experience_replay", { root: project.root, id: inspection.manifest.id });
      message = "Opened the selected experience in a separate Rerun window.";
    });
  }
</script>

<section class="flex h-full flex-col bg-background text-sm" aria-label="Experiences" aria-busy={!!busy}>
  <header class="flex flex-wrap items-center justify-between gap-4 border-b border-border-strong px-7 py-5">
    <div><p class="mb-1 text-xs uppercase tracking-widest text-muted">Local project</p><h1 class="text-xl font-semibold">{project?.project.name ?? "Experiences"}</h1>
      {#if project}<p class="mt-1 break-all text-xs text-muted">{project.root}</p>{/if}
    </div>
    <button disabled={!!busy || dirty} onclick={() => open()}>Open project</button>
  </header>
  {#if busy}<div class="flex items-center gap-2 border-b border-border px-7 py-3 text-accent" role="status"><LoaderCircle size={16} class="animate-spin" /> {busy}… <span class="text-muted">{elapsed}s elapsed</span></div>{/if}
  {#if error}<div class="mx-7 mt-4 rounded border border-error/40 bg-error/10 p-3 break-words" role="alert">{error}</div>{/if}
  {#if message}<p class="mx-7 mt-4 break-all text-success" role="status">{message}</p>{/if}
  {#if !project}
    <div class="mx-auto flex w-full max-w-lg flex-1 flex-col justify-center gap-5 px-8 py-10">
      <h2 class="text-2xl font-medium">Your robot’s experience, in one place.</h2>
      <p class="leading-relaxed text-muted">Open a project created with the CLI, or create one in a local folder. Import LeRobot recordings to inspect their tasks, cameras, and artifacts. No account required.</p>
      <form class="flex flex-col gap-3 rounded-lg border border-border-strong bg-surface p-5" onsubmit={(e) => { e.preventDefault(); open(true); }}>
        <label for="project-name">New project name</label><input id="project-name" placeholder="Pick and place" bind:value={name} disabled={!!busy || dirty} required />
        <button class="primary" disabled={!!busy || dirty || !name.trim()} type="submit">Choose folder and create project</button>
      </form>
    </div>
  {:else}
    <div class="flex flex-wrap items-center gap-3 px-7 py-4">
      <button class="primary" disabled={!!busy || dirty} onclick={importRecording}>Import recordings</button>
      <button disabled={!!busy || dirty} onclick={() => work("Refreshing experience metadata", async () => { inspection = null; verification = null; await refresh(); })}>Refresh</button>
      <button disabled={!!busy || dirty} onclick={verify}>Verify all</button>
      <span class="ml-auto text-xs text-muted">{loaded ? `${experiences.length} experiences` : "Catalog not loaded"}</span>
    </div>
    <div class="flex min-h-0 flex-1 flex-wrap border-t border-border-strong">
      <aside class="flex min-w-64 basis-80 flex-col border-r border-border-strong bg-surface">
        <div class="p-4 flex flex-col gap-3">
          <select aria-label="Review queue" bind:value={queue} disabled={!!busy || dirty}>
            <option value="unreviewed">Unreviewed</option><option value="all">All experiences</option><option value="reviewed">Reviewed</option><option value="unknown">Unknown outcome</option><option value="failure">Failure</option><option value="timeout">Timeout</option><option value="aborted">Aborted</option><option value="success">Success</option><option value="assisted">Human assisted</option>
          </select>
          <span class="text-xs text-muted">{filtered.length} in queue · {experiences.filter(e => latest(e.id)?.review.review_state === "reviewed").length} reviewed</span>
          <input class="w-full" aria-label="Search experiences" placeholder="Search task, robot, or ID" bind:value={query} disabled={!!busy || dirty} /></div>
        <div class="min-h-0 flex-1 overflow-y-auto">
          {#each filtered as experience (experience.id)}
            <button class="entry {inspection?.manifest.id === experience.id ? 'selected' : ''}" disabled={!!busy || dirty} onclick={() => inspect(experience.id)}>
              <span class="block font-medium">{experience.task}</span><span class="mt-1 block text-xs text-muted">{experience.robot}</span>
              <span class="mt-2 block text-xs">{latest(experience.id)?.review.outcome ?? "unknown"} · {latest(experience.id)?.review.review_state ?? "unreviewed"}</span>
              <span class="mt-2 block truncate font-mono text-xs text-muted" title={experience.id}>{experience.id}</span>
            </button>
          {:else}<p class="p-5 text-muted">{!loaded ? "Refresh to load recordings. Use Verify all to diagnose damaged entries." : experiences.length ? "No matching experiences." : "No experiences yet. Import a complete LeRobot v3.0 dataset or a Humaan recording folder."}</p>{/each}
        </div>
        <details class="border-t border-border-strong p-4 text-xs text-muted"><summary>Project identity</summary><p class="mt-2 break-all font-mono">{project.project.id}</p></details>
      </aside>
      <div class="min-w-72 flex-1 overflow-y-auto px-7 py-6">
        {#if verification}
          <section class="mb-6 rounded border border-border-strong p-4" aria-label="Verification results"><h2 class="mb-3 font-semibold">Verification results</h2>
            {#each verification as result}<div class="mb-2 break-all text-xs"><span class={result.valid ? 'text-success' : 'text-error'}>{result.valid ? "OK" : "FAIL"}</span>
              <button class="result" disabled={!!busy || dirty || !result.valid} onclick={() => inspect(result.id)}>{result.id}</button>
              {#if result.error}<p class="mt-1 text-error">{result.error}</p>{/if}</div>{/each}
          </section>
        {/if}
        {#if inspection}
          <div class="mb-5 flex items-start justify-between gap-4"><div><p class="text-xs uppercase tracking-widest text-muted">Experience</p><h2 class="mt-2 text-xl font-medium">{inspection.manifest.task}</h2></div><button disabled={!!busy || dirty} onclick={exportRecording}>Export</button></div>
          <p class="mb-5 break-all font-mono text-xs text-muted">{inspection.manifest.id}</p>
          {#if inspection.source}
            <button class="primary mb-5" disabled={!!busy} onclick={replay}>Open in Rerun ↗</button>
          {/if}
          <dl class="grid grid-cols-2 gap-4 rounded-lg border border-border-strong bg-surface p-5">
            <div><dt>Robot</dt><dd>{inspection.manifest.robot}</dd></div><div><dt>Integrity</dt><dd class="text-success">Verified</dd></div>
            <div><dt>Outcome</dt><dd>{latest(inspection.manifest.id)?.review.outcome ?? "Unknown"}</dd></div><div><dt>Policy</dt><dd>Unknown</dd></div>
            {#if inspection.source}<div><dt>Source</dt><dd>LeRobot v3.0 · episode {inspection.source.episode_index}</dd></div><div><dt>Duration</dt><dd>{inspection.source.duration_s.toFixed(2)}s · {inspection.source.frames} frames at {inspection.source.fps} FPS</dd></div>{/if}
          </dl>
          <form class="mt-5 rounded-lg border border-border-strong bg-surface p-5" onsubmit={(event) => { event.preventDefault(); saveReview(false); }}>
            <h3 class="font-medium">Review outcome</h3>
            <p class="my-3 text-xs text-muted">Judge against the task’s success criteria. Keep the outcome unknown when evidence is insufficient. Assistance is separate from success.</p>
            <fieldset disabled={!!busy} class="grid grid-cols-2 gap-4">
              <label>Final outcome<select class="mt-1 w-full" bind:value={draft.outcome}><option value="unknown">Unknown</option><option value="success">Success</option><option value="failure">Failure</option><option value="timeout">Timeout</option><option value="aborted">Aborted</option></select></label>
              <label>Human assistance<select class="mt-1 w-full" bind:value={draft.assistance}><option value="unknown">Unknown</option><option value="yes">Yes</option><option value="no">No</option></select></label>
              <label>Reviewer<input class="mt-1 w-full" bind:value={draft.reviewer} required maxlength="200" placeholder="Your name" /></label>
              <label>Review state<select class="mt-1 w-full" bind:value={draft.review_state}><option value="reviewed">Reviewed</option><option value="unreviewed">Unreviewed</option></select></label>
              <label class="col-span-2">Notes<textarea class="mt-1 w-full" rows="3" bind:value={draft.notes} maxlength="10000" placeholder="Evidence, failure reason, or uncertainty"></textarea></label>
            </fieldset>
            <div class="mt-4 flex flex-wrap gap-2">
              <button class="primary" type="submit" disabled={!!busy || !draft.reviewer.trim()}>Save review</button>
              <button type="button" disabled={!!busy || !draft.reviewer.trim()} onclick={() => saveReview(true)}>Save and next</button>
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
          <p class="mt-6 text-xs leading-relaxed text-muted">Verification checks recording integrity, not task success or training compatibility. LeRobot replay opens the selected episode in the standalone Rerun viewer.</p>
        {:else}<div class="flex min-h-48 flex-col items-center justify-center gap-3 text-center"><h2 class="text-lg">Inspect an experience</h2><p class="max-w-sm leading-relaxed text-muted">Select an attempt to verify its files and view source metadata. Outcomes stay unknown until reviewed.</p></div>{/if}
      </div>
    </div>
  {/if}
</section>
<style>
  button { display: inline-flex; align-items: center; justify-content: center; gap: 0.5rem; border-radius: 6px; padding: 0.5rem 0.85rem; font-size: 0.8rem; background: var(--color-raised); border: 1px solid var(--color-border-strong); }
  .primary { background: var(--color-accent); color: #101010; font-weight: 600; }
  .entry { display: block; width: 100%; border-radius: 0; border: 0; border-bottom: 1px solid var(--color-border); padding: 1rem; text-align: left; background: transparent; }
  .entry:hover, .entry.selected { background: var(--color-selection); }
  .result { margin-left: 0.5rem; padding: 0; border: 0; background: transparent; color: var(--color-link); text-decoration: underline; overflow-wrap: anywhere; text-align: left; }
  button:disabled { opacity: 0.45; cursor: default; }
  input, select, textarea { border: 1px solid var(--color-border-strong); border-radius: 5px; background: var(--color-raised); padding: 0.6rem; font-size: 0.8rem; }
  input:focus-visible, select:focus-visible, textarea:focus-visible, button:focus-visible, summary:focus-visible { outline: 2px solid var(--color-accent); outline-offset: 2px; }
  summary { cursor: pointer; }
  dt { color: var(--color-muted); font-size: 0.75rem; margin-bottom: 0.3rem; }
  dd { overflow-wrap: anywhere; }
</style>
