<script lang="ts">
  import { onMount } from "svelte";
  import Settings from "@lucide/svelte/icons/settings";
  import X from "@lucide/svelte/icons/x";
  import Info from "@lucide/svelte/icons/info";
  import Wrench from "@lucide/svelte/icons/wrench";
  import { studio } from "$lib/studio.svelte";
  import { readRerunPath, saveRerunPath } from "$lib/tool-settings";
  import DatasetTools from "./DatasetTools.svelte";
  import { version } from "../../../package.json";

  let { open = $bindable(false) }: { open?: boolean } = $props();
  let dialog: HTMLDialogElement;
  let toolsVisited = $state(false);
  $effect(() => {
    if (open) dialog?.showModal();
    else dialog?.close();
  });
  $effect(() => { if (selected === "tools") toolsVisited = true; });
  let selected = $state("general");
  let rerunPath = $state("");
  let notice = $state("");
  let error = $state("");
  const tabs = [{ id: "general", label: "General", icon: Settings }, { id: "tools", label: "Tools", icon: Wrench }, { id: "about", label: "About", icon: Info }];
  const buttonClass = "rounded-md border border-border-strong bg-raised px-3 py-2 text-xs hover:bg-selection focus-visible:outline-2 focus-visible:outline-accent";
  onMount(() => {
    try { rerunPath = readRerunPath(); } catch (cause) { error = String(cause); }
  });
  function save() {
    try { saveRerunPath(rerunPath.trim()); rerunPath = rerunPath.trim(); notice = "Rerun configuration saved. It applies the next time you open an experience."; error = ""; }
    catch (cause) { error = `Could not save settings: ${String(cause)}`; notice = ""; }
  }
  function navigate(event: KeyboardEvent, index: number) {
    if (!["ArrowDown", "ArrowUp", "Home", "End"].includes(event.key)) return;
    event.preventDefault();
    const next = event.key === "Home" ? 0 : event.key === "End" ? tabs.length - 1 : (index + (event.key === "ArrowDown" ? 1 : -1) + tabs.length) % tabs.length;
    selected = tabs[next].id;
    document.getElementById(`settings-tab-${selected}`)?.focus();
  }
</script>

