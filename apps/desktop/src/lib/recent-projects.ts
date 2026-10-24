import type { LocalProject } from './studio.svelte';
export type RecentProject = { root: string; name: string; openedAt: number };
export const RECENT_PROJECTS_KEY = 'humaan-recent-projects';
export function parseRecentProjects(raw: string | null): RecentProject[] {
  try {
    const value: unknown = JSON.parse(raw || '[]');
    if (!Array.isArray(value)) return [];
    const seen = new Set<string>();
    return value.filter((entry): entry is RecentProject => !!entry && typeof entry.root === 'string' && !!entry.root.trim() && typeof entry.name === 'string' && !!entry.name.trim() && Number.isFinite(entry.openedAt))
      .sort((a, b) => b.openedAt - a.openedAt).filter(entry => { if (seen.has(entry.root)) return false; seen.add(entry.root); return true; }).slice(0, 12);
  } catch { return []; }
}
export function rememberProject(entries: RecentProject[], project: LocalProject, now = Date.now()): RecentProject[] {
  return [{ root: project.root, name: project.project.name, openedAt: now }, ...entries.filter(entry => entry.root !== project.root)].slice(0, 12);
}
