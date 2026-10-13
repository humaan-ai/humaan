<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { toast } from 'svelte-sonner';
  import Policies from './Policies.svelte';
  import RobotPolicy from './RobotPolicy.svelte';
  import DeploymentNavigation from './DeploymentNavigation.svelte';
  import RobotArm from './icons/RobotArm.svelte';
  import Radio from '@lucide/svelte/icons/radio';
  import History from '@lucide/svelte/icons/history';
  import Plus from '@lucide/svelte/icons/plus';
  import Settings2 from '@lucide/svelte/icons/settings-2';
  import Play from '@lucide/svelte/icons/play';
  import Pause from '@lucide/svelte/icons/pause';
  import { studio } from '$lib/studio.svelte';
  import { emptyRobot, type SavedVersion, type Robot } from '$lib/builder/model';
  import { mapTelemetry, type Profile, type Session, type Frame, type Mapping } from '$lib/deployments/model';
  import StudioSidebar from './StudioSidebar.svelte';
  import WorkspaceColumn from './WorkspaceColumn.svelte';
  import BuilderViewport from './BuilderViewport.svelte';
  import { Button } from '$lib/components/ui/button/index.js';
  import { Input } from '$lib/components/ui/input/index.js';
  import { NativeSelect } from '$lib/components/ui/native-select/index.js';
  import * as Dialog from '$lib/components/ui/dialog/index.js';
  let { root }: { root: string } = $props();
  type Summary = { id: string; name: string; createdAt?: number };
  let robots = $state<Summary[]>([]), sessions = $state<Summary[]>([]), versions = $state<SavedVersion[]>([]);
  let profile = $state<Profile | null>(null), session = $state<Session | null>(null);
  let error = $state(''), busy = $state(false), query = $state(''), selected = $state('');
  let connected = $state(false), connecting = $state(false), recording = $state(false), pending = $state<Frame[]>([]);
  let positions = $state<Record<string, number>>({}), lastReceived = $state(0), now = $state(Date.now());
  let recordingStart = 0, recordingDate = 0, lastSequence = -1, generation = 0, disposed = false, polling = false;
  let index = $state(0), playing = $state(false), inspector = $state(true);
  let createOpen = $state(false), name = $state(''), versionId = $state(''), port = $state(9876), draftRobot = $state<Robot>(emptyRobot()), mappings = $state<Mapping[]>([]);
  const section = $derived(studio.deploymentSection);
  const displayed = $derived(session?.profile.robot || profile?.robot);
  const shownPositions = $derived(session ? session.frames[index]?.positions || {} : positions);
  const entries = $derived((section === 'robots' ? robots : sessions).filter(item => item.name.toLowerCase().includes(query.toLowerCase())));
  const locked = $derived(connected || connecting || recording || pending.length > 0 || busy || studio.policyBusy);
  const status = $derived(connecting ? 'Connecting' : connected ? now - lastReceived > 1500 ? 'Waiting for telemetry' : 'Live · Read only' : 'Disconnected');
  async function work(action: () => Promise<void>) {
    if (busy) return; busy = true; error = '';
    try { await action(); } catch (cause) { error = String(cause); } finally { busy = false; }
  }
  async function refresh() {
    [robots, sessions, versions] = await Promise.all([
      invoke<Summary[]>('deployment_list', { root, kind: 'robots' }),
      invoke<Summary[]>('deployment_list', { root, kind: 'sessions' }),
      invoke<SavedVersion[]>('builder_list', { root, kind: 'versions' })
    ]);
  }
  async function chooseVersion() {
    await work(async () => {
      draftRobot = await invoke<Robot>('builder_load', { root, kind: 'versions', id: versionId });
      mappings = draftRobot.joints.filter(j => j.type !== 'fixed').map(j => ({ jointId: j.id, channel: j.name, unit: j.type === 'prismatic' ? 'm' : 'rad', direction: 1, offset: 0 }));
      if (!name) name = draftRobot.name;
    });
  }
  async function create() {
    await work(async () => {
      profile = await invoke<Profile>('deployment_save', { root, kind: 'robots', value: { name: name.trim(), versionId, port: Number(port), mappings: $state.snapshot(mappings), createdAt: Date.now() } });
      session = null; positions = {}; selected = ''; createOpen = false; studio.deploymentSection = 'robots'; await refresh(); toast.success('Robot connection saved.');
    });
  }
  function newRobot() {
    name = ''; versionId = ''; port = 9876; mappings = []; draftRobot = emptyRobot(); createOpen = true; void work(refresh);
  }
  async function open(item: Summary) {
    if (locked) return;
    playing = false;
    await work(async () => {
      if (section === 'robots') { profile = await invoke<Profile>('deployment_load', { root, kind: 'robots', id: item.id }); session = null; positions = {}; }
      else { session = await invoke<Session>('deployment_load', { root, kind: 'sessions', id: item.id }); index = 0; }
      selected = '';
    });
  }
  async function saveRecording(reason = 'Stopped by user') {
    recording = false;
    if (!pending.length || !profile || busy) return;
    await work(async () => {
      const saved = await invoke<Session>('deployment_save', { root, kind: 'sessions', value: { name: `${profile!.name} · ${new Date(recordingDate).toLocaleString()}`, createdAt: recordingDate, profile: $state.snapshot(profile), frames: $state.snapshot(pending), reason } });
      pending = []; await refresh(); toast.success(`Saved session · ${saved.frames.length} samples`);
    });
  }
  async function disconnect(reason = 'Disconnected') {
    generation++; connected = false; connecting = false;
    if (recording || pending.length) await saveRecording(reason);
  }
  async function poll(token: number) {
    if (!profile || polling || (!connected && !connecting)) return;
    polling = true;
    try {
      const packet = await invoke<{ sequence: number; positions: Record<string, number> }>('deployment_sample', { port: profile.port });
      if (disposed || token !== generation) return;
      if (packet.sequence <= lastSequence) {
        if (Date.now() - lastReceived > 2000) throw new Error('Telemetry is stale. Live Link disconnected; last measured pose retained.');
        return;
      }
      const mapped = mapTelemetry(profile, packet.positions);
      lastSequence = packet.sequence; lastReceived = Date.now(); positions = mapped; connected = true; connecting = false;
      if (recording) {
        pending.push({ time: (performance.now() - recordingStart) / 1000, positions: mapped });
        if (pending.length >= 10000) { void saveRecording('10,000 sample limit reached'); }
      }
    } catch (cause) {
      if (disposed || token !== generation) return;
      const message = String(cause); error = ''; await disconnect(message); error = error ? `${message} ${error}` : message;
    } finally { polling = false; }
  }
  function connect() {
    if (!profile || locked) return;
    session = null; error = ''; lastSequence = -1; lastReceived = Date.now(); connecting = true; const token = ++generation; void poll(token);
  }
  function startRecording() {
    if (!connected || now - lastReceived > 1500 || pending.length) return;
    pending = []; recordingStart = performance.now(); recordingDate = Date.now(); recording = true;
  }
  function navigate(value: 'robots' | 'policies' | 'sessions') {
    if (locked) return;
    studio.deploymentSection = value; session = null; if (value === 'sessions') profile = null; playing = false; query = '';
  }
  $effect(() => { studio.deploymentBusy = locked; });
  onMount(() => {
    void work(refresh);
    const timer = setInterval(() => { now = Date.now(); if (connected || connecting) void poll(generation); }, 100);
    const unload = (event: BeforeUnloadEvent) => { if (recording || pending.length) { event.preventDefault(); event.returnValue = ''; } };
    window.addEventListener('beforeunload', unload);
    return () => { studio.deploymentBusy = false; disposed = true; generation++; clearInterval(timer); window.removeEventListener('beforeunload', unload); };
  });
  $effect(() => {
    if (!playing || !session) return;
    if (index >= session.frames.length - 1) { playing = false; return; }
    const delay = Math.max(1, (session.frames[index + 1].time - session.frames[index].time) * 1000);
    const timer = setTimeout(() => index++, delay);
    return () => clearTimeout(timer);
  });
