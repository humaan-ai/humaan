<script lang="ts">
  import LoaderCircle from "@lucide/svelte/icons/loader-circle";
  let { active, label, pending }: { active: boolean; label: string; pending: string } = $props();
</script>

<span class="inline-grid items-center justify-items-center align-middle">
  <span class="col-start-1 row-start-1" class:invisible={active} aria-hidden={active}>{label}</span>
  <span class="col-start-1 row-start-1 inline-flex items-center gap-1.5" class:invisible={!active} aria-hidden={!active}>
    <LoaderCircle size={14} class={active ? "animate-spin motion-reduce:animate-none" : ""} aria-hidden="true" />{pending}
  </span>
</span>
