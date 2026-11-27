import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { compileModule } from 'svelte/compiler';
import { notesForVersion } from '../../../scripts/changelog-section.ts';

test('installed changelog selects the exact version even when a newer section is first', () => {
  const changelog = '## [1.0.0]\r\n### Added\r\n- New update\r\n## [0.9.0]\r\n### Fixed\r\n- **Installed** change\r\n## [0.8.0]\r\n- Older update';
  assert.equal(notesForVersion(changelog, '0.9.0'), '### Fixed\n- **Installed** change');
  assert.equal(notesForVersion(changelog, '0.9.1'), '');
});

test('updates download once, retain verified bytes for install retry, and prevent concurrent actions', async () => {
  let checks = 0;
  let downloads = 0;
  let installs = 0;
  let finishDownload;
  const candidate = {
    version: '0.2.0', body: 'Release notes',
    async download(progress) {
      downloads++;
      progress({ event: 'Started', data: { contentLength: 100 } });
      progress({ event: 'Progress', data: { chunkLength: 100 } });
      await new Promise(resolve => { finishDownload = resolve; });
    },
    async install(options) {
      assert.equal(options.restartAfterInstall, true);
      if (++installs === 1) throw new Error('Installer unavailable');
    },
    async close() {},
  };
  globalThis.__lumaUpdateTest = {
    native: true, invoke: async () => true, getVersion: async () => '0.1.0',
    check: async () => { checks++; return candidate; },
  };
  try {
    const source = readFileSync(new URL('./updates.svelte.ts', import.meta.url), 'utf8')
      .replace(/^import .*;\r?\n/gm, '')
      .replace('type Phase =', 'type Update = any;\ntype Phase =');
    const typescript = await import('typescript');
    const stripped = typescript.transpileModule(source, { compilerOptions: { target: typescript.ScriptTarget.ESNext, module: typescript.ModuleKind.ESNext } }).outputText;
    const compiled = compileModule(`const { native, invoke, getVersion, check } = globalThis.__lumaUpdateTest;\n${stripped}`, { filename: 'updates.svelte.js', generate: 'client' }).js.code
      .replace(/['"]svelte\/internal\/client['"]/g, JSON.stringify(import.meta.resolve('svelte/internal/client')));
    const state = await import(`data:text/javascript;base64,${Buffer.from(compiled).toString('base64')}`);
    await state.initializeUpdates();
    await state.initializeUpdates();
    assert.equal(checks, 1);
    assert.equal(state.updates.phase, 'available');
    const downloading = state.downloadUpdate();
    await state.downloadUpdate();
    await state.checkUpdates();
    assert.equal(downloads, 1);
    assert.equal(checks, 1);
    assert.equal(state.updates.downloaded, 100);
    assert.equal(installs, 0);
    finishDownload();
    await downloading;
    assert.equal(state.updates.phase, 'ready');
    await state.installUpdate();
    assert.equal(state.updates.phase, 'ready');
    assert.match(state.updates.error, /installer/);
    await state.installUpdate();
    assert.equal(installs, 2);
    assert.equal(downloads, 1);
  } finally {
    delete globalThis.__lumaUpdateTest;
  }
});
