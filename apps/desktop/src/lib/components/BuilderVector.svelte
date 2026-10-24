<script lang="ts">
  import { Input } from '$lib/components/ui/input/index.js';
  let { label, value = $bindable(), axes = ['X', 'Y', 'Z'] }: { label: string; value: number[]; axes?: string[] } = $props();
</script>
<fieldset class="min-w-0"><legend class="mb-1 max-w-full text-xs text-muted">{label}</legend><div class="grid grid-cols-3 gap-1">
  {#each value as _, i}<Input type="number" step="any" aria-label={`${label} ${axes[i] || i + 1}`} title={axes[i] || String(i + 1)} value={value[i]} oninput={(event) => { value[i] = event.currentTarget.valueAsNumber; }} class="min-w-0 px-2" />{/each}
</div></fieldset>
