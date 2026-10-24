import { describe, test } from 'node:test';
import assert from 'node:assert/strict';
import { checkStudioUpdate, compareVersions, selectStudioUpdate, updateFilename } from './updates';

function release(version: string, target = 'macos-aarch64', extra: Record<string, unknown> = {}) {
  const filename = updateFilename(version, target);
  const tag = `studio-v${version}`;
  return { tag_name: tag, draft: false, prerelease: version.includes('-'), assets: [{
    name: filename, state: 'uploaded', size: 100,
    browser_download_url: `https://github.com/humaan-ai/humaan/releases/download/${tag}/${filename}`,
  }], ...extra };
}

describe('Studio updates', () => {
  test('upgrades the archived baseline to the new repository baseline', () => {
    assert.equal(selectStudioUpdate([release('0.1.0-alpha.1')], '0.0.1-alpha.2', 'macos-aarch64')?.version, '0.1.0-alpha.1');
    assert.equal(selectStudioUpdate([release('0.0.1-alpha.2')], '0.1.0-alpha.1', 'macos-aarch64'), null);
    const archived = release('0.1.0-alpha.2');
    archived.assets[0]!.browser_download_url = archived.assets[0]!.browser_download_url.replace('/humaan/releases/', '/humaan-achieve/releases/');
    assert.equal(selectStudioUpdate([archived], '0.1.0-alpha.1', 'macos-aarch64'), null);
  });
  test('selects newer versions semantically, including graduation to stable', () => {
    assert.ok(compareVersions('0.0.1-alpha.10', '0.0.1-alpha.2') > 0);
    assert.ok(compareVersions('0.0.1', '0.0.1-rc.1') > 0);
    const values = [release('0.0.1-alpha.3'), release('0.0.1'), release('0.0.1-alpha.10')];
    assert.equal(selectStudioUpdate(values, '0.0.1-alpha.2', 'macos-aarch64')?.version, '0.0.1');
  });
  test('stable does not receive prereleases, drafts, or CLI releases', () => {
    const values = [release('2.0.0-alpha.1'), release('2.0.0', undefined, { prerelease: true }),
      release('3.0.0', undefined, { draft: true }), release('4.0.0', undefined, { tag_name: 'cli-v4.0.0' })];
    assert.equal(selectStudioUpdate(values, '1.0.0', 'macos-aarch64'), null);
  });
  test('requires the uploaded asset for the platform at the exact release URL', () => {
    const wrongUrl = release('0.0.1-alpha.4');
    wrongUrl.assets[0]!.browser_download_url = 'https://evil.example/update.dmg';
    assert.equal(selectStudioUpdate([wrongUrl, release('0.0.1-alpha.3', 'windows-x86_64')], '0.0.1-alpha.2', 'macos-aarch64'), null);
    const pending = release('0.0.1-alpha.3');
    pending.assets[0]!.state = 'new';
    assert.equal(selectStudioUpdate([pending], '0.0.1-alpha.2', 'macos-aarch64'), null);
  });
  test('never downgrades or reinstalls the current version', () => {
    assert.equal(selectStudioUpdate([release('0.0.1-alpha.1'), release('0.0.1-alpha.2')], '0.0.1-alpha.2', 'macos-aarch64'), null);
  });
  test('selects architecture and Linux packaging explicitly', () => {
    for (const target of ['macos-aarch64', 'macos-x86_64', 'windows-x86_64', 'linux-x86_64-appimage', 'linux-x86_64-deb']) {
      assert.equal(selectStudioUpdate([release('1.1.0', target)], '1.0.0', target)?.filename, updateFilename('1.1.0', target)!);
    }
    assert.throws(() => selectStudioUpdate([], '1.0.0', 'windows-aarch64'), /not supported/);
  });
  test('paginates across independently released tools', async () => {
    const urls: string[] = [];
    const fetcher = (async (url: string) => {
      urls.push(url);
      return Response.json(url.endsWith('page=1') ? Array.from({length: 100}, () => ({ tag_name: 'cli-v1.0.0' })) : [release('1.1.0')]);
    }) as typeof fetch;
    assert.equal((await checkStudioUpdate('1.0.0', 'macos-aarch64', fetcher))?.version, '1.1.0');
    assert.equal(urls.length, 2);
  });
  test('reports network or invalid-response failures instead of saying up to date', async () => {
    await assert.rejects(checkStudioUpdate('1.0.0', 'macos-aarch64', (async () => new Response('', { status: 403 })) as typeof fetch), /HTTP 403/);
    await assert.rejects(checkStudioUpdate('1.0.0', 'macos-aarch64', (async () => Response.json({})) as typeof fetch), /Invalid update response/);
  });
});
