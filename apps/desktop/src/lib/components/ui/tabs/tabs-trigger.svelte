<script lang="ts">
	import { Tabs as TabsPrimitive } from "bits-ui";
	import { cn } from "$lib/utils.js";

	let {
		ref = $bindable(null),
		class: className,
		...restProps
	}: TabsPrimitive.TriggerProps = $props();
</script>

<TabsPrimitive.Trigger
	bind:ref
	data-slot="tabs-trigger"
	class={cn(
		"inline-flex h-8 shrink-0 items-center justify-center gap-2 rounded-md px-3 text-xs text-muted hover:bg-hover hover:text-title data-[state=active]:bg-selection data-[state=active]:text-title outline-none focus-visible:ring-2 focus-visible:ring-ring disabled:opacity-45",
		className
	)}
	{...restProps}
/>
