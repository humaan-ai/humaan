<script lang="ts">
  import "../app.css";
  import TitleBar from "$lib/components/TitleBar.svelte";
  import StatusBar from "$lib/components/StatusBar.svelte";

  let { children } = $props();
</script>

<div class="flex h-screen flex-col overflow-hidden">
  <TitleBar />
  <main class="min-h-0 min-w-0 flex-1 overflow-auto">
    {@render children()}
  </main>
  <StatusBar />
</div>
