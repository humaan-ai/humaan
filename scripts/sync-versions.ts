import { readFileSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

export const repositoryRoot = fileURLToPath(new URL('../', import.meta.url));
const semver = /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-([0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*))?$/;
export function versionPlan(root = repositoryRoot) {
  const read = (path: string) => readFileSync(resolve(root, path), 'utf8');
  const versions = JSON.parse(read('version.json'));
  if (!versions || typeof versions !== 'object' || Array.isArray(versions) || Object.keys(versions).sort().join(',') !== 'cli,studio') throw new Error('version.json must contain studio and cli versions only.');
  for (const component of ['studio', 'cli']) {
    const value = versions[component];
    const match = typeof value === 'string' && semver.exec(value);
    if (!match || match[4]?.split('.').some(part => /^0\d+$/.test(part))) throw new Error(`Invalid ${component} version: use SemVer without build metadata.`);
  }
  const changes: { path: string; before: string; after: string }[] = [];
  function update(path: string, transform: (text: string) => string) {
    const before = read(path), after = transform(before);
    if (before !== after) changes.push({ path, before, after });
  }
  function jsonVersion(text: string, version: string) {
    const value = JSON.parse(text);
    if (typeof value.version !== 'string') throw new Error('Missing JSON version field.');
    return text.replace(/("version"\s*:\s*)"[^"]*"/, `$1"${version}"`);
  }
  function cargoVersion(text: string, version: string) {
    const parsed = Bun.TOML.parse(text) as any;
    if (typeof parsed.package?.version !== 'string') throw new Error('Missing Cargo package version.');
    return text.replace(/(\[package\][\s\S]*?^version\s*=\s*)"[^"]*"/m, `$1"${version}"`);
  }
  update('apps/desktop/package.json', text => jsonVersion(text, versions.studio));
  update('apps/desktop/src-tauri/tauri.conf.json', text => jsonVersion(text, versions.studio));
  update('apps/desktop/src-tauri/Cargo.toml', text => cargoVersion(text, versions.studio));
  update('apps/cli/Cargo.toml', text => cargoVersion(text, versions.cli));
  update('Cargo.lock', text => {
    const lock = Bun.TOML.parse(text) as any;
    for (const [name, version] of [['desktop', versions.studio], ['humaan-cli', versions.cli]]) {
      if (lock.package.filter((p: any) => p.name === name).length !== 1) throw new Error(`Expected one ${name} package in Cargo.lock.`);
      const pattern = new RegExp(`(\\[\\[package\\]\\]\\r?\\nname = "${name}"\\r?\\nversion = )"[^"]*"`);
      if (!pattern.test(text)) throw new Error(`Cannot locate ${name} lockfile version.`);
      text = text.replace(pattern, `$1"${version}"`);
    }
    return text;
  });
  return { versions: versions as {studio: string; cli: string}, changes };
}
export function syncVersions({ root = repositoryRoot, check = false, tag = '' } = {}) {
  const { versions, changes } = versionPlan(root);
  if (tag && tag !== `studio-v${versions.studio}` && tag !== `cli-v${versions.cli}`) throw new Error(`Release tag ${tag} does not match version.json.`);
  if (check && changes.length) throw new Error(`Versions differ from version.json: ${changes.map(c => c.path).join(', ')}. Run bun run version:sync and commit the synchronized files.`);
  if (!check) for (const change of changes) writeFileSync(resolve(root, change.path), change.after);
  return { versions, changed: changes.map(c => c.path) };
}
if (import.meta.main) {
  if (process.argv.slice(2).some(arg => arg !== '--check')) throw new Error('Usage: bun scripts/sync-versions.ts [--check]');
  const { versions, changed } = syncVersions({ check: process.argv.includes('--check'), tag: process.env.RELEASE_TAG || '' });
  console.log(`Studio ${versions.studio}; CLI ${versions.cli}. ${changed.length ? `Synchronized ${changed.length} files.` : 'Versions aligned.'}`);
}
