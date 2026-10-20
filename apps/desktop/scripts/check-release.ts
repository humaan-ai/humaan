import { syncVersions } from '../../../scripts/sync-versions';
import { readFileSync, existsSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { resolve } from 'node:path';

syncVersions({ check: true });

const root = fileURLToPath(new URL('../../../', import.meta.url));
const read = (path: string) => readFileSync(resolve(root, path), 'utf8');
const pkg = JSON.parse(read('apps/desktop/package.json'));
const config = JSON.parse(read('apps/desktop/src-tauri/tauri.conf.json'));
const cargo = Bun.TOML.parse(read('apps/desktop/src-tauri/Cargo.toml')) as any;
const lock = Bun.TOML.parse(read('Cargo.lock')) as any;
const version = config.version;
const semver = /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-([0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*))?$/;
const match = typeof version === 'string' && semver.exec(version);
if (!match || match[4]?.split('.').some((part: string) => /^0\d+$/.test(part))) {
  throw new Error('Studio version must be a valid SemVer without build metadata');
}
if ([pkg.version, cargo.package.version, lock.package.find((p: any) => p.name === 'desktop')?.version]
  .some(value => value !== version)) {
  throw new Error('Studio package.json, Cargo.toml, Cargo.lock, and tauri.conf.json versions must match');
}
if (config.identifier !== 'ai.humaan.desktop') throw new Error('Preserve the installed Studio app identifier');
for (const path of ['LICENSE', 'crates/engine/Cargo.toml', 'docs/README.mdx', 'resources/README.mdx']) {
  if (!existsSync(resolve(root, path))) throw new Error(`Missing repository dependency: ${path}`);
}
const tag = process.env.RELEASE_TAG;
if (tag && tag !== `studio-v${version}`) throw new Error(`Release tag must be studio-v${version}`);
console.log(`Studio ${version}: versions, identity, repository layout${tag ? ', and release tag' : ''} verified`);
