import { tick } from 'svelte';
import { studio } from './studio.svelte';

export type ProblemSeverity = 'error' | 'warning' | 'info';
export type Problem = { id: string; severity: ProblemSeverity; message: string; source: string; reveal?: () => void };
export type ProblemGroup = { label: string; path?: string; items: Problem[] };

// Each feature owns its group and removes it when its project is unloaded.
export const problemGroups = $state<Record<string, ProblemGroup>>({});

export function problemCounts() {
  const counts = { error: 0, warning: 0, info: 0, total: 0 };
  for (const group of Object.values(problemGroups)) {
    for (const problem of group.items) { counts[problem.severity]++; counts.total++; }
  }
  return counts;
}

export async function showProblems() {
  studio.bottomPanelTab = 'problems';
  studio.terminalOpen = true;
  studio.chatExpanded = false;
  await tick();
  document.getElementById('problems-tab')?.focus();
}
