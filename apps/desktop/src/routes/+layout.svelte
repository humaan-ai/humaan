<script lang="ts">
  import "../app.css";
  import { Toaster } from "svelte-sonner";
  import { Provider as TooltipProvider } from "$lib/components/ui/tooltip/index.js";
  import TitleBar from "$lib/components/TitleBar.svelte";
  import StatusBar from "$lib/components/StatusBar.svelte";

  let { children } = $props();
</script>

<TooltipProvider delayDuration={400}>
<div class="flex h-screen flex-col overflow-hidden">
  <TitleBar />
  <div class="relative flex min-h-0 flex-1 gap-1 bg-surface px-1 pb-1">
    <main class="min-h-0 min-w-0 flex-1 overflow-hidden">
      {@render children()}
    </main>
  </div>
  <StatusBar />
  <Toaster theme="dark" position="bottom-right" offset="36px" closeButton duration={4000} toastOptions={{ style: 'background: var(--color-surface); color: var(--color-foreground); border-color: var(--color-border); font-size: 12px;' }} />
</div>
</TooltipProvider>
