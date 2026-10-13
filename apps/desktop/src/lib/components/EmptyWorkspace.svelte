<script lang="ts">
  import type { StudioView } from "$lib/studio.svelte";
  import WorkspaceColumn from "./WorkspaceColumn.svelte";
  import StudioSidebar from "./StudioSidebar.svelte";

  let { view }: { view: Exclude<StudioView, "experiences" | "loop"> } = $props();
  const titles = { project: "Project", builder: "Builder", simulation: "Simulation", deployments: "Deployments", cloud: "Cloud resources" };
</script>

<div class="flex min-h-0 flex-1 gap-1">
  <StudioSidebar title={titles[view]}>
    <div class="min-h-0 flex-1"></div>
  </StudioSidebar>
  <WorkspaceColumn>
  <div class="workbench-panel min-w-0 flex-1" role="region" aria-label={`${titles[view]} workspace`}></div>
  </WorkspaceColumn>
</div>
