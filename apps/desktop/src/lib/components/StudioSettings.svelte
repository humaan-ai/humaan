<script lang="ts">
  import * as Tabs from "$lib/components/ui/tabs/index.js";
  import * as Dialog from "$lib/components/ui/dialog/index.js";
  import IconButton from "$lib/components/ui/icon-button.svelte";
  import { Button } from "$lib/components/ui/button/index.js";
  import { Input } from "$lib/components/ui/input/index.js";
  import { onMount } from "svelte";
  import Settings from "@lucide/svelte/icons/settings";
  import X from "@lucide/svelte/icons/x";
  import Info from "@lucide/svelte/icons/info";
  import Wrench from "@lucide/svelte/icons/wrench";
  import { studio } from "$lib/studio.svelte";
  import { readRerunPath, saveRerunPath } from "$lib/tool-settings";
  import SimulationTools from "./SimulationTools.svelte";
  import DatasetTools from "./DatasetTools.svelte";
  import { invoke, isTauri } from "@tauri-apps/api/core";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { checkStudioUpdate, type StudioUpdate } from "$lib/updates";
  import { version } from "../../../package.json";

  let { open = $bindable(false) }: { open?: boolean } = $props();
  let checkingUpdate = $state(false);
  let openingDownload = $state(false);
  let availableUpdate = $state<StudioUpdate | null>(null);
  let updateStatus = $state("");
  let updateError = $state("");
  async function checkUpdates() {
    if (checkingUpdate || openingDownload) return;
    checkingUpdate = true; availableUpdate = null; updateStatus = ""; updateError = "";
    try {
      if (!isTauri()) throw new Error("Open Studio to check for updates.");
      const target = await invoke<string>("studio_update_target");
      availableUpdate = await checkStudioUpdate(version, target);
      updateStatus = availableUpdate
        ? `Version ${availableUpdate.version} is available for your platform.`
        : "No newer download is available for your platform and release channel.";
    } catch (cause) { updateError = cause instanceof Error ? cause.message : String(cause); }
    finally { checkingUpdate = false; }
  }
  async function openDownload() {
    if (!availableUpdate) return;
    openingDownload = true; updateError = "";
    try { await invoke("open_studio_download", { url: availableUpdate.url }); }
    catch (cause) { updateError = cause instanceof Error ? cause.message : String(cause); }
    finally { openingDownload = false; }
  }
  let toolsVisited = $state(false);
  $effect(() => { if (selected === "tools") toolsVisited = true; });
  let selected = $state("general");
  $effect(() => { if (studio.openTools) { selected = "tools"; open = true; studio.openTools = false; } });
  let rerunPath = $state("");
  let notice = $state("");
  let error = $state("");
  const tabs = [{ id: "general", label: "General", icon: Settings }, { id: "tools", label: "Tools", icon: Wrench }, { id: "about", label: "About", icon: Info }];
  onMount(() => {
    try { rerunPath = readRerunPath(); } catch (cause) { error = String(cause); }
    if (!isTauri()) return;
    let disposed = false;
    let unlisten: UnlistenFn | undefined;
    void listen("studio:check-updates", () => {
      if (disposed) return;
      selected = "about";
      open = true;
      void checkUpdates();
    }).then((cleanup) => {
      if (disposed) cleanup();
      else unlisten = cleanup;
    }).catch((cause) => {
      if (!disposed) updateError = `Update menu unavailable: ${String(cause)}`;
    });
    return () => { disposed = true; unlisten?.(); };
  });
  function save() {
    try { saveRerunPath(rerunPath.trim()); rerunPath = rerunPath.trim(); notice = "Rerun configuration saved. It applies the next time you open an experience."; error = ""; }
    catch (cause) { error = `Could not save settings: ${String(cause)}`; notice = ""; }
  }

</script>

