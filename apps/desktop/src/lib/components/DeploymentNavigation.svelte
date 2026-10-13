<script lang="ts">
  import RobotArm from './icons/RobotArm.svelte';
  import History from '@lucide/svelte/icons/history';
  import SlidersHorizontal from '@lucide/svelte/icons/sliders-horizontal';
  import { Button } from '$lib/components/ui/button/index.js';
  let { active, disabled = false, onselect }: { active: 'robots' | 'policies' | 'sessions'; disabled?: boolean; onselect: (section: 'robots' | 'policies' | 'sessions') => void } = $props();
  const items = [{ id: 'robots', label: 'Robots', icon: RobotArm }, { id: 'policies', label: 'Policies', icon: SlidersHorizontal }, { id: 'sessions', label: 'Sessions', icon: History }] as const;
</script>
<nav aria-label="Deployment sections" class="flex shrink-0 flex-col gap-1 border-b border-border px-3 pb-3">
  {#each items as item}<Button variant="ghost" class="h-8 w-full justify-start {active === item.id ? 'bg-selection text-title' : 'text-muted'}" aria-current={active === item.id ? 'page' : undefined} {disabled} onclick={() => onselect(item.id)}><item.icon size={16} strokeWidth={1.5} aria-hidden="true" />{item.label}</Button>{/each}
</nav>
