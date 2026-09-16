<script lang="ts">
  import ActionLabel from "./ActionLabel.svelte";
  import { feedback } from "$lib/feedback";
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import type { PreparationToolStatus } from "$lib/dataset-preparation";
  let { tools = $bindable(null), busy = $bindable(false) }: {
    tools?: PreparationToolStatus | null;
    busy?: boolean;
  } = $props();
  let installing = $state(false);
  let checking = $state(false);
  let error = $state("");
  let alive = true;
  const buttonClass = "rounded-md border border-border-strong bg-raised px-3 py-2 text-xs hover:bg-selection focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-accent disabled:opacity-45";
  async function refresh() {
    if (checking) return;
    checking = true;
    try {
      const status = await invoke<PreparationToolStatus>("dataset_tools_status");
      if (alive) { tools = status; error = ""; }
    } catch (cause) { if (alive) error = String(cause); }
    finally { checking = false; }
  }
  async function install() {
    if (busy || tools?.installing) return;
    busy = true; installing = true; error = "";
    const activity = feedback.start("Setting up dataset tools", "Downloading and installing the local dataset environment.");
    try {
      tools = await invoke<PreparationToolStatus>("dataset_tools_install");
      feedback.publish({ title: "Dataset tools ready", detail: "You can now prepare your training dataset.", kind: "success", popup: true });
    } catch (cause) { feedback.publish({ title: "Dataset tool setup failed", detail: `${String(cause)}\nCheck the tool status and use Set up or Repair to try again.`, kind: "error", popup: true }); }
    finally { feedback.finish(activity); installing = false; busy = false; }
  }
  onMount(() => {
    alive = true;
    refresh();
    const timer = setInterval(() => { if (installing || tools?.installing) refresh(); }, 2000);
    return () => { alive = false; clearInterval(timer); };
  });
</script>

<section class="mt-3 rounded-md border border-border-strong p-4" aria-label="Dataset tools" aria-busy={installing}>
  <p class="text-sm" role="status">{tools?.phase ?? "Checking dataset tools…"}</p>
  {#if !tools?.ready}<p class="mt-2 max-w-xl text-xs leading-relaxed text-muted">One-time setup downloads the tools needed to prepare datasets. It may take several minutes and substantial disk space. After setup, preparation works offline.</p>{/if}
  {#if tools?.custom}<p class="mt-2 text-xs text-muted">Using a custom dataset environment. Manage its installation outside Studio.</p>{/if}
  <div class="mt-3 flex flex-wrap gap-2">
    {#if tools && !tools.custom}<button type="button" class={buttonClass} disabled={busy || tools.installing} onclick={install}><ActionLabel active={installing || tools.installing} label={tools.ready ? "Repair dataset tools" : "Set up dataset tools"} pending="Setting up…" /></button>{/if}
    <button type="button" class={buttonClass} disabled={busy || checking} onclick={refresh}><ActionLabel active={checking} label="Refresh tool status" pending="Checking…" /></button>
  </div>
  {#if error || tools?.error}<p class="mt-3 whitespace-pre-wrap break-words text-xs text-error" role="alert">{error || tools?.error}</p>{/if}
</section>
