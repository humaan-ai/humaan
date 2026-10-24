<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { toast } from 'svelte-sonner';
  import { Button } from '$lib/components/ui/button/index.js';
  import { Input } from '$lib/components/ui/input/index.js';
  import { NativeSelect } from '$lib/components/ui/native-select/index.js';
  import * as Dialog from '$lib/components/ui/dialog/index.js';
  import ArrowLeft from '@lucide/svelte/icons/arrow-left';
  import ArrowUpRight from '@lucide/svelte/icons/arrow-up-right';
  import LayoutGrid from '@lucide/svelte/icons/layout-grid';
  import Server from '@lucide/svelte/icons/server';
  import RefreshCw from '@lucide/svelte/icons/refresh-cw';
  import Plus from '@lucide/svelte/icons/plus';
  import Download from '@lucide/svelte/icons/download';
  import Square from '@lucide/svelte/icons/square';
  import StudioSidebar from './StudioSidebar.svelte';
  import WorkspaceColumn from './WorkspaceColumn.svelte';
  import { studio } from '$lib/studio.svelte';
  import type { Catalog } from '$lib/training/model';
  type Config = { countryCode: string; gpuTypeId: string; shutdown: string; minutes: number };
  type Account = { accountId: string; gpus: { id: string; displayName: string; memoryInGb: number; secureCloud: boolean }[] };
  type Quote = { id: string; expiresAt: number; accountId: string; experimentName: string; hourlyUsd: number; estimatedComputeUsd: number; imageName: string; config: Config };
  type Job = { id: string; name: string; createdAt: number; experimentId: string; status: string; instanceId?: string; instanceState: string; config: Config; accountId: string; error?: string; logs?: string; retrieved?: boolean; lastCheckedAt?: number; remote?: { status: string; error?: string; updatedAt: number; elapsedSeconds: number; gpu?: string; progress?: { iteration: number; loss: number }; evaluation?: { successRate: number; meanError: number } } };
  let { root }: { root: string } = $props();
  let config = $state<Config>({ countryCode: '', gpuTypeId: '', shutdown: 'stop', minutes: 30 });
  let provider = $state<'runpod' | 'brev' | null>(null);
  function chooseProvider(next: 'runpod' | 'brev' | null) { provider = next; selected = null; apiKey = ''; error = ''; }
  let apiKey = $state('');
  let account = $state<Account | null>(studio.cloudAccount);
  async function connect() { await work('Connecting RunPod', async () => { account = await invoke<Account>('cloud_connect', { apiKey }); studio.cloudAccount = account; apiKey = ''; if (!config.gpuTypeId) config.gpuTypeId = account.gpus.find(g => g.secureCloud)?.id ?? ''; }); }
  let jobs = $state<Job[]>([]), experiments = $state<Catalog['experiments']>([]), experimentId = $state('');
  let selected = $state<Job | null>(null), quote = $state<Quote | null>(null), reviewOpen = $state(false), releaseOpen = $state(false);
  let busy = $state(''), error = $state(''), acknowledged = $state(false), monitoring = $state(true);
  let tab = $state<'overview' | 'logs'>('overview');
  function update(job: Job) { selected = job; jobs = [job, ...jobs.filter(j => j.id !== job.id)].sort((a,b) => b.createdAt-a.createdAt); }
  async function work(label: string, action: () => Promise<void>, quiet = false) {
    if (busy) return; busy = label;
    try { await action(); error = ''; } catch (cause) { error = String(cause); if (!quiet) toast.error(error); } finally { busy = ''; }
  }
  async function refresh() { const [next, catalog] = await Promise.all([invoke<Job[]>('cloud_jobs', { root }), invoke<Catalog>('training_catalog', { root })]); jobs = next; experiments = catalog.experiments; if (!experimentId) experimentId = experiments.find(e => e.id === studio.cloudExperimentId)?.id ?? experiments[0]?.id ?? ''; studio.cloudExperimentId = ''; if (selected) selected = jobs.find(j => j.id === selected!.id) ?? null; }
  async function estimate() { await work('Checking account and price', async () => { quote = await invoke<Quote>('cloud_quote', { root, config: $state.snapshot(config), experimentId }); acknowledged = false; reviewOpen = true; localStorage.setItem('humaan-runpod-config', JSON.stringify(config)); }); }
  async function launch() { if (!quote || !acknowledged) return; await work('Provisioning', async () => { const job = await invoke<Job>('cloud_launch', { root, quoteId: quote!.id }); update(job); reviewOpen = false; quote = null; toast.info(job.error ? 'Launch needs attention. Refresh before creating another job.' : 'Cloud job submitted'); }); }
  async function action(action: 'refresh' | 'retrieve' | 'release' | 'stop', quiet = false) { if (!selected) return; const id = selected.id; await work(action === 'refresh' ? 'Refreshing' : action === 'retrieve' ? 'Retrieving results' : 'Releasing compute', async () => { update(await invoke<Job>('cloud_job_action', { root, id, action })); if (action === 'retrieve') toast.success('Policy, evaluation, and replay saved in Training experiments'); if (action === 'stop') toast.info('Stop requested. Refresh to confirm.'); if (action === 'release') { releaseOpen = false; toast.info('Termination requested. Refresh to confirm cleanup.'); } }, quiet); }
  onMount(() => { try { const saved = JSON.parse(localStorage.getItem('humaan-runpod-config') ?? 'null'); if (saved) config = { ...config, ...saved }; } catch { /* keep defaults */ } void work('Loading', refresh); const timer = setInterval(() => { if (account && monitoring && selected && studio.activeView === 'cloud' && !busy && !reviewOpen && !releaseOpen) void action('refresh', true); }, 15000); return () => clearInterval(timer); });
