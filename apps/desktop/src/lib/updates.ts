const RELEASES_API = "https://api.github.com/repos/humaan-ai/humaan/releases";
const DOWNLOADS = "https://github.com/humaan-ai/humaan/releases/download/";
const VERSION = /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-([0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*))?$/;
export type StudioUpdate = { version: string; url: string; filename: string };

export function compareVersions(a: string, b: string): number {
  const aa = VERSION.exec(a), bb = VERSION.exec(b);
  if (!aa || !bb) throw new Error("Invalid Studio version");
  for (let i = 1; i <= 3; i++) {
    if (BigInt(aa[i]!) !== BigInt(bb[i]!)) return BigInt(aa[i]!) > BigInt(bb[i]!) ? 1 : -1;
  }
  if (!aa[4] || !bb[4]) return aa[4] === bb[4] ? 0 : aa[4] ? -1 : 1;
  const ap = aa[4].split('.'), bp = bb[4].split('.');
  for (let i = 0; i < Math.max(ap.length, bp.length); i++) {
    const x = ap[i], y = bp[i];
    if (x === y) continue;
    if (x === undefined || y === undefined) return x === undefined ? -1 : 1;
    const xn = /^\d+$/.test(x), yn = /^\d+$/.test(y);
    if (xn && yn) { if (BigInt(x) === BigInt(y)) continue; return BigInt(x) > BigInt(y) ? 1 : -1; }
    if (xn !== yn) return xn ? -1 : 1;
    return x > y ? 1 : -1;
  }
  return 0;
}

export function updateFilename(version: string, target: string): string | null {
  const suffix: Record<string, string> = {
    'macos-aarch64': 'aarch64.dmg', 'macos-x86_64': 'x64.dmg',
    'windows-x86_64': 'x64-setup.exe',
    'linux-x86_64-deb': 'amd64.deb', 'linux-x86_64-appimage': 'amd64.AppImage',
  };
  return suffix[target] ? `Humaan_${version}_${suffix[target]}` : null;
}

export function selectStudioUpdate(values: unknown[], current: string, target: string): StudioUpdate | null {
  if (!VERSION.test(current)) throw new Error('Invalid installed Studio version');
  if (!updateFilename(current, target)) throw new Error('Update downloads are not supported for this platform.');
  let result: StudioUpdate | null = null;
  for (const value of values) {
    if (!value || typeof value !== 'object') continue;
    const r = value as Record<string, unknown>;
    if (r.draft !== false || typeof r.prerelease !== 'boolean' || typeof r.tag_name !== 'string' || !Array.isArray(r.assets)) continue;
    if (!r.tag_name.startsWith('studio-v')) continue;
    const version = r.tag_name.slice('studio-v'.length);
    if (!VERSION.test(version) || (!current.includes('-') && (r.prerelease || version.includes('-')))) continue;
    if (compareVersions(version, current) <= 0 || (result && compareVersions(version, result.version) <= 0)) continue;
    const filename = updateFilename(version, target)!;
    const url = `${DOWNLOADS}${r.tag_name}/${filename}`;
    if (r.assets.some((a) => a && typeof a === 'object' && a.name === filename && a.state === 'uploaded' && typeof a.size === 'number' && a.size > 0 && a.browser_download_url === url)) {
      result = {version, filename, url};
    }
  }
  return result;
}

export async function checkStudioUpdate(current: string, target: string, fetcher: typeof fetch = fetch): Promise<StudioUpdate | null> {
  const releases: unknown[] = [];
  const signal = AbortSignal.timeout(15000);
  for (let page = 1; page <= 10; page++) {
    const response = await fetcher(`${RELEASES_API}?per_page=100&page=${page}`, {
      headers: { Accept: 'application/vnd.github+json', 'X-GitHub-Api-Version': '2022-11-28' },
      signal, cache: 'no-store',
    });
    if (!response.ok) throw new Error(`Could not check updates (HTTP ${response.status}). Try again later.`);
    const data: unknown = await response.json();
    if (!Array.isArray(data)) throw new Error('Invalid update response. Try again later.');
    releases.push(...data);
    if (data.length < 100) return selectStudioUpdate(releases, current, target);
  }
  throw new Error('Release history is too large to check. Visit humaan.ai/download.');
}
