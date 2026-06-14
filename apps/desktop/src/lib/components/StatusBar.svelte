<script lang="ts">
  import { layout } from "$lib/layout.svelte";
  import Zap from "@lucide/svelte/icons/zap";
  import GitBranch from "@lucide/svelte/icons/git-branch";
  import CircleX from "@lucide/svelte/icons/circle-x";
  import TriangleAlert from "@lucide/svelte/icons/triangle-alert";
  import Bell from "@lucide/svelte/icons/bell";
</script>

<footer class="flex h-[22px] shrink-0 items-center justify-between border-t border-border bg-surface text-[11px] text-foreground/80 select-none">
  <div class="flex h-full items-center">
    <div class="flex h-full items-center gap-1 bg-accent px-2 text-white" title="Humaan Kernel">
      <Zap size={12} strokeWidth={2} />
      kernel
    </div>

    <div class="flex h-full items-center gap-1 px-2 hover:bg-hover" title="Git branch">
      <GitBranch size={12} strokeWidth={1.8} />
      main
    </div>

    <div class="flex h-full items-center gap-1 px-2 hover:bg-hover" title="No problems">
      <CircleX size={12} strokeWidth={1.8} />
      0
      <TriangleAlert size={12} strokeWidth={1.8} />
      0
    </div>
  </div>

  <div class="flex h-full items-center">
    <button
      class="flex h-full items-center px-2 hover:bg-hover"
      onclick={() => {
        layout.bottomTab = "debug";
        layout.bottomPanelOpen = true;
      }}
    >
      Debug Console
    </button>
    <span class="flex h-full items-center px-2 hover:bg-hover">UTF-8</span>
    <span class="flex h-full items-center px-2 hover:bg-hover">Tauri</span>
    <span class="flex h-full items-center px-2 hover:bg-hover" title="Notifications">
      <Bell size={12} strokeWidth={1.8} />
    </span>
  </div>
</footer>
