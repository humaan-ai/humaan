<script lang="ts">
  import { onMount } from 'svelte';
  import { thumbnail } from '$lib/builder/thumbnail';
  import type { Robot } from '$lib/builder/model';
  let { name, load, compact = false }: { name: string; load: () => Promise<Robot>; compact?: boolean } = $props();
  let host: HTMLDivElement;
  let image = $state(''), failed = $state(false), empty = $state(false);
  onMount(() => {
    let disposed = false;
    const observer = new IntersectionObserver(entries => {
      if (!entries.some(entry => entry.isIntersecting)) return;
      observer.disconnect();
      void load().then(robot => {
        if (disposed) return;
        if (!robot.links.some(link => link.visuals.length)) empty = true;
        else image = thumbnail(robot);
      }).catch(() => { if (!disposed) failed = true; });
    }, { rootMargin: '100px' });
    observer.observe(host);
    return () => { disposed = true; observer.disconnect(); };
  });
</script>
<div bind:this={host} class="flex w-full items-center justify-center bg-background text-xs text-muted {compact ? 'h-16' : 'h-36'}">
  {#if image}<img src={image} alt={`3D preview of ${name}`} class="h-full w-full object-contain" />{:else}<span class="px-2 text-center text-[10px]">{failed ? 'Preview unavailable' : empty ? 'No visual geometry' : 'Loading preview…'}</span>{/if}
</div>
