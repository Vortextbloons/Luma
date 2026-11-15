import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { stripTypeScriptTypes } from 'node:module';

const source = stripTypeScriptTypes(readFileSync(new URL('./customization.ts', import.meta.url), 'utf8').replace(/^import .*;\r?\n/gm, ''));
const { createCustomizationSync, customizationConfig } = await import(`data:text/javascript;base64,${Buffer.from(`const invoke = (...args) => globalThis.__customizationTest.invoke(...args); const listen = (...args) => globalThis.__customizationTest.listen(...args);\n${source}`).toString('base64')}`);
const config = () => ({ theme: 'violet-night', density: 'comfortable', appTileShape: 'wide', home: [{ id: 'hero', component: 'hero', visible: true }], widgets: { spotify: { enabled: true, placement: 'bar' } }, order: ['Custom'] });
const document = (revision, value = config()) => ({ version: 2, revision, config: structuredClone(value) });
const flush = () => new Promise(resolve => setImmediate(resolve));

test('migration hydrates native preferences and incoming changes do not echo back', async () => {
  const settings = config();
  const errors = [];
  let listener;
  let stopped = false;
  const saved = config();
  saved.theme = 'onyx';
  globalThis.__customizationTest = {
    listen: async (_, callback) => { listener = callback; return () => { stopped = true; }; },
    invoke: async command => { assert.equal(command, 'customization_initialize'); return document(4, saved); },
  };
  const sync = createCustomizationSync(settings, error => errors.push(error));
  try {
    await sync.start();
    assert.equal(settings.theme, 'onyx');
    sync.update(customizationConfig(settings));
    const remote = document(5, { ...saved, density: 'compact' });
    listener({ payload: remote });
    sync.update(customizationConfig(settings));
    listener({ payload: document(3) });
    assert.equal(settings.density, 'compact');
    settings.theme = 'ember';
    listener({ payload: remote });
    assert.equal(settings.theme, 'ember');
    assert.equal(errors.length, 0);
  } finally { sync.dispose(); delete globalThis.__customizationTest; }
  assert.equal(stopped, true);
});

test('rapid edits serialize with increasing revisions and preserve edits during a save', async () => {
  const settings = config();
  const writes = [];
  const finishes = [];
  globalThis.__customizationTest = {
    listen: async () => () => {},
    invoke: async (command, args) => {
      if (command === 'customization_initialize') return document(1);
      assert.equal(command, 'customization_commit');
      writes.push(structuredClone(args.request));
      return new Promise(resolve => finishes.push(resolve));
    },
  };
  const sync = createCustomizationSync(settings, error => assert.fail(error));
  try {
    await sync.start();
    settings.home[0].visible = false;
    sync.update(customizationConfig(settings));
    settings.widgets.spotify.placement = 'bottom-left';
    sync.update(customizationConfig(settings));
    assert.equal(writes.length, 1);
    assert.equal(writes[0].config.widgets.spotify.placement, 'bar');
    finishes[0](document(2, writes[0].config));
    await flush();
    assert.equal(writes.length, 2);
    assert.equal(writes[1].expectedRevision, 2);
    assert.equal(writes[1].config.home[0].visible, false);
    assert.equal(writes[1].config.widgets.spotify.placement, 'bottom-left');
    finishes[1](document(3, writes[1].config));
    await flush();
    sync.update(customizationConfig(settings));
    assert.equal(writes.length, 2);
  } finally { sync.dispose(); delete globalThis.__customizationTest; }
});

test('conflict reloads the latest document and write failures retain user input without retry loops', async () => {
  const settings = config();
  const errors = [];
  let writes = 0;
  globalThis.__customizationTest = {
    listen: async () => () => {},
    invoke: async command => {
      if (command === 'customization_initialize') return document(1);
      if (command === 'customization_read') return document(3, { ...config(), theme: 'onyx' });
      writes++;
      throw writes === 1 ? { code: 'conflict', message: 'Changed elsewhere' } : { code: 'storage', message: 'Disk full' };
    },
  };
  const sync = createCustomizationSync(settings, error => errors.push(error));
  try {
    await sync.start();
    settings.theme = 'mint';
    sync.update(customizationConfig(settings));
    await flush();
    assert.equal(settings.theme, 'onyx');
    settings.theme = 'ember';
    sync.update(customizationConfig(settings));
    await flush();
    assert.equal(settings.theme, 'ember');
    sync.update(customizationConfig(settings));
    await flush();
    assert.equal(writes, 2);
    assert.equal(errors.length, 2);
    assert.match(errors[1], /Disk full/);
  } finally { sync.dispose(); delete globalThis.__customizationTest; }
});

test('disposal during initialization removes the late subscription without mutating settings', async () => {
  const settings = config();
  let finish;
  let stopped = false;
  globalThis.__customizationTest = {
    listen: async () => new Promise(resolve => { finish = resolve; }),
    invoke: async () => assert.fail('Disposed controller must not initialize'),
  };
  const sync = createCustomizationSync(settings, error => assert.fail(error));
  try {
    const starting = sync.start();
    sync.dispose();
    finish(() => { stopped = true; });
    await starting;
    assert.equal(stopped, true);
  } finally { sync.dispose(); delete globalThis.__customizationTest; }
});
