<script lang="ts">
  import type { Snippet } from "svelte";
  import { studio } from "$lib/studio.svelte";
  import BottomPanel from "./BottomPanel.svelte";

  let { children, active = true }: { children: Snippet; active?: boolean } = $props();
</script>

<div class="flex min-h-0 min-w-0 flex-1 flex-col">
  <div class="flex min-h-0 flex-1 flex-col overflow-hidden">
    {@render children()}
  </div>
  {#if active && studio.terminalOpen && !studio.chatExpanded}<BottomPanel />{/if}
</div>