</script>

{#if section === 'policies'}
  <Policies {root} blocked={connected || connecting || recording || pending.length > 0 || busy} onsection={navigate} />
{:else}
<div class="flex h-full min-h-0 gap-1">
  <StudioSidebar title="Deployments">
    <DeploymentNavigation active={section} disabled={locked} onselect={navigate} />
    <div class="flex items-center gap-2 p-3"><Input class="w-full min-w-0" aria-label="Search deployments" placeholder={section === 'robots' ? 'Search robots' : 'Search sessions'} bind:value={query} />{#if section === 'robots'}<Button size="icon-sm" title="Add robot" aria-label="Add robot" disabled={locked} onclick={newRobot}><Plus size={15} /></Button>{/if}</div>
    <div class="min-h-0 flex-1 overflow-auto">
      {#each entries as item}
        <button class="flex w-full items-center gap-2 px-4 py-2 text-left text-xs hover:bg-selection disabled:opacity-50 {(section === 'robots' ? profile?.id : session?.id) === item.id ? 'bg-selection text-title' : 'text-muted'}" disabled={locked} onclick={() => open(item)}>{#if section === 'robots'}<RobotArm size={15} />{:else}<History size={15} />{/if}<span class="truncate">{item.name}</span></button>
      {:else}<p class="px-4 py-3 text-xs text-muted">{query ? 'No matches.' : section === 'robots' ? 'Add a physical robot using a saved Builder version.' : 'Record a Live Link session to see it here.'}</p>{/each}
    </div>
    <div class="border-t border-border p-4 text-xs text-muted">Read-only telemetry<p class="mt-1 leading-relaxed">Manage trained controllers in Policies. Hardware execution is unavailable.</p></div>
  </StudioSidebar>
  <WorkspaceColumn active={studio.activeView === 'deployments'}>
    <section aria-label="Deployments workspace" class="workbench-panel flex min-h-0 flex-1 flex-col overflow-hidden">
      <header class="flex h-[35px] shrink-0 items-center justify-between border-b border-border px-3 text-xs"><span class="flex min-w-0 items-center gap-2"><RobotArm size={14} /><span class="truncate">{session?.name || profile?.name || (section === 'sessions' ? 'Sessions' : 'Robots')}</span></span><Button size="icon-sm" variant="ghost" aria-label="Toggle robot details" aria-pressed={inspector} onclick={() => inspector = !inspector}><Settings2 size={14} /></Button></header>
      <div class="flex h-[57px] shrink-0 items-center gap-2 border-b border-border px-4">
        {#if session}<span class="text-xs text-muted">Recorded session · {session.frames.length} samples</span><Button class="ml-auto" onclick={() => { studio.activeView = 'experiences'; }}>Experiences</Button>
        {:else if profile}<Button disabled={busy || (!connected && !connecting && locked)} onclick={() => connected || connecting ? disconnect() : connect()}><Radio size={15} />{connected || connecting ? 'Disconnect' : 'Live Link'}</Button><span class="rounded border border-border px-2 py-1 text-xs {connected ? 'text-success' : 'text-muted'}" role="status">{status}</span><Button class="ml-auto" disabled={busy || (!recording && (!connected || now - lastReceived > 1500 || pending.length > 0))} onclick={() => recording ? saveRecording() : startRecording()}>{recording ? `Stop & save · ${pending.length}` : 'Record session'}</Button>
        {:else}<span class="text-xs text-muted">Physical robots and recorded sessions</span>{/if}
      </div>
      {#if error}<div role="alert" class="border-b border-border px-4 py-3 text-xs text-error">{error}</div>{/if}
      {#if !recording && pending.length}<div class="flex items-center gap-3 border-b border-border p-3 text-xs text-warning">{pending.length} samples waiting to save.<Button disabled={busy} onclick={() => saveRecording('Recovered unsaved session')}>Retry save</Button></div>{/if}
      {#if displayed && (section === 'robots' || session)}
        <div class="flex min-h-0 flex-1">
          <div class="relative flex min-h-0 min-w-0 flex-1 flex-col"><BuilderViewport robot={displayed} {selected} onselect={id => selected = id} positions={shownPositions} /></div>
          {#if inspector}<aside aria-label="Robot details" class="w-72 shrink-0 overflow-auto border-l border-border p-4 text-xs">
            <h3 class="font-medium text-title">{session ? 'Session details' : 'Connection'}</h3>
            <dl class="mt-4 space-y-3"><div><dt class="text-muted">Source version</dt><dd class="mt-1 break-all">{session?.profile.versionId || profile?.versionId}</dd></div><div><dt class="text-muted">Interface</dt><dd class="mt-1">Local telemetry bridge · 127.0.0.1:{session?.profile.port || profile?.port}</dd></div>{#if session}<div><dt class="text-muted">Recorded</dt><dd class="mt-1">{new Date(session.createdAt).toLocaleString()}</dd></div><div><dt class="text-muted">End reason</dt><dd class="mt-1">{session.reason}</dd></div>{/if}</dl>
            {#if profile && !session}<RobotPolicy {root} robotId={profile.id} blocked={locked} />{/if}
            <h3 class="mb-2 mt-6 font-medium text-title">Joint telemetry</h3>
            {#each (session?.profile.mappings || profile?.mappings || []) as mapping}<div class="border-t border-border py-2"><div class="flex justify-between gap-2"><span class="truncate">{displayed.joints.find(j => j.id === mapping.jointId)?.name}</span><span class="font-mono">{shownPositions[mapping.jointId]?.toFixed(3) ?? '—'} {mapping.unit === 'm' ? 'm' : 'rad'}</span></div><p class="mt-1 text-muted">{mapping.channel} · {mapping.unit} · {mapping.direction === -1 ? 'Reversed' : 'Forward'} · offset {mapping.offset}</p></div>{/each}
            <p class="mt-5 leading-relaxed text-muted">Measured motion is mirrored in 3D. This view does not run physics or send motor commands.</p>
          </aside>{/if}
        </div>
        {#if session}<div class="flex items-center gap-3 border-t border-border px-4 py-3"><Button size="icon-sm" aria-label={playing ? 'Pause session' : 'Play session'} onclick={() => { if (index >= session!.frames.length - 1) index = 0; playing = !playing; }}>{#if playing}<Pause size={15} />{:else}<Play size={15} />{/if}</Button><input class="min-w-0 flex-1 accent-white" type="range" aria-label="Session timeline" min={0} max={session.frames.length - 1} bind:value={index} oninput={() => playing = false} /><span class="text-xs text-muted">{session.frames[index]?.time.toFixed(1)} / {session.frames.at(-1)?.time.toFixed(1)} s</span></div>{/if}
      {:else}<div class="flex flex-1 flex-col items-center justify-center gap-4 p-8 text-center"><RobotArm size={36} class="text-muted" /><h2 class="text-lg">{section === 'robots' ? 'Connect your physical robot' : 'Replay physical robot sessions'}</h2><p class="max-w-md text-sm leading-relaxed text-muted">{section === 'robots' ? 'Pin a Builder version, map telemetry channels to joints, and mirror your robot in the workspace.' : 'Select a recorded session to inspect its measured motion and exact robot configuration.'}</p>{#if section === 'robots'}<Button onclick={newRobot} disabled={busy}><Plus size={15} />Add robot</Button>{/if}</div>{/if}
    </section>
  </WorkspaceColumn>
</div>
{/if}
<Dialog.Root bind:open={createOpen}>
  <Dialog.Content class="max-w-3xl" onInteractOutside={event => event.preventDefault()}>
    <Dialog.Title>Add robot</Dialog.Title><Dialog.Description>Connect a saved robot version to a local read-only telemetry bridge.</Dialog.Description>
    <form class="mt-4 space-y-4" onsubmit={event => { event.preventDefault(); void create(); }}>
      <div class="grid grid-cols-2 gap-4"><label class="text-xs">Robot name<Input class="mt-2 w-full" bind:value={name} required maxlength={200} /></label><label class="text-xs">Builder version<NativeSelect aria-label="Builder version" class="mt-2 w-full" bind:value={versionId} onchange={event => { versionId = event.currentTarget.value; void chooseVersion(); }} disabled={busy}><option value="">Select saved version</option>{#each versions as version}<option value={version.id}>{version.name} · {new Date(version.savedAt).toLocaleString()}</option>{/each}</NativeSelect></label></div>
      {#if !versions.length}<p class="text-xs text-muted">Save a version in Builder first.</p>{/if}
      <label class="block text-xs">Local bridge port<Input type="number" class="mt-2 w-40" min={1} max={65535} step={1} bind:value={port} required /></label>
      <p class="text-xs leading-relaxed text-muted">The bridge supplies measured positions on 127.0.0.1. Use calibrated radians/degrees for rotating joints and metres for sliding joints. Offsets use radians or metres after conversion.</p>
      {#if mappings.length}<div class="max-h-64 overflow-auto rounded border border-border"><table class="w-full text-left text-xs"><thead class="bg-surface text-muted"><tr><th class="p-2">Joint</th><th class="p-2">Channel</th><th class="p-2">Unit</th><th class="p-2">Direction</th><th class="p-2">Offset</th></tr></thead><tbody>{#each mappings as mapping}<tr class="border-t border-border"><td class="p-2">{draftRobot.joints.find(j => j.id === mapping.jointId)?.name}</td><td class="p-2"><Input class="w-full min-w-20" aria-label="Channel {mapping.jointId}" bind:value={mapping.channel} required /></td><td class="p-2"><NativeSelect aria-label="Unit {mapping.jointId}" bind:value={mapping.unit}>{#if draftRobot.joints.find(j => j.id === mapping.jointId)?.type === 'prismatic'}<option value="m">m</option>{:else}<option value="rad">rad</option><option value="deg">deg</option>{/if}</NativeSelect></td><td class="p-2"><NativeSelect aria-label="Direction {mapping.jointId}" bind:value={mapping.direction}><option value={1}>Forward</option><option value={-1}>Reverse</option></NativeSelect></td><td class="p-2"><Input class="w-20" type="number" step="any" aria-label="Offset {mapping.jointId}" bind:value={mapping.offset} required /></td></tr>{/each}</tbody></table></div>{/if}
      {#if error}<p role="alert" class="text-xs text-error">{error}</p>{/if}
      <div class="flex justify-end gap-2"><Button type="button" disabled={busy} onclick={() => createOpen = false}>Cancel</Button><Button type="submit" disabled={busy || !versionId || !mappings.length}>Save robot</Button></div>
    </form>
  </Dialog.Content>
</Dialog.Root>
