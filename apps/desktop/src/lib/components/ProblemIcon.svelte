<script lang="ts">
  import CircleX from '@lucide/svelte/icons/circle-x';
  import TriangleAlert from '@lucide/svelte/icons/triangle-alert';
  import Info from '@lucide/svelte/icons/info';
  import type { ProblemSeverity } from '$lib/problems.svelte';
  let { severity, muted = false }: { severity: ProblemSeverity; muted?: boolean } = $props();
  const label = $derived(severity === 'info' ? 'Information' : severity === 'warning' ? 'Warning' : 'Error');
</script>

<span role="img" aria-label={label} title={label} class="inline-flex h-4 shrink-0 items-center {muted ? 'text-muted' : severity === 'error' ? 'text-error' : severity === 'warning' ? 'text-warning' : 'text-link'}">
  {#if severity === 'error'}<CircleX size={14} aria-hidden="true" />{:else if severity === 'warning'}<TriangleAlert size={14} aria-hidden="true" />{:else}<Info size={14} aria-hidden="true" />{/if}
</span>
