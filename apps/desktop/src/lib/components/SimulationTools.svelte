<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { toast } from 'svelte-sonner';
  import { studio } from '$lib/studio.svelte';
  import { Button } from '$lib/components/ui/button/index.js';
  import { Input } from '$lib/components/ui/input/index.js';
  import ActionLabel from './ActionLabel.svelte';
  type Status = { ready: boolean; installing: boolean; phase: string; version: string; python: string | null; error: string | null };
  let status = $state<Status | null>(null), checking = $state(false), installing = $state(false), error = $state('');
  let alive = false;
  async function refresh() {
    if (checking) return;
    checking = true;
    try { const result = await invoke<Status>('simulation_tools_status'); studio.simulationRuntime = result; if (alive) { status = result; error = ''; } }
    catch (cause) { if (alive) error = String(cause); }
    finally { checking = false; }
  }
  function useManaged() {
    localStorage.removeItem('humaan-simulation-python');
    localStorage.setItem('humaan-simulation-runtime-mode', 'managed');
    studio.simulationPython = '';
  }
  async function install() {
    if (installing || status?.installing) return;
    installing = true; error = '';
    try {
      const result = await invoke<Status>('simulation_tools_install');
      studio.simulationRuntime = result;
      if (alive) status = result;
      if (result.ready) { useManaged(); toast.success('MuJoCo is ready. Simulation will use the installed runtime.', { id: 'mujoco-install' }); }
    } catch (cause) { if (alive) error = String(cause); toast.error('MuJoCo installation failed. Retry in Settings → Tools.', { id: 'mujoco-install' }); }
    finally { installing = false; }
  }
  onMount(() => {
    alive = true; void refresh();
    const timer = setInterval(() => { if (installing || status?.installing) void refresh(); }, 2000);
    return () => { alive = false; clearInterval(timer); };
  });
</script>

<section class="mt-6 rounded-lg border border-border-strong p-4" aria-label="MuJoCo tools" aria-busy={installing || status?.installing}>
  <h4 class="text-sm font-medium">MuJoCo</h4>
  <p class="mt-1 text-xs leading-relaxed text-muted">Local robot physics simulation. Install Python and MuJoCo 3.2.7 in an isolated Humaan environment, shared across your projects.</p>
  <p class="mt-3 text-xs" role="status">{installing && !status?.installing ? 'Starting MuJoCo installation…' : status?.phase || 'Checking MuJoCo…'}</p>
  {#if !status?.ready}<p class="mt-2 text-xs text-muted">One-time setup needs an internet connection and may take several minutes. Your system Python stays unchanged.</p>{/if}
  {#if status?.ready}<p class="mt-2 text-xs text-muted">{studio.simulationPython ? 'Simulation currently uses a custom Python executable.' : 'Simulation uses the Humaan-managed runtime automatically.'}</p>{/if}
  <div class="mt-3 flex flex-wrap gap-2">
    <Button disabled={checking || installing || status?.installing} onclick={install}><ActionLabel active={installing || !!status?.installing} label={status?.ready ? 'Repair MuJoCo' : 'Install MuJoCo'} pending="Installing…" /></Button>
    <Button disabled={checking || installing} onclick={refresh}><ActionLabel active={checking} label="Refresh MuJoCo status" pending="Checking…" /></Button>
    {#if status?.ready && studio.simulationPython}<Button disabled={installing || status.installing} onclick={() => { useManaged(); toast.success('Simulation now uses the managed MuJoCo runtime.'); }}>Use installed MuJoCo</Button>{/if}
  </div>
      <details class="mt-4 border-t border-border pt-3 text-xs text-muted"><summary class="cursor-pointer">Advanced runtime</summary>
        <label class="mt-2 flex flex-col gap-1">Custom Python executable<Input aria-label="Simulation Python executable" placeholder="Managed MuJoCo (Settings → Tools)" bind:value={studio.simulationPython} disabled={installing || status?.installing || studio.simulationBusy || studio.trainingBusy} onchange={() => { localStorage.setItem('humaan-simulation-runtime-mode', studio.simulationPython.trim() ? 'custom' : 'managed'); if (studio.simulationPython.trim()) localStorage.setItem('humaan-simulation-python', studio.simulationPython.trim()); else localStorage.removeItem('humaan-simulation-python'); }} /></label>
        <p class="mt-2">Leave blank to use MuJoCo installed through Settings → Tools. Enter an absolute Python path only to use your own environment.</p>
      </details>
  {#if error || status?.error}<p class="mt-3 whitespace-pre-wrap break-words text-xs text-error" role="alert">{error || status?.error}</p>{/if}
</section>