<Dialog.Root bind:open>
{#if open}
<Dialog.Content id="studio-settings-dialog" class="max-w-4xl overflow-hidden p-0" showCloseButton={false} onInteractOutside={(event) => event.preventDefault()} onCloseAutoFocus={(event) => { event.preventDefault(); document.getElementById("studio-settings-trigger")?.focus(); }}>
  <Dialog.Description class="sr-only">Project information and local tool settings.</Dialog.Description>
  <header class="flex items-center justify-between border-b border-border px-5 py-4">
    <Dialog.Title class="flex items-center gap-2"><Settings size={18} aria-hidden="true" />Settings</Dialog.Title>
    <IconButton class="text-muted" aria-label="Close settings" onclick={() => open = false}><X size={18} aria-hidden="true" /></IconButton>
  </header>
  <Tabs.Root bind:value={selected} orientation="vertical" class="h-[min(36rem,70vh)] min-h-0 gap-0">
    <Tabs.List role="tablist" aria-label="Settings sections" aria-orientation="vertical" class="h-full w-32 shrink-0 justify-start rounded-none border-r border-border bg-surface p-2 sm:w-44 sm:p-3">
      {#each tabs as tab}
        <Tabs.Trigger id="settings-tab-{tab.id}" value={tab.id} class="w-full justify-start">
          <tab.icon size={16} aria-hidden="true" />{tab.label}
        </Tabs.Trigger>
      {/each}
    </Tabs.List>
    <div class="min-w-0 flex-1 overflow-y-auto p-5 sm:p-8">
      <Tabs.Content value="general" id="settings-panel-general" class="max-w-2xl" role="tabpanel" aria-labelledby="settings-tab-general" tabindex={0} hidden={selected !== "general"}>
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
      </Tabs.Content>
      <Tabs.Content value="about" id="settings-panel-about" class="max-w-2xl" role="tabpanel" aria-labelledby="settings-tab-about" tabindex={0} hidden={selected !== "about"}>
        <h3 class="text-lg font-medium">About</h3>
        <div class="mt-6">
          <h4 class="text-base font-medium">Humaan Studio</h4>
          <p class="mt-2 text-sm text-muted">Version <span class="font-mono text-foreground">{version}</span></p>
          <p class="mt-2 text-xs text-muted">{version.includes("-") ? "Checks stable and early-access releases." : "Checks stable releases only."} Downloads open in your browser for manual installation.</p>
          <div class="mt-5 flex flex-wrap gap-2">
            <Button type="button" disabled={checkingUpdate || openingDownload} onclick={checkUpdates} aria-describedby="settings-update-status">{checkingUpdate ? "Checking…" : "Check for updates"}</Button>
            {#if availableUpdate}
              <Button type="button" disabled={openingDownload || checkingUpdate} onclick={openDownload}>{openingDownload ? "Opening…" : `Download ${availableUpdate.version}`}</Button>
            {/if}
          </div>
          <p id="settings-update-status" role="status" class="mt-2 text-xs leading-relaxed text-muted">{updateStatus}</p>
          {#if updateError}<p role="alert" class="mt-2 text-xs text-error">{updateError}</p>{/if}
        </div>
      </Tabs.Content>
      <Tabs.Content value="tools" id="settings-panel-tools" class="max-w-2xl" role="tabpanel" aria-labelledby="settings-tab-tools" tabindex={0} hidden={selected !== "tools"}>
        <h3 class="text-lg font-medium">Tools</h3>
        <p class="mt-1 text-sm text-muted">Install and configure tools for robot simulation, experience replay, and dataset preparation.</p>
        {#if toolsVisited}<SimulationTools />{/if}
        <form class="mt-6 rounded-lg border border-border-strong p-4" onsubmit={(event) => { event.preventDefault(); save(); }}>
          <h4 class="text-sm font-medium">Rerun</h4>
          <p class="mt-1 text-xs leading-relaxed text-muted">Open experience replays in the standalone Rerun 0.37.2 viewer.</p>
          <label for="rerun-path" class="mb-2 mt-4 block text-xs">Executable path</label>
          <Input id="rerun-path" type="text" bind:value={rerunPath} oninput={() => notice = ""} placeholder="Automatic detection" aria-describedby="rerun-path-help" class="font-mono" />
          <p id="rerun-path-help" class="mt-2 text-xs leading-relaxed text-muted">Enter the full path to your installed viewer. Leave blank to use HUMAAN_RERUN_PATH, the Cargo bin folder, or PATH. The viewer version is checked when opening a replay.</p>
          <Button type="submit" class="mt-4">Save configuration</Button>
          {#if notice}<p role="status" class="mt-3 text-xs text-muted">{notice}</p>{/if}
          {#if error}<p role="alert" class="mt-3 text-xs text-error">{error}</p>{/if}
        </form>
        <div class="mt-6">
          <h4 class="text-sm font-medium">Dataset tools</h4>
          <p class="mt-1 text-xs leading-relaxed text-muted">Manage the local environment used to prepare training datasets.</p>
          {#if toolsVisited}<DatasetTools />{/if}
        </div>
      </Tabs.Content>
    </div>
  </Tabs.Root>
  <footer class="flex justify-end border-t border-border px-5 py-3">
    <Button onclick={() => open = false}>Close</Button>
  </footer>
</Dialog.Content>
{/if}
</Dialog.Root>
