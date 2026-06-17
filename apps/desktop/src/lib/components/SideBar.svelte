<script lang="ts">
  import { layout, type ViewId } from "$lib/layout.svelte";
  import ChevronDown from "@lucide/svelte/icons/chevron-down";
  import Search from "@lucide/svelte/icons/search";
  import Funnel from "@lucide/svelte/icons/funnel";
  import Folder from "@lucide/svelte/icons/folder";
  import FolderOpen from "@lucide/svelte/icons/folder-open";
  import File from "@lucide/svelte/icons/file";
  import { projectState, type FileEntry } from "$lib/project.svelte";

  let expanded = $state<Record<string, boolean>>({});

  type Item = { label: string; badge?: string };
  type Section = { label: string; items: Item[] };

  const views: Record<ViewId, { title: string; sections: Section[] }> = {
    project: {
      title: "Project",
      sections: [
        {
          label: "Projects",
          items: [{ label: "humaan-desktop" }, { label: "humaan-kernel" }],
        },
        {
          label: "Recent",
          items: [
            { label: "agent-orchestration.md" },
            { label: "kernel-design.md" },
          ],
        },
      ],
    },
    assembly: {
      title: "Assembly",
      sections: [
        {
          label: "Parts",
          items: [{ label: "perception-stack" }, { label: "control-loop" }],
        },
        {
          label: "Modules",
          items: [
            { label: "planner" },
            { label: "retriever" },
            { label: "actuator" },
          ],
        },
        {
          label: "Assemblies",
          items: [
            { label: "warehouse-automation" },
            { label: "social-bot framework" },
          ],
        },
      ],
    },
    simulation: {
      title: "Simulation",
      sections: [
        {
          label: "Scenarios",
          items: [{ label: "multi-agent-swarm" }, { label: "warehouse-pick" }],
        },
        {
          label: "Environments",
          items: [{ label: "physics-lab" }, { label: "gridworld" }],
        },
      ],
    },
  };

  let collapsed = $state<Record<string, boolean>>({});
  let selected = $state<string | null>(null);

  const view = $derived(views[layout.activeView]);

  let query = $state("");

  $effect(() => {
    void layout.activeView;
    query = "";
  });

  const visibleSections = $derived.by(() => {
    const q = query.trim().toLowerCase();
    if (!q) return view.sections;
    return view.sections
      .map((s) => ({
        ...s,
        items: s.items.filter((i) => i.label.toLowerCase().includes(q)),
      }))
      .filter((s) => s.items.length > 0);
  });

  let resizing = $state(false);

  function onResize(e: PointerEvent) {
    if (!resizing) return;
    // 48px = activity bar width to the left of the sidebar
    layout.sidebarWidth = Math.min(480, Math.max(160, e.clientX - 48));
  }
</script>

<aside
  class="relative flex shrink-0 flex-col border-r border-border bg-surface select-none"
  style="width: {layout.sidebarWidth}px"
