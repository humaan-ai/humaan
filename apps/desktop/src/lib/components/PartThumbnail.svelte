<script lang="ts">
  import { onMount } from 'svelte';
  import { thumbnail } from '$lib/builder/thumbnail';
  import type { Robot } from '$lib/builder/model';
  let { name, load }: { name: string; load: () => Promise<Robot> } = $props();
  let host: HTMLDivElement;
  let image = $state(''), failed = $state(false);
  onMount(() => {
    let disposed = false;
    const observer = new IntersectionObserver(entries => {
      if (!entries.some(entry => entry.isIntersecting)) return;
      observer.disconnect();
      void load().then(robot => { if (!disposed) image = thumbnail(robot); }).catch(() => { if (!disposed) failed = true; });
    }, { rootMargin: '100px' });
    observer.observe(host);
    return () => { disposed = true; observer.disconnect(); };
  });
</script>
<div bind:this={host} class="flex h-36 w-full items-center justify-center bg-[#181818] text-xs text-muted">
  {#if image}<img src={image} alt={`3D preview of ${name}`} class="h-full w-full object-contain" />{:else}<span>{failed ? 'Preview unavailable' : 'Loading preview…'}</span>{/if}
</div>
