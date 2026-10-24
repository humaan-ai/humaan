<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { untrack, onDestroy } from 'svelte';
  import { studio } from '$lib/studio.svelte';
  import type { Assignment } from '$lib/deployments/policies';
  import { Button } from '$lib/components/ui/button/index.js';
  let { root, robotId, blocked }: { root: string; robotId: string; blocked: boolean } = $props();
  let assignment = $state<Assignment | null>(null), error = $state(''); let request = 0;
  $effect(() => { const id = robotId; const project = root; const token = ++request; assignment = null; error = ''; untrack(() => { void invoke<Assignment>('policy_assignment', { root: project, robotId: id }).then(value => { if (token === request) assignment = value; }).catch(cause => { if (token === request) error = String(cause); }); }); });
  onDestroy(() => request++);
  const current = $derived(assignment?.history.at(-1));
</script>
<section class="mt-5 border-t border-border pt-4" aria-label="Assigned policy"><h3 class="font-medium text-title">Assigned policy</h3><p class="mt-2 break-words">{error ? 'Assignment unavailable' : assignment ? current?.policyName || 'No policy assigned' : 'Loading…'}</p>{#if current?.policyId}<p class="mt-1 text-muted">Assigned locally · Not running</p>{/if}<Button class="mt-3" size="sm" disabled={blocked} onclick={() => { studio.policyRobotId = robotId; studio.deploymentSection = 'policies'; }}>Manage policies</Button><p class="mt-2 leading-relaxed text-muted">Hardware execution requires a motor-control adapter.</p></section>