>
  <div
    class="flex h-[35px] shrink-0 items-center px-4 text-[11px] font-bold tracking-wider text-muted uppercase"
  >
    {view.title}
  </div>

  {#if layout.activeView === "assembly"}
    <div class="px-3 pb-2">
      <div
        class="flex items-center gap-1.5 rounded-sm border border-border-strong bg-raised px-2 py-1 text-[13px] focus-within:border-accent"
      >
        <Search size={13} strokeWidth={1.8} class="shrink-0 text-muted" />
        <input
          type="text"
          placeholder="Search parts, modules, assemblies..."
          bind:value={query}
          class="w-full bg-transparent placeholder:text-muted/60 focus:outline-none"
        />
        <button
          class="shrink-0 text-muted hover:text-foreground"
          title="Filters"
        >
          <Funnel size={13} strokeWidth={1.8} />
        </button>
      </div>
    </div>
  {/if}

  {#snippet fileNode(entry: FileEntry, depth: number)}
    {@const isExpanded = expanded[entry.path]}
    {#if entry.isDir}
      <div class="flex flex-col">
        <button
          class="flex h-[24px] w-full items-center gap-1.5 hover:bg-hover text-foreground/90 text-[13px] text-left"
          style="padding-left: {depth * 12 + 16}px"
          onclick={() => (expanded[entry.path] = !isExpanded)}
        >
          <ChevronDown
            size={14}
            strokeWidth={2}
            class="transition-transform shrink-0 {!isExpanded
              ? '-rotate-90'
              : ''} text-muted"
          />
          {#if isExpanded}
            <FolderOpen size={15} class="text-accent shrink-0" />
          {:else}
            <Folder size={15} class="text-accent shrink-0" />
          {/if}
          <span class="truncate">{entry.name}</span>
        </button>

        {#if isExpanded && entry.children}
          {#each entry.children as child}
            {@render fileNode(child, depth + 1)}
          {/each}
        {/if}
      </div>
    {:else}
      <button
        class="flex h-[24px] w-full items-center gap-1.5 hover:bg-hover text-[13px] text-left {projectState.selectedFile ===
        entry.path
          ? 'bg-selection text-foreground font-medium'
          : 'text-foreground/90'}"
        style="padding-left: {depth * 12 + 32}px"
        onclick={() => projectState.selectFile(entry.path)}
      >
        <File size={15} class="text-muted shrink-0" />
        <span class="truncate">{entry.name}</span>
      </button>
    {/if}
  {/snippet}

  {#if layout.activeView === "project"}
    {#if projectState.rootPath}
      <div
        class="flex h-[22px] w-full items-center justify-between px-4 text-[11px] font-bold tracking-wide text-foreground/80 uppercase"
      >
        <span class="truncate"
          >{projectState.rootPath.split("/").pop() || "Project"}</span
        >
      </div>
      <div class="min-h-0 flex-1 overflow-y-auto pb-4">
        {#each projectState.files as file}
          {@render fileNode(file, 0)}
        {/each}
      </div>
    {:else}
      <div
        class="flex flex-1 flex-col items-center justify-start p-6 text-center select-none"
      >
        <span class="text-xs text-muted mb-4"
          >No folder opened in workspace</span
        >
        <button
          onclick={() => projectState.openFolder()}
          class="rounded-md bg-accent px-4 py-2 text-xs font-medium text-white hover:bg-accent-hover transition-colors shadow-sm"
        >
          Open Folder
        </button>
      </div>
    {/if}
  {:else}
    <div class="min-h-0 flex-1 overflow-y-auto pb-4">
      {#each visibleSections as section}
        {@const key = layout.activeView + "/" + section.label}
        <button
          class="flex h-[22px] w-full items-center gap-0.5 px-1.5 text-[11px] font-bold tracking-wide text-foreground/80 uppercase hover:text-foreground"
          onclick={() => (collapsed[key] = !collapsed[key])}
        >
          <ChevronDown
            size={14}
            strokeWidth={2}
            class="transition-transform {collapsed[key] ? '-rotate-90' : ''}"
          />
          {section.label}
        </button>

        {#if !collapsed[key]}
          {#each section.items as item}
            <button
              class="flex h-[24px] w-full items-center justify-between px-6 text-[13px] {selected ===
              item.label
                ? 'bg-selection text-foreground'
                : 'text-foreground/90 hover:bg-hover'}"
              onclick={() => (selected = item.label)}
            >
              <span class="truncate">{item.label}</span>
              {#if item.badge}
                <span
                  class="rounded-full bg-raised px-1.5 text-[10px] text-muted"
                  >{item.badge}</span
                >
              {/if}
            </button>
          {/each}
        {/if}
      {/each}
    </div>
  {/if}

  <div
    role="separator"
    aria-orientation="vertical"
    class="absolute inset-y-0 -right-0.5 z-10 w-1 cursor-col-resize transition-colors {resizing
      ? 'bg-accent'
      : 'hover:bg-accent/50'}"
    onpointerdown={(e) => {
      resizing = true;
      e.currentTarget.setPointerCapture(e.pointerId);
    }}
    onpointermove={onResize}
    onpointerup={() => (resizing = false)}
  ></div>
</aside>
