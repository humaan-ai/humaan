<script lang="ts">
  import FolderOpen from "@lucide/svelte/icons/folder-open";
  import FolderPlus from "@lucide/svelte/icons/folder-plus";

  let {
    busy,
    onopen,
    oncreate,
  }: {
    busy: boolean;
    onopen: () => Promise<void>;
    oncreate: () => void;
  } = $props();

  const actionClass =
    "flex min-h-[64px] w-full flex-col items-start justify-center gap-1 rounded-lg bg-[#242424] px-5 py-4 text-left text-xs font-medium leading-6 text-white transition-colors enabled:hover:bg-[#2c2c2c] enabled:active:bg-selection focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-accent disabled:cursor-default disabled:opacity-45";
</script>

<div class="flex flex-1 items-center justify-center px-6 py-12">
  <div class="w-full max-w-lg">
    <div class="flex flex-col items-center text-center">
      <h1 class="m-0">
        <img
          class="block h-auto max-w-full"
          src="/humaan-studio-logo.svg"
          alt="Humaan Studio"
          width="420"
          height="240"
        />
      </h1>
      <p class="max-w-sm text-sm leading-relaxed text-muted">
        A local workbench to inspect, review, and curate robot experiences for
        learning.
      </p>
    </div>
    <div class="mt-7 grid grid-cols-1 gap-4 sm:grid-cols-2">
      <button
        class={actionClass}
        disabled={busy}
        aria-haspopup="dialog"
        onclick={oncreate}
      >
        <FolderPlus size={18} strokeWidth={1.5} aria-hidden="true" />
        <span>Create new</span>
      </button>
      <button class={actionClass} disabled={busy} onclick={onopen}>
        <FolderOpen size={18} strokeWidth={1.5} aria-hidden="true" />
        <span>Open project</span>
      </button>
    </div>
  </div>
</div>
