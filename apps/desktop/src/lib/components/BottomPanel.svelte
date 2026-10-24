<script lang="ts">
  import IconButton from "$lib/components/ui/icon-button.svelte";
  import X from "@lucide/svelte/icons/x";
  import { studio } from "$lib/studio.svelte";
  import { problemGroups, problemCounts, type Problem } from "$lib/problems.svelte";
  import * as Tabs from "$lib/components/ui/tabs/index.js";
  import ProblemIcon from './ProblemIcon.svelte';

  const counts = $derived(problemCounts());
  const groups = $derived(Object.entries(problemGroups).filter(([, group]) => group.items.length));

  let drag: { y: number; height: number } | null = null;
  function resize(height: number) {
    studio.terminalHeight = Math.max(100, Math.min(500, height));
  }
  function close() {
    studio.terminalOpen = false;
    document.getElementById("terminal-toggle")?.focus();
  }
</script>

<!-- A focusable separator is an interactive window splitter with keyboard resizing. -->
<!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
<div role="separator" aria-label="Resize bottom panel" aria-controls="bottom-panel" aria-orientation="horizontal" aria-valuemin={100} aria-valuemax={500} aria-valuenow={studio.terminalHeight} tabindex={0}
  class="flex h-1 shrink-0 cursor-row-resize touch-none items-center justify-center gap-0.5 focus-visible:outline focus-visible:outline-accent"
  onpointerdown={(event) => { if (event.button !== 0) return; drag = { y: event.clientY, height: studio.terminalHeight }; event.currentTarget.setPointerCapture(event.pointerId); event.preventDefault(); }}
  onpointermove={(event) => { if (drag) resize(drag.height + drag.y - event.clientY); }}
  onpointerup={() => drag = null} onpointercancel={() => drag = null} onlostpointercapture={() => drag = null}
  onkeydown={(event) => { if (["ArrowUp", "ArrowDown", "Home", "End"].includes(event.key)) { event.preventDefault(); resize(event.key === "Home" ? 100 : event.key === "End" ? 500 : studio.terminalHeight + (event.key === "ArrowUp" ? 10 : -10)); } }}>
  {#each [0, 1, 2] as dot}<span class="h-0.5 w-0.5 rounded-full bg-muted" aria-hidden="true"></span>{/each}
</div>
{#snippet problemRow(problem: Problem)}
  <ProblemIcon severity={problem.severity} />
  <span class="min-w-0 flex-1 break-words leading-relaxed">{problem.message}<span class="ml-2 text-[10px] text-muted">{problem.source}</span></span>
{/snippet}

<section id="bottom-panel" aria-label="Bottom panel" class="workbench-panel flex min-h-0 shrink-0 flex-col overflow-hidden" style:height={`${studio.terminalHeight}px`} style:max-height="60%">
  <Tabs.Root bind:value={studio.bottomPanelTab} class="min-h-0 flex-1 gap-0">
    <header class="flex h-7 shrink-0 items-center gap-2 px-2">
      <Tabs.List aria-label="Bottom panel views" class="h-6 gap-1 p-0 group-data-[orientation=horizontal]/tabs:h-6">
        <Tabs.Trigger id="problems-tab" value="problems" aria-label={`Problems (${counts.total})`} class="h-6 gap-1.5 px-2.5 text-xs font-medium">Problems<span class="rounded-full bg-hover px-1.5 text-[10px] tabular-nums">{counts.total}</span></Tabs.Trigger>
        <Tabs.Trigger value="terminal" class="h-6 px-2.5 text-xs font-medium">Terminal</Tabs.Trigger>
      </Tabs.List>
      <IconButton class="ml-auto size-6" aria-label="Close bottom panel" title="Close bottom panel" onclick={close}><X size={14} aria-hidden="true" /></IconButton>
    </header>
    <Tabs.Content value="problems" class="min-h-0 overflow-auto text-xs">
      {#each groups as [key, group] (key)}
        <details open class="border-b border-border last:border-0">
          <summary class="cursor-pointer px-3 py-2 text-title"><span class="font-medium">{group.label}</span><span class="ml-2 rounded-full bg-selection px-1.5 py-px text-[10px] tabular-nums">{group.items.length}</span>{#if group.path}<span class="ml-2 break-all text-[10px] text-muted">{group.path}</span>{/if}</summary>
          <ul class="pb-1" aria-label={group.label}>
            {#each group.items as problem (problem.id)}
              <li>
                {#if problem.reveal}
                  <button class="flex w-full items-start gap-2 px-5 py-1 text-left hover:bg-hover focus-visible:bg-selection focus-visible:outline-1 focus-visible:outline-ring focus-visible:-outline-offset-1" title="Open in Builder" onclick={problem.reveal}>{@render problemRow(problem)}</button>
                {:else}
                  <div class="flex items-start gap-2 px-5 py-1">{@render problemRow(problem)}</div>
                {/if}
              </li>
            {/each}
          </ul>
        </details>
      {:else}
        <p class="px-4 py-3 text-muted">No problems reported.</p>
      {/each}
    </Tabs.Content>
    <Tabs.Content value="terminal" class="min-h-0 overflow-auto px-4 py-2 font-mono text-xs text-muted">No terminal session connected.</Tabs.Content>
  </Tabs.Root>
</section>
