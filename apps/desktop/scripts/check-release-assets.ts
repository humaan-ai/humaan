import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

const config = JSON.parse(readFileSync(new URL('../src-tauri/tauri.conf.json', import.meta.url), 'utf8'));
const version = config.version;
const [directory, platform = 'all'] = process.argv.slice(2);
const groups: Record<string, { manifest: string; files: string[] }> = {
  macos: { manifest: 'SHA256SUMS', files: [`Humaan_${version}_aarch64.dmg`, `Humaan_${version}_aarch64.app.zip`] },
  linux: { manifest: 'humaan-studio-linux-x86_64-SHA256SUMS', files: [`Humaan_${version}_amd64.deb`, `Humaan_${version}_amd64.AppImage`] },
  windows: { manifest: 'humaan-studio-windows-x86_64-SHA256SUMS', files: [`Humaan_${version}_x64-setup.exe`] },
};
if (!directory || (platform !== 'all' && !groups[platform])) {
  throw new Error('Usage: bun check-release-assets.ts <directory> [all|macos|linux|windows]');
}
for (const group of platform === 'all' ? Object.values(groups) : [groups[platform]!]) {
  const entries = new Map<string, string>();
  for (const line of readFileSync(resolve(directory, group.manifest), 'utf8').trim().split(/\r?\n/)) {
    const match = /^([a-fA-F0-9]{64}) [ *](?:\.\/)?([A-Za-z0-9._-]+)$/.exec(line);
    if (!match || entries.has(match[2]!)) throw new Error(`Invalid or duplicate checksum in ${group.manifest}`);
    entries.set(match[2]!, match[1]!.toLowerCase());
  }
  if (entries.size !== group.files.length) throw new Error(`Unexpected asset list in ${group.manifest}`);
  for (const filename of group.files) {
    const content = readFileSync(resolve(directory, filename));
    if (!content.length || createHash('sha256').update(content).digest('hex') !== entries.get(filename)) {
      throw new Error(`Missing checksum, empty asset, or checksum mismatch: ${filename}`);
    }
    console.log(`Verified ${filename}`);
  }
}
