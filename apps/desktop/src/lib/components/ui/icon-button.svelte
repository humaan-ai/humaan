<script lang="ts">
  import { Button, type ButtonProps } from "./button/index.js";
  let { ref = $bindable(null), children, ...props }: ButtonProps & { "aria-label": string } = $props();
</script>

<Button variant="ghost" size="icon-sm" bind:ref {...props}>
  {@render children?.()}
</Button>
