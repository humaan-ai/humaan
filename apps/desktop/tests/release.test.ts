import { test } from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, writeFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import config from '../src-tauri/tauri.conf.json';

const script = (name: string) => fileURLToPath(new URL(`../scripts/${name}.ts`, import.meta.url));

test('release gate accepts the matching tag and rejects a mismatched tag', () => {
  for (const [tag, expected] of [[`studio-v${config.version}`, 0], ['studio-v99.0.0', 1]] as const) {
    const result = spawnSync(process.execPath, [script('check-release')], {
      env: { ...process.env, RELEASE_TAG: tag }, encoding: 'utf8',
    });
    assert.equal(result.status, expected, result.stderr);
  }
});

test('asset gate requires every platform and rejects corruption and duplicate checksums', () => {
  const dir = mkdtempSync(resolve(tmpdir(), 'studio-release-'));
  const version = config.version;
  const groups = [
    ['SHA256SUMS', [`Humaan_${version}_aarch64.dmg`, `Humaan_${version}_aarch64.app.zip`]],
    ['humaan-studio-linux-x86_64-SHA256SUMS', [`Humaan_${version}_amd64.deb`, `Humaan_${version}_amd64.AppImage`]],
    ['humaan-studio-windows-x86_64-SHA256SUMS', [`Humaan_${version}_x64-setup.exe`]],
  ] as const;
  const run = () => spawnSync(process.execPath, [script('check-release-assets'), dir, 'all'], { encoding: 'utf8' });
  try {
    for (const [manifest, files] of groups) {
      const lines = files.map(file => {
        writeFileSync(resolve(dir, file), file);
        return `${createHash('sha256').update(file).digest('hex')}  ./${file}\n`;
      }).join('');
      writeFileSync(resolve(dir, manifest), lines);
    }
    assert.equal(run().status, 0);
    const filename = groups[2][1][0];
    writeFileSync(resolve(dir, filename), 'corrupted');
    assert.notEqual(run().status, 0);
    writeFileSync(resolve(dir, filename), filename);
    const line = `${createHash('sha256').update(filename).digest('hex')}  ${filename}\n`;
    writeFileSync(resolve(dir, groups[2][0]), line + line);
    assert.notEqual(run().status, 0);
    writeFileSync(resolve(dir, groups[2][0]), line);
    rmSync(resolve(dir, filename));
    assert.notEqual(run().status, 0);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});