<dialog id="studio-settings-dialog" bind:this={dialog} aria-labelledby="studio-settings-title" class="fixed m-auto w-[calc(100%_-_2rem)] max-w-4xl overflow-hidden rounded-xl border border-border-strong bg-background p-0 text-foreground shadow-2xl backdrop:bg-black/60" onclose={() => { open = false; document.getElementById("studio-settings-trigger")?.focus(); }}>
  <header class="flex items-center justify-between border-b border-border px-5 py-4">
    <h2 id="studio-settings-title" class="flex items-center gap-2 text-base font-medium"><Settings size={18} aria-hidden="true" />Settings</h2>
    <button class="rounded p-1 text-muted hover:bg-selection hover:text-foreground focus-visible:outline-2 focus-visible:outline-accent" aria-label="Close settings" onclick={() => open = false}><X size={18} aria-hidden="true" /></button>
  </header>
  <div class="flex h-[min(36rem,70vh)] min-h-0">
    <div role="tablist" aria-label="Settings sections" aria-orientation="vertical" class="w-32 shrink-0 space-y-1 border-r border-border bg-surface p-2 sm:w-44 sm:p-3">
      {#each tabs as tab, index}
        <button id="settings-tab-{tab.id}" role="tab" aria-selected={selected === tab.id} aria-controls="settings-panel-{tab.id}" tabindex={selected === tab.id ? 0 : -1} class="flex w-full items-center gap-2 rounded-md px-3 py-2 text-left text-sm focus-visible:outline-2 focus-visible:outline-accent {selected === tab.id ? 'bg-selection text-foreground' : 'text-muted hover:bg-hover hover:text-foreground'}" onclick={() => selected = tab.id} onkeydown={(event) => navigate(event, index)}>
          <tab.icon size={16} aria-hidden="true" />{tab.label}
        </button>
      {/each}
    </div>
    <div class="min-w-0 flex-1 overflow-y-auto p-5 sm:p-8">
      <div id="settings-panel-general" class="max-w-2xl" role="tabpanel" aria-labelledby="settings-tab-general" tabindex="0" hidden={selected !== "general"}>
        <h3 class="text-lg font-medium">General</h3>
        <p class="mt-1 text-sm text-muted">Your local project details.</p>
        <h4 class="mt-6 text-sm font-medium">Project information</h4>
        {#if studio.project}
          <dl class="mt-4 space-y-4 text-sm">
            <div><dt class="mb-1 text-muted">Name</dt><dd>{studio.project.project.name}</dd></div>
            <div><dt class="mb-1 text-muted">Folder</dt><dd class="break-all font-mono text-xs">{studio.project.root}</dd></div>
            <div><dt class="mb-1 text-muted">Project ID</dt><dd class="break-all font-mono text-xs">{studio.project.project.id}</dd></div>
          </dl>
          <p class="mt-4 text-xs text-muted">Stored locally. No account or cloud connection required.</p>
        {:else}<p class="mt-3 text-sm text-muted">Open a project to see its details.</p>{/if}
      </div>
      <div id="settings-panel-about" class="max-w-2xl" role="tabpanel" aria-labelledby="settings-tab-about" tabindex="0" hidden={selected !== "about"}>
        <h3 class="text-lg font-medium">About</h3>
        <div class="mt-6">
          <h4 class="text-base font-medium">Humaan Studio</h4>
          <p class="mt-2 text-sm text-muted">Version <span class="font-mono text-foreground">{version}</span></p>
          <button type="button" class="{buttonClass} mt-5 disabled:cursor-not-allowed disabled:opacity-45" disabled aria-describedby="settings-update-status">Check for updates</button>
          <p id="settings-update-status" class="mt-2 text-xs leading-relaxed text-muted">Update checking is not configured for this alpha build.</p>
        </div>
      </div>
      <div id="settings-panel-tools" class="max-w-2xl" role="tabpanel" aria-labelledby="settings-tab-tools" tabindex="0" hidden={selected !== "tools"}>
        <h3 class="text-lg font-medium">Tools</h3>
        <p class="mt-1 text-sm text-muted">Configure the local tools used to inspect experiences and prepare datasets.</p>
        <form class="mt-6 rounded-lg border border-border-strong p-4" onsubmit={(event) => { event.preventDefault(); save(); }}>
          <h4 class="text-sm font-medium">Rerun</h4>
          <p class="mt-1 text-xs leading-relaxed text-muted">Open experience replays in the standalone Rerun 0.37.2 viewer.</p>
          <label for="rerun-path" class="mb-2 mt-4 block text-xs">Executable path</label>
          <input id="rerun-path" type="text" bind:value={rerunPath} oninput={() => notice = ""} placeholder="Automatic detection" aria-describedby="rerun-path-help" class="w-full rounded-md border border-border-strong bg-background px-3 py-2 font-mono text-xs focus-visible:outline-2 focus-visible:outline-accent" />
          <p id="rerun-path-help" class="mt-2 text-xs leading-relaxed text-muted">Enter the full path to your installed viewer. Leave blank to use HUMAAN_RERUN_PATH, the Cargo bin folder, or PATH. The viewer version is checked when opening a replay.</p>
          <button type="submit" class="{buttonClass} mt-4">Save configuration</button>
          {#if notice}<p role="status" class="mt-3 text-xs text-muted">{notice}</p>{/if}
          {#if error}<p role="alert" class="mt-3 text-xs text-error">{error}</p>{/if}
        </form>
        <div class="mt-6">
          <h4 class="text-sm font-medium">Dataset tools</h4>
          <p class="mt-1 text-xs leading-relaxed text-muted">Manage the local environment used to prepare training datasets.</p>
          {#if toolsVisited}<DatasetTools />{/if}
        </div>
      </div>
    </div>
  </div>
  <footer class="flex justify-end border-t border-border px-5 py-3">
    <button class={buttonClass} onclick={() => open = false}>Close</button>
  </footer>
</dialog>
