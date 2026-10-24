export type ProjectEntry = { name: string; path: string; directory: boolean; size: number; kind: string };

export function canDeleteProjectFile(entry: ProjectEntry): boolean {
  return !entry.directory && !entry.path.split(/[\\/]/).some(part => ['.humaan', '.git'].includes(part.toLowerCase()));
}
