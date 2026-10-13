<script lang="ts">
  import { Checkbox } from "$lib/components/ui/checkbox/index.js";
  import { NativeSelect } from "$lib/components/ui/native-select/index.js";
  import { Button } from "$lib/components/ui/button/index.js";
  import ActionLabel from "./ActionLabel.svelte";
  import { feedback } from "$lib/feedback";
  import { eligibilityReason, preparationBlockers, type PreparationToolStatus as ToolStatus } from "../dataset-preparation";
  import DatasetTools from "./DatasetTools.svelte";
  import { invoke } from "@tauri-apps/api/core";
  let { root, set, busy = $bindable(false) }: {
    root: string;
    set: { id: string; members: { experience_id: string; review: { review: { outcome: string; assistance: string; review_state: string } } }[] };
    busy?: boolean;
  } = $props();
  let tools = $state<ToolStatus | null>(null);
  let assignments = $state<Record<string, string>>({});
  let approved = $state(false);
  let error = $state("");
  let preparing = $state(false);
  const eligible = $derived(set.members.filter(m => !eligibilityReason(m.review.review)));
  const train = $derived(eligible.filter(m => assignments[m.experience_id] === "train").map(m => m.experience_id));
  const validation = $derived(eligible.filter(m => assignments[m.experience_id] === "validation").map(m => m.experience_id));
  const blockers = $derived(preparationBlockers(tools, eligible.length, train.length, validation.length, approved));
  async function prepare() {
    if (busy) return;
    busy = true; preparing = true; error = "";
    let activity: number | null = null;
    try {
      tools = await invoke<ToolStatus>("dataset_tools_status");
      const missing = preparationBlockers(tools, eligible.length, train.length, validation.length, approved);
      if (missing.length) { error = missing.join("\n"); return; }
      const parent = await invoke<string | null>("select_directory", { title: "Choose parent folder for prepared LeRobot dataset" });
      if (!parent) return;
      activity = feedback.start("Preparing dataset", "Verifying recordings, extracting frames, and checking DataLoader batches.");
      const destination = await invoke<string>("learning_set_prepare", { root, id: set.id, parent, config: {
        version: 1, approve_whole_trajectories: approved, train, validation
      } });
      feedback.publish({ title: "Dataset prepared", detail: `Validated with the LeRobot PyTorch DataLoader. No model training was run.\n${destination}`, copyText: destination, kind: "success", popup: true });
    } catch (e) { feedback.publish({ title: "Dataset preparation failed", detail: String(e), kind: "error", popup: true }); }
    finally { if (activity !== null) feedback.finish(activity); busy = false; preparing = false; }
  }
</script>

<form class="mt-4 rounded border border-border-strong p-4" onsubmit={(e) => { e.preventDefault(); prepare(); }}>
  <h4 class="font-medium">Prepare LeRobot training data</h4>
  <p class="mt-2 text-xs text-muted">Assign whole episodes to training or held-out validation. This first version accepts reviewed successes with no human assistance, matching float32 state/action vectors, and RGB cameras. Train and validation each need at least one episode.</p>
  <p class="mt-2 text-xs text-muted">{set.members.length - eligible.length} members are ineligible. Outcomes come from this set’s pinned reviews.</p>
  <DatasetTools bind:tools bind:busy />
  {#if set.members.length !== eligible.length}
    <details class="mt-3 text-xs"><summary>Why some episodes cannot be used ({set.members.length - eligible.length})</summary>
      {#each set.members.filter(m => eligibilityReason(m.review.review)) as member}
        <p class="mt-2 break-words"><span class="font-mono">{member.experience_id}</span>: {eligibilityReason(member.review.review)}</p>
      {/each}
      <p class="mt-2 text-muted">These are the reviews saved with this learning set. If you updated them afterward, curate a new set to use those changes.</p>
    </details>
  {/if}
  <fieldset disabled={busy} class="mt-3">
    {#each eligible as member}
      <label class="mb-2 flex flex-wrap items-center gap-3 text-xs"><span class="min-w-0 flex-1 break-all">{member.experience_id}</span>
        <NativeSelect  value={assignments[member.experience_id] ?? "exclude"} onchange={(e) => assignments[member.experience_id] = e.currentTarget.value}>
          <option value="exclude">Exclude</option><option value="train">Train</option><option value="validation">Validation</option>
        </NativeSelect>
      </label>
    {/each}
    <p class="mt-3 text-xs text-muted">{train.length} train · {validation.length} validation · {set.members.length - train.length - validation.length} excluded</p>
    <label class="mt-3 flex items-start gap-2 text-xs"><Checkbox bind:checked={approved} aria-label="Approve selected trajectories for imitation" /><span>I have checked that the complete selected trajectories contain behavior suitable for imitation.</span></label>
    {#if blockers.length}
      <div class="mt-3 rounded border border-border-strong p-3 text-xs" aria-live="polite"><p class="font-medium">Before preparing:</p><ul class="mt-2 list-disc space-y-1 pl-4">{#each blockers as blocker}<li>{blocker}</li>{/each}</ul></div>
    {:else}<p class="mt-3 text-xs text-success" role="status">Ready to prepare: tools, split assignments, and confirmation are complete.</p>{/if}
    <Button variant="default" class="mt-3" type="submit"><ActionLabel active={preparing} label="Prepare and validate dataset" pending="Preparing…" /></Button>
  </fieldset>
  {#if error}<p class="mt-3 whitespace-pre-wrap break-words text-error" role="alert">{error}</p>{/if}
  <p class="mt-3 text-xs text-muted">Humaan manages dataset tools locally. Output includes independent train/validation datasets and a preparation report. This checks data compatibility; it does not train or evaluate policy behavior.</p>
</form>
