<script lang="ts">
  import { projectState } from "$lib/project.svelte";
  import CodeEditor from "$lib/components/CodeEditor.svelte";
  import ThreeViewport from "$lib/components/ThreeViewport.svelte";
  import X from "@lucide/svelte/icons/x";
  import Folder from "@lucide/svelte/icons/folder";
  import File from "@lucide/svelte/icons/file";
  import { invoke } from "@tauri-apps/api/core";

  let is3DFile = $derived.by(() => {
    const file = projectState.selectedFile;
    if (!file) return false;
    const ext = file.split('.').pop()?.toLowerCase() || '';
    return ['gltf', 'glb', 'stl', 'urdf'].includes(ext);
  });

  // Derive breadcrumb array from the active selected file path
  let breadcrumbs = $derived.by(() => {
    if (!projectState.selectedFile) return [];
    let relative = projectState.selectedFile;
    if (projectState.rootPath && relative.startsWith(projectState.rootPath)) {
      relative = relative.slice(projectState.rootPath.length);
    }
    // Remove leading slashes
    if (relative.startsWith('/') || relative.startsWith('\\')) {
      relative = relative.slice(1);
    }
    return relative.split(/[/\\]/).filter(Boolean);
  });

  // Derive absolute accumulated paths for dropdown navigation
  let accumulatedPaths = $derived.by(() => {
    let current = projectState.rootPath || "";
    return breadcrumbs.map(part => {
      current = current + (current.endsWith('/') || current.endsWith('\\') ? '' : '/') + part;
      return { name: part, path: current };
    });
  });

  let activeDropdown = $state<string | null>(null);
  let dropdownItems = $state<any[]>([]);

  async function toggleDropdown(path: string) {
    if (activeDropdown === path) {
      activeDropdown = null;
      dropdownItems = [];
    } else {
      activeDropdown = path;
      try {
        const entries = await invoke<any[] | null>("read_directory", { path });
        dropdownItems = entries || [];
      } catch (err) {
        console.error("Failed to load dropdown items:", err);
        dropdownItems = [];
      }
    }
  }
</script>

<!-- Close any active breadcrumb dropdown on window click -->
<svelte:window onclick={() => { activeDropdown = null; dropdownItems = []; }} />

{#if projectState.selectedFile}
  <div class="flex flex-col h-full w-full bg-background overflow-hidden">
    <!-- Tab Bar -->
    <div class="flex h-8 shrink-0 items-center justify-between border-b border-border bg-surface select-none">
      <div class="flex h-full items-center overflow-x-auto min-w-0">
        {#each projectState.openFiles as file}
          {@const isActive = projectState.selectedFile === file}
          {@const isDirty = projectState.selectedFile === file ? projectState.isDirty : false}
          <!-- svelte-ignore a11y_click_events_have_key_events -->
          <!-- svelte-ignore a11y_no_static_element_interactions -->
          <div
            class="group relative flex h-full items-center gap-2 pl-4 pr-2 border-r border-border/40 text-xs transition-colors cursor-pointer {isActive ? 'bg-background text-foreground font-medium' : 'bg-surface text-muted hover:bg-hover hover:text-foreground'}"
            onclick={() => projectState.selectFile(file)}
          >
            <span class="truncate max-w-[120px]">{file.split('/').pop()}</span>
            
            <button
              class="flex h-5 w-5 items-center justify-center rounded-sm hover:bg-raised text-muted hover:text-foreground transition-all"
              onclick={(e) => {
                e.stopPropagation();
                projectState.closeFile(file);
              }}
            >
              {#if isDirty}
                <span class="h-2 w-2 rounded-full bg-accent group-hover:hidden"></span>
                <span class="hidden group-hover:block font-mono text-lg leading-none">✕</span>
              {:else}
                <span class="font-mono text-lg leading-none">
                  <X class="h-3 w-3" />   
                </span>
              {/if}
            </button>
          </div>
        {/each}
      </div>
    </div>
    
    <!-- Breadcrumb Bar -->
    <div class="flex h-[26px] shrink-0 items-center gap-1.5 px-4 text-[11px] text-muted border-b border-border bg-background select-none relative z-50">
      {#each accumulatedPaths as segment, index}
        {@const isLast = index === accumulatedPaths.length - 1}
        {#if index > 0}
          <span class="text-muted/30 font-normal">›</span>
        {/if}
        
        <div class="relative flex items-center">
          <button
            class="flex items-center gap-1 hover:text-foreground transition-colors cursor-pointer {isLast ? 'text-foreground/80 font-medium' : ''}"
            onclick={(e) => {
              e.stopPropagation();
              if (!isLast) {
                toggleDropdown(segment.path);
              }
            }}
          >
            {#if isLast}
              <File size={12} class="text-muted shrink-0" />
            {:else}
              <Folder size={12} class="text-accent shrink-0" />
            {/if}
            <span>{segment.name}</span>
          </button>
          
          <!-- Dropdown Menu -->
          {#if activeDropdown === segment.path && dropdownItems.length > 0}
            <!-- svelte-ignore a11y_click_events_have_key_events -->
            <!-- svelte-ignore a11y_no_static_element_interactions -->
            <div
              class="absolute left-0 top-full mt-1 min-w-[160px] max-h-[300px] overflow-y-auto rounded border border-border-strong bg-surface p-1 shadow-lg z-30"
              onclick={(e) => e.stopPropagation()}
            >
              {#each dropdownItems as item}
                <button
                  class="flex h-6 w-full items-center gap-1.5 rounded px-2 text-left text-[12px] text-foreground/90 hover:bg-hover hover:text-foreground transition-colors"
                  onclick={() => {
                    if (item.isDir) {
                      toggleDropdown(item.path);
                    } else {
                      projectState.selectFile(item.path);
                      activeDropdown = null;
                    }
                  }}
                >
                  {#if item.isDir}
                    <Folder size={12} class="text-accent shrink-0" />
                  {:else}
                    <File size={12} class="text-muted shrink-0" />
                  {/if}
                  <span class="truncate">{item.name}</span>
                </button>
              {/each}
            </div>
          {/if}
        </div>
      {/each}
    </div>
    
    {#if is3DFile}
      <ThreeViewport />
    {:else}
      <CodeEditor />
    {/if}
  </div>
{:else}
  <div class="flex h-full flex-col items-center justify-center gap-8 select-none bg-background">
    <h1 class="text-5xl font-semibold tracking-tight text-foreground/10">Assembly Intelligence</h1>

    <div class="grid grid-cols-[auto_auto] items-center gap-x-5 gap-y-2.5 text-[13px] text-muted">
      <span class="text-right">Toggle Primary Side Bar</span>
      <span><kbd>⌘</kbd><kbd>B</kbd></span>
      <span class="text-right">Toggle Panel</span>
      <span><kbd>⌘</kbd><kbd>J</kbd></span>
      <span class="text-right">Toggle Configuration</span>
      <span><kbd>⌥</kbd><kbd>⌘</kbd><kbd>B</kbd></span>
    </div>
  </div>
{/if}

<style>
  kbd {
    display: inline-block;
    min-width: 18px;
    margin: 0 1px;
    padding: 1px 4px;
    border: 1px solid var(--color-border-strong);
    border-bottom-width: 2px;
    border-radius: 3px;
    background: var(--color-raised);
    font-size: 11px;
    text-align: center;
  }
</style>
