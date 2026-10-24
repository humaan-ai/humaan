import { invoke } from '@tauri-apps/api/core';
import { importUrdf, resolveUrdfMesh, type UrdfFile } from './urdf';
import type { Robot } from './model';

export type ProjectRobot = { id: string; name: string; source: string | null; missing: boolean };
export type ProjectRobots = { activeRobotId: string | null; robots: ProjectRobot[] };
export type RobotDraft = { robot: Robot; saved: string; currentVersion: string; selected: string; positions: Record<string, number>; warnings: string[] };

export function importedPath(file: UrdfFile) {
  // A directory picker includes the selected folder's name. The destination
  // name is chosen separately, so retain only its contents' relative paths.
  return file.webkitRelativePath ? file.webkitRelativePath.split('/').slice(1).join('/') : file.name;
}

export async function prepareRobotImport(files: File[]) {
  if (files.length > 10_000 || files.reduce((size, file) => size + file.size, 0) > 128 * 1024 * 1024) {
    throw new Error('Import up to 128 MB and 10,000 files at a time.');
  }
  const output: { path: string; data: number[] }[] = [];
  for (const file of files) {
    const path = importedPath(file);
    let bytes: Uint8Array;
    if (/\.urdf$/i.test(file.name)) {
      const original = await file.text();
      const doc = new DOMParser().parseFromString(original, 'application/xml');
      if (doc.querySelector('parsererror') || doc.documentElement.tagName !== 'robot') {
        throw new Error(`Invalid URDF: ${file.webkitRelativePath || file.name}`);
      }
      const base = (file.webkitRelativePath || file.name).split('/').slice(0, -1).join('/');
      let changed = false;
      for (const mesh of Array.from(doc.querySelectorAll('mesh[filename]'))) {
        const resource = mesh.getAttribute('filename')!;
        const target = resolveUrdfMesh(files, base, resource);
        const from = path.split('/').slice(0, -1), to = importedPath(target).split('/');
        while (from.length && to.length && from[0] === to[0]) { from.shift(); to.shift(); }
        const relative = [...from.map(() => '..'), ...to].join('/');
        if (relative !== resource) { mesh.setAttribute('filename', relative); changed = true; }
      }
      bytes = new TextEncoder().encode(changed ? new XMLSerializer().serializeToString(doc) : original);
    } else bytes = new Uint8Array(await file.arrayBuffer());
    output.push({ path, data: Array.from(bytes) });
  }
  return output;
}

export async function loadProjectUrdf(root: string, source: string) {
  const entries = await invoke<{ path: string; size: number }[]>('builder_robot_files', { root });
  const files: UrdfFile[] = entries.map(entry => {
    let bytes: Promise<ArrayBuffer> | undefined;
    const read = () => bytes ??= invoke<number[]>('builder_read_robot_file', { root, path: entry.path }).then(data => new Uint8Array(data).buffer);
    return {
      name: entry.path.split('/').at(-1)!,
      webkitRelativePath: entry.path,
      size: entry.size,
      arrayBuffer: read,
      text: async () => new TextDecoder().decode(await read()),
    };
  });
  return importUrdf(files, source);
}
