import { test } from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, mkdirSync, readFileSync, writeFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { repositoryRoot, syncVersions } from './sync-versions';
function fixture(run: (root: string) => void) {
  const root = mkdtempSync(join(tmpdir(), 'humaan-versions-'));
  try {
    for (const file of ['version.json', 'apps/desktop/package.json', 'apps/desktop/src-tauri/tauri.conf.json', 'apps/desktop/src-tauri/Cargo.toml', 'apps/cli/Cargo.toml', 'Cargo.lock']) {
      mkdirSync(dirname(join(root, file)), {recursive:true});
      writeFileSync(join(root, file), readFileSync(join(repositoryRoot, file)));
    }
    run(root);
  } finally { rmSync(root, {recursive:true, force:true}); }
}
test('syncs independent versions, leaves dependencies unchanged, and is idempotent', () => fixture(root => {
  const read = (p: string) => readFileSync(join(root,p),'utf8');
  const before = (Bun.TOML.parse(read('Cargo.lock')) as any).package.filter((p: any) => !['desktop','humaan-cli'].includes(p.name));
  writeFileSync(join(root,'version.json'), JSON.stringify({studio:'0.2.0-beta.1',cli:'0.1.1'}));
  assert.throws(() => syncVersions({root,check:true}), /Versions differ/);
  assert.equal(syncVersions({root}).changed.length, 5);
  assert.equal(JSON.parse(read('apps/desktop/package.json')).version, '0.2.0-beta.1');
  assert.equal((Bun.TOML.parse(read('apps/cli/Cargo.toml')) as any).package.version, '0.1.1');
  assert.deepEqual((Bun.TOML.parse(read('Cargo.lock')) as any).package.filter((p: any) => !['desktop','humaan-cli'].includes(p.name)), before);
  assert.deepEqual(syncVersions({root}).changed, []);
  syncVersions({root,check:true,tag:'cli-v0.1.1'});
  syncVersions({root,check:true,tag:'studio-v0.2.0-beta.1'});
  assert.throws(() => syncVersions({root,check:true,tag:'studio-v0.1.1'}), /does not match/);
}));
test('invalid versions and malformed lockfiles fail before any files are written', () => fixture(root => {
  const manifest = join(root,'apps/desktop/package.json');
  const before = readFileSync(manifest,'utf8');
  for (const studio of ['v0.1.0', '0.1.0-alpha.01', '01.0.0', '0.1.0+build']) {
    writeFileSync(join(root,'version.json'), JSON.stringify({studio,cli:'0.1.0'}));
    assert.throws(() => syncVersions({root}), /Invalid studio version/);
  }
  writeFileSync(join(root,'version.json'), JSON.stringify({studio:'0.2.0',cli:'0.1.0'}));
  writeFileSync(join(root,'Cargo.lock'), '[[package]]\nname = "other"\nversion = "1.0.0"\n');
  assert.throws(() => syncVersions({root}), /Expected one desktop/);
  assert.equal(readFileSync(manifest,'utf8'), before);
}));
