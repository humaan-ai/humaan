<script lang="ts">
  import type { SVGAttributes } from 'svelte/elements';

  let { size = 24, strokeWidth = 1.75, ...attributes }: SVGAttributes<SVGSVGElement> & {
    size?: number;
    strokeWidth?: number;
  } = $props();
</script>

<svg xmlns="http://www.w3.org/2000/svg" width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width={strokeWidth} stroke-linecap="round" stroke-linejoin="round" aria-hidden="true" {...attributes}>
  <path d="M3 21h12M6 21v-3h6v3M8 13l-2-3M7 7l7-2M17.5 6.5 20 9M18 10v3l1 1M22 10v3l-1 1" />
  <circle cx="9" cy="15" r="3" />
  <circle cx="5" cy="8" r="2" />
  <circle cx="16" cy="5" r="2" />
</svg>
