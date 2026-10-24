<script lang="ts" module>
	import { type VariantProps, tv } from "tailwind-variants";
	import { cn, type WithElementRef } from "$lib/utils.js";
	import type { HTMLAnchorAttributes, HTMLButtonAttributes } from "svelte/elements";

	export const buttonVariants = tv({
		base: "inline-flex shrink-0 items-center justify-center gap-2 whitespace-nowrap rounded-md border border-transparent text-xs font-medium transition-colors outline-none select-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background aria-invalid:border-destructive disabled:cursor-default disabled:opacity-45 [&_svg]:shrink-0",
		variants: {
			variant: {
                default: "bg-primary text-primary-foreground hover:bg-accent-hover",
                outline: "border-border-strong bg-raised text-foreground hover:bg-selection",
                secondary: "bg-selection text-title hover:bg-hover",
                ghost: "text-muted hover:bg-hover hover:text-title aria-selected:bg-selection aria-selected:text-title",
                destructive: "border-error/40 bg-error/10 text-error hover:bg-error/20",
                link: "text-link underline-offset-4 hover:underline",
            },
            size: {
                default: "h-8 px-3",
                xs: "h-6 gap-1 px-2",
                sm: "h-7 gap-1.5 px-2.5",
                lg: "h-9 px-4",
                icon: "size-8 p-0",
                "icon-xs": "size-6 p-0",
                "icon-sm": "size-7 p-0",
                "icon-lg": "size-10 p-0",
            },
		},
		defaultVariants: {
			variant: "outline",
			size: "default",
		},
	});

	export type ButtonVariant = VariantProps<typeof buttonVariants>["variant"];
	export type ButtonSize = VariantProps<typeof buttonVariants>["size"];

	export type ButtonProps = WithElementRef<HTMLButtonAttributes> &
		WithElementRef<HTMLAnchorAttributes> & {
			variant?: ButtonVariant;
			size?: ButtonSize;
		};
</script>

<script lang="ts">
	let {
		class: className,
		variant = "outline",
		size = "default",
		ref = $bindable(null),
		href = undefined,
		type = "button",
		disabled,
		children,
		...restProps
	}: ButtonProps = $props();
</script>

{#if href}
	<a
		bind:this={ref}
		data-slot="button"
		class={cn(buttonVariants({ variant, size }), className)}
		href={disabled ? undefined : href}
		aria-disabled={disabled}
		role={disabled ? "link" : undefined}
		tabindex={disabled ? -1 : undefined}
		{...restProps}
	>
		{@render children?.()}
	</a>
{:else}
	<button
		bind:this={ref}
		data-slot="button"
		class={cn(buttonVariants({ variant, size }), className)}
		{type}
		{disabled}
		{...restProps}
	>
		{@render children?.()}
	</button>
{/if}