</script>

<div class="flex h-full min-h-0 gap-1">
  <StudioSidebar title="Compute">
    <Button variant="ghost" class="mx-2 mt-2 justify-start {provider === null && !selected ? 'bg-selection text-title' : 'text-muted'}" disabled={!!busy} onclick={() => chooseProvider(null)}><LayoutGrid size={14}/>Providers</Button>
    {#if provider === 'runpod'}
    <details class="border-b border-border p-3 text-xs" open={!account}><summary class="cursor-pointer">{account ? 'Connected · RunPod' : 'Connect RunPod'}</summary><form class="mt-3 space-y-2" onsubmit={e => { e.preventDefault(); connect(); }}><Input type="password" aria-label="RunPod API key" placeholder="RunPod API key" bind:value={apiKey} required autocomplete="off"/><Button type="submit" disabled={!!busy}>Connect account</Button><p class="text-muted">API key stays in memory for this app session. Reconnect after restarting Humaan.</p>{#if account}<p class="break-all text-muted">{account.accountId}</p>{/if}</form></details>
    {/if}
    <div class="flex items-center justify-between border-b border-border px-3 py-2 text-xs text-muted"><span>RunPod jobs · {jobs.length}</span><Button size="icon-sm" variant="ghost" aria-label="Refresh Compute jobs" disabled={!!busy} onclick={() => work('Loading', refresh)}><RefreshCw size={14}/></Button></div>
    <Button variant="ghost" class="m-2 justify-start" disabled={!!busy} onclick={() => chooseProvider(null)}><Plus size={14}/>New cloud job</Button>
    <div class="min-h-0 flex-1 overflow-auto">{#each jobs as job}<button class="flex w-full flex-col gap-1 px-4 py-2 text-left text-xs hover:bg-hover" class:bg-selection={selected?.id === job.id} disabled={!!busy} onclick={() => { provider = 'runpod'; selected = job; tab = 'overview'; error = ''; }}><span class="truncate text-title">{job.name}</span><span class="text-muted">{job.status} · {job.instanceState}</span></button>{/each}</div>
  </StudioSidebar>
  <WorkspaceColumn active={studio.activeView === 'cloud'}>
    <section class="workbench-panel relative flex min-h-0 flex-1 flex-col" aria-label="Compute workspace">
      <header class="flex h-[35px] shrink-0 items-center gap-2 border-b border-border px-3 text-xs"><Server size={15}/><span class="font-medium text-title">{selected ? selected.name : 'Compute'}</span>{#if provider}<Button class="ml-auto h-6 text-xs text-muted" size="sm" variant="ghost" disabled={!!busy} onclick={() => chooseProvider(null)}><ArrowLeft size={12}/>Change provider</Button>{/if}</header>
      {#if error}<div role="alert" class="border-b border-border px-4 py-3 text-xs text-red-400">{error}</div>{/if}
      {#if busy}<div role="status" class="absolute left-1/2 top-12 z-10 -translate-x-1/2 rounded-md border border-border bg-surface px-4 py-2 text-xs shadow-lg">{busy}…</div>{/if}
      {#if !provider}
        <div class="flex min-h-0 flex-1 overflow-auto p-6 sm:p-10">
          <div class="m-auto w-full max-w-3xl py-8">
            <p class="text-xs font-medium uppercase tracking-widest text-muted">Cloud compute</p>
            <h1 class="mt-3 text-2xl font-medium tracking-tight text-title">Choose your provider</h1>
            <p class="mt-3 max-w-lg text-sm leading-relaxed text-muted">Connect your cloud account to run experiments and manage compute from Humaan.</p>
            <div class="mt-8 grid gap-4 sm:grid-cols-2" aria-label="Cloud providers">
              <button type="button" class="group flex min-h-56 flex-col rounded-lg border border-border bg-background p-6 text-left transition-colors hover:bg-hover focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ring disabled:opacity-50" disabled={!!busy} onclick={() => chooseProvider('runpod')} aria-label="Select RunPod">
                <div class="flex h-10 w-full items-center justify-between gap-4"><img src="/providers/runpod.svg" alt="RunPod" class="h-8 w-36 object-contain object-left"/><span class="rounded border border-border px-2 py-1 text-[10px] text-muted">{account ? 'Connected' : 'Available'}</span></div>
                <p class="mt-6 text-sm font-medium text-title">RunPod</p><p class="mt-2 text-xs leading-relaxed text-muted">On-demand GPU pods for your training experiments.</p>
                <span class="mt-6 flex items-center justify-between text-xs text-title">{account ? 'Configure a job' : 'Connect account'}<ArrowUpRight size={16}/></span>
              </button>
              <button type="button" class="group flex min-h-56 flex-col rounded-lg border border-border bg-background p-6 text-left transition-colors hover:bg-hover focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ring disabled:opacity-50" disabled={!!busy} onclick={() => chooseProvider('brev')} aria-label="Select NVIDIA Brev">
                <div class="flex h-10 w-full items-center justify-between gap-4"><div class="flex items-center gap-3"><img src="/providers/nvidia.svg" alt="NVIDIA" class="h-9 w-10 object-contain"/><span class="text-lg font-semibold tracking-tight text-title">Brev</span></div><span class="whitespace-nowrap rounded border border-border px-2 py-1 text-[10px] text-muted">Coming soon</span></div>
                <p class="mt-6 text-sm font-medium text-title">NVIDIA Brev</p><p class="mt-2 text-xs leading-relaxed text-muted">GPU development environments across cloud providers.</p>
                <span class="mt-6 flex items-center justify-between text-xs text-muted">View integration status<ArrowUpRight size={16}/></span>
              </button>
            </div>
            <p class="mt-5 text-xs text-muted">Selecting a provider does not create resources or start billing.</p>
          </div>
        </div>
      {:else if provider === 'brev'}
        <div class="flex min-h-0 flex-1 items-center justify-center overflow-auto p-8"><div class="max-w-md text-center"><img src="/providers/nvidia.svg" alt="NVIDIA" class="mx-auto h-14 w-16"/><h1 class="mt-6 text-xl font-medium text-title">NVIDIA Brev</h1><p class="mt-3 text-xs text-muted">Coming soon</p><p class="mt-4 text-sm leading-relaxed text-muted">Brev account connection, provisioning, and job monitoring are planned. Choose RunPod to launch a cloud experiment today.</p><Button class="mt-6" variant="outline" onclick={() => chooseProvider(null)}><ArrowLeft size={14}/>Back to providers</Button></div></div>
      {:else if selected}
        <div class="flex min-h-[57px] shrink-0 items-center gap-3 border-b border-border px-4 py-3">
          <Button variant="outline" class={tab === 'overview' ? 'bg-selection text-title' : ''} onclick={() => tab = 'overview'}>Overview</Button>
          <Button variant="outline" class={tab === 'logs' ? 'bg-selection text-title' : ''} onclick={() => tab = 'logs'}>Logs</Button>
          <label class="ml-auto flex items-center gap-2 text-xs text-muted"><input type="checkbox" bind:checked={monitoring}/>Monitor</label>
          <Button variant="outline" size="icon-sm" aria-label="Refresh cloud job" disabled={!!busy || !account} onclick={() => action('refresh')}><RefreshCw size={14}/></Button>
        </div>
        <div class="min-h-0 flex-1 overflow-auto p-5">
          {#if tab === 'logs'}<pre class="whitespace-pre-wrap break-words rounded-md bg-background p-4 font-mono text-xs leading-5">{selected.logs || 'Logs will appear after the remote worker starts. Refresh to reconnect.'}</pre>
          {:else}
            <div class="flex flex-wrap gap-2 text-xs"><span class="rounded border border-border bg-selection px-2 py-1">Job · {selected.status}</span><span class="rounded border border-border px-2 py-1">Compute · {selected.instanceState}</span>{#if selected.retrieved}<span class="rounded border border-border px-2 py-1">Results saved locally</span>{/if}</div>
            <dl class="mt-5 grid max-w-3xl grid-cols-[140px_1fr] gap-x-4 gap-y-3 text-xs"><dt class="text-muted">Account / region</dt><dd>{selected.accountId} · {selected.config.countryCode || 'Any region'}</dd><dt class="text-muted">Instance</dt><dd>{selected.instanceId || 'Awaiting instance discovery'} · {selected.config.gpuTypeId}</dd><dt class="text-muted">Runtime</dt><dd>MuJoCo 3.2.7 · NumPy 2.0.2 · CPU training on GPU instance</dd><dt class="text-muted">Runtime limit</dt><dd>{selected.config.minutes} minutes · {selected.config.shutdown} at deadline</dd><dt class="text-muted">Experiment</dt><dd class="break-all">{selected.experimentId}</dd><dt class="text-muted">Last checked</dt><dd>{selected.lastCheckedAt ? new Date(selected.lastCheckedAt).toLocaleString() : 'Not yet synchronized'}</dd><dt class="text-muted">Remote heartbeat</dt><dd>{selected.remote ? new Date(selected.remote.updatedAt).toLocaleString() : 'Waiting for bootstrap'}</dd><dt class="text-muted">GPU utilization</dt><dd class="whitespace-pre-wrap">{selected.remote?.gpu || 'Unavailable until the worker starts'}</dd></dl>
            {#if selected.remote?.progress}<div class="mt-5 max-w-xl text-xs"><p>Iteration {selected.remote.progress.iteration} · loss {selected.remote.progress.loss.toFixed(5)}</p></div>{/if}
            {#if selected.remote?.evaluation}<p class="mt-4 text-sm">Evaluation success: {(selected.remote.evaluation.successRate * 100).toFixed(1)}%</p>{/if}
            {#if selected.remote?.error || selected.error}<p role="alert" class="mt-4 text-xs text-red-400">{selected.remote?.error || selected.error}</p>{/if}
            <div class="mt-6 flex flex-wrap gap-3"><Button variant="outline" disabled={!!busy || !account || selected.status !== 'completed' || selected.retrieved} onclick={() => action('retrieve')}><Download size={14}/>{selected.retrieved ? 'Results retrieved' : 'Retrieve results'}</Button><Button variant="outline" disabled={!!busy || !account || selected.instanceState !== 'RUNNING'} onclick={() => action('stop')}>Stop pod</Button><Button variant="outline" disabled={!!busy || !account || selected.instanceState === 'TERMINATED'} onclick={() => releaseOpen = true}><Square size={14}/>Release compute</Button></div>
            <p class="mt-4 max-w-2xl text-xs leading-relaxed text-muted">Keep Humaan open until the experiment upload finishes (Preparing or Running). Then jobs continue when Humaan closes. Reconnect your account and refresh to monitor again. Retrieve results to inspect the policy, evaluation, and 3D replay in Training experiments. Live remote viewport is not available for this adapter.</p>
            <p class="mt-3 max-w-2xl text-xs leading-relaxed text-muted">A stopped pod retains billable volume storage. Resume a stopped pod in the RunPod console to retrieve its results, then stop or release it. Release compute permanently deletes the pod and its volume. Retrieve results first. Cleanup is complete when Compute shows TERMINATED.</p>
          {/if}
        </div>
      {:else}
        <form class="min-h-0 flex-1 overflow-auto p-5" onsubmit={e => { e.preventDefault(); estimate(); }}>
          <div class="max-w-3xl"><h2 class="text-base font-medium text-title">Launch a cloud experiment</h2><p class="mt-2 text-xs leading-relaxed text-muted">Connect a RunPod account to provision an on-demand GPU pod. A provider-side deadline stops or terminates it even when Humaan disconnects. The initial adapter runs MuJoCo on CPU; GPU acceleration and Isaac Sim training are not yet available.</p>
            <fieldset disabled={!!busy} class="mt-5 grid grid-cols-2 gap-4 text-xs">
              <label class="col-span-2">Experiment<NativeSelect class="mt-1 w-full" bind:value={experimentId} required><option value="">Select a saved training experiment</option>{#each experiments as e}<option value={e.id}>{e.name} · {e.robotName}</option>{/each}</NativeSelect></label>
              <label>Region<NativeSelect class="mt-1 w-full" bind:value={config.countryCode}><option value="">Any available region</option><option value="US">United States</option><option value="CA">Canada</option><option value="RO">Romania</option><option value="CZ">Czechia</option><option value="SE">Sweden</option><option value="NO">Norway</option><option value="NL">Netherlands</option><option value="GB">United Kingdom</option></NativeSelect></label>
              <label>GPU configuration<NativeSelect class="mt-1 w-full" bind:value={config.gpuTypeId} required><option value="">Connect an account and select GPU</option>{#each account?.gpus.filter(g => g.secureCloud) ?? [] as gpu}<option value={gpu.id}>{gpu.displayName} · {gpu.memoryInGb} GB</option>{/each}</NativeSelect></label>
              <label>Runtime limit (minutes)<Input class="mt-1 w-full" type="number" min={10} max={120} step={1} bind:value={config.minutes} required/></label>
              <label>At runtime deadline<NativeSelect class="mt-1 w-full" bind:value={config.shutdown}><option value="terminate">Terminate · delete all remote results</option><option value="stop">Stop · retain billable volume</option></NativeSelect></label>
            </fieldset>
            <p class="mt-5 text-xs leading-relaxed text-muted">RunPod Secure Cloud · 1 GPU · 20 GB container disk + 20 GB persistent pod volume. Runtime: MuJoCo 3.2.7 / NumPy 2.0.2. Training finishes independently; the pod stays available for retrieving results until its deadline. NVIDIA Brev is planned as the next provider.</p>
            <Button class="mt-5" type="submit" disabled={!!busy || !account || !experimentId || !config.gpuTypeId}>Review cost and launch</Button>
          </div>
        </form>
      {/if}
    </section>
  </WorkspaceColumn>
</div>
<Dialog.Root bind:open={reviewOpen}><Dialog.Content><Dialog.Header><Dialog.Title>Review cloud launch</Dialog.Title><Dialog.Description>Provision one RunPod GPU pod for this experiment.</Dialog.Description></Dialog.Header>{#if quote}<div class="space-y-3 text-sm"><p>{quote.experimentName}</p><p class="text-xs text-muted">Account {quote.accountId} · {quote.config.countryCode || 'Any region'}<br/>{quote.config.gpuTypeId}</p><p class="text-lg font-medium">${quote.estimatedComputeUsd.toFixed(2)} estimated compute</p><p class="text-xs text-muted">${quote.hourlyUsd.toFixed(4)}/hour × {quote.config.minutes} minutes. A live estimate, not a reservation or spending cap. Storage, transfer, and taxes may add charges. {quote.config.shutdown === 'stop' ? 'The pod stops at its deadline and retains billable volume storage.' : 'The pod and all unretrieved results are deleted at the deadline.'}</p><p class="text-xs text-muted">Launch transfers the pinned robot, meshes, experiment, and worker through the authenticated RunPod proxy. Estimate expires {new Date(quote.expiresAt).toLocaleTimeString()}.</p><label class="flex items-start gap-2 text-xs"><input type="checkbox" bind:checked={acknowledged} disabled={!!busy}/>I reviewed the estimate, runtime deadline, and storage behavior.</label></div>{/if}<Dialog.Footer><Button variant="outline" disabled={!!busy} onclick={() => reviewOpen = false}>Cancel</Button><Button disabled={!!busy || !acknowledged} onclick={launch}>Provision and launch</Button></Dialog.Footer></Dialog.Content></Dialog.Root>
<Dialog.Root bind:open={releaseOpen}><Dialog.Content><Dialog.Header><Dialog.Title>Release compute?</Dialog.Title><Dialog.Description>Permanently terminate this pod and delete its volume, including remote results. Retrieve results first. An active workload will stop.</Dialog.Description></Dialog.Header><Dialog.Footer><Button variant="outline" disabled={!!busy} onclick={() => releaseOpen = false}>Cancel</Button><Button variant="destructive" disabled={!!busy} onclick={() => action('release')}>Terminate pod</Button></Dialog.Footer></Dialog.Content></Dialog.Root>
