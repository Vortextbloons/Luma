import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { stripTypeScriptTypes } from 'node:module';

const source = stripTypeScriptTypes(readFileSync(new URL('./customization.ts', import.meta.url), 'utf8').replace(/^import .*;\r?\n/gm, ''));
const { createCustomizationSync, customizationConfig } = await import(`data:text/javascript;base64,${Buffer.from(`const invoke = (...args) => globalThis.__customizationTest.invoke(...args); const listen = (...args) => globalThis.__customizationTest.listen(...args);\n${source}`).toString('base64')}`);
const config = () => ({ theme: 'violet-night', density: 'comfortable', appTileShape: 'wide', textSize: 'standard', reducedMotion: false, interfaceSounds: 'real', soundLevel: 'standard', home: [{ id: 'hero', component: 'hero', visible: true, span: 2 }], homeColumns: 1, widgets: { spotify: { enabled: true, placement: 'bar' } }, order: ['Custom'] });
const document = (revision, value = config()) => ({ version: 3, revision, config: structuredClone(value) });
const flush = () => new Promise(resolve => setImmediate(resolve));

test('migration hydrates native preferences and incoming changes do not echo back', async () => {
  const settings = config();
  const errors = [];
  let listener;
  let stopped = false;
  const saved = config();
  saved.theme = 'onyx';
  globalThis.__customizationTest = {
    listen: async (event, callback) => { if (event === 'customization-changed') listener = callback; return () => { stopped = true; }; },
    invoke: async command => { if (command === 'customization_preview_read') return null; assert.equal(command, 'customization_initialize'); return document(4, saved); },
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

test('a document serialized in a different key order does not trigger another save', async () => {
  const settings = config();
  let writes = 0;
  globalThis.__customizationTest = {
    listen: async () => () => {},
    invoke: async command => {
      if (command === 'customization_preview_read') return null;
      if (command === 'customization_initialize') {
        const value = config();
        return { version: 3, revision: 2, config: {
          theme: value.theme, density: value.density, appTileShape: value.appTileShape, textSize: value.textSize,
          reducedMotion: value.reducedMotion, interfaceSounds: value.interfaceSounds, soundLevel: value.soundLevel,
          homeColumns: value.homeColumns, home: value.home, widgets: value.widgets, order: value.order,
        } };
      }
      assert.equal(command, 'customization_commit');
      writes++;
      return document(3);
    },
  };
  const sync = createCustomizationSync(settings, error => assert.fail(error));
  try {
    await sync.start();
    sync.update(customizationConfig(settings));
    await flush();
    assert.equal(writes, 0);
  } finally { sync.dispose(); delete globalThis.__customizationTest; }
});

test('rapid edits serialize with increasing revisions and preserve edits during a save', async () => {
  const settings = config();
  const writes = [];
  const finishes = [];
  globalThis.__customizationTest = {
    listen: async () => () => {},
    invoke: async (command, args) => {
      if (command === 'customization_preview_read') return null;
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
      if (command === 'customization_preview_read') return null;
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

function previewFixture() {
  const settings = config();
  const listeners = new Map();
  const changes = [];
  const previews = [];
  const writes = [];
  let saved = document(4);
  let pending = null;
  let failKeep = false;
  globalThis.__customizationTest = {
    listen: async (event, callback) => { listeners.set(event, callback); return () => listeners.delete(event); },
    invoke: async (command, args) => {
      if (command === 'customization_initialize' || command === 'customization_read') return structuredClone(saved);
      if (command === 'customization_preview_read') return structuredClone(pending);
      if (command === 'customization_preview') {
        if (args.request.expectedRevision !== saved.revision) throw { code: 'conflict', message: 'Reload the saved layout' };
        pending = { id: `review-${writes.length}`, baseRevision: saved.revision, config: structuredClone(args.request.config) };
        return structuredClone(pending);
      }
      if (command === 'customization_preview_keep') {
        if (!pending || args.request.previewId !== pending.id) throw { code: 'preview_mismatch', message: 'Preview ended' };
        if (failKeep) throw { code: 'storage', message: 'Disk full' };
        writes.push(args.request);
        saved = document(saved.revision + 1, pending.config);
        pending = null;
        return structuredClone(saved);
      }
      if (command === 'customization_preview_cancel') { pending = null; return; }
      assert.fail(`Unexpected command: ${command}`);
    },
  };
  const sync = createCustomizationSync(settings, error => assert.fail(error), { document: value => changes.push(value), preview: value => previews.push(value) });
  return { settings, sync, previews, writes, listeners,
    get saved() { return saved; },
    failKeep(value) { failKeep = value; },
    externalWrite(value) { saved = value; pending = null; },
    cleanup() { sync.dispose(); delete globalThis.__customizationTest; },
  };
}

test('preview and cancel never autosave temporary appearance or layout, while Keep commits it once', async () => {
  const f = previewFixture();
  try {
    await f.sync.start();
    const original = customizationConfig(f.settings);
    const draft = { ...structuredClone(original), theme: 'onyx', homeColumns: 2 };
    draft.home[0].span = 1;
    await f.sync.preview(draft, 4);
    assert.deepEqual(customizationConfig(f.settings), original);
    f.sync.update(customizationConfig(f.settings));
    assert.equal(f.writes.length, 0);
    assert.equal(f.previews.at(-1).config.homeColumns, 2);
    await f.sync.cancel();
    assert.equal(f.previews.at(-1), null);
    assert.deepEqual(customizationConfig(f.settings), original);
    assert.equal(f.saved.revision, 4);
    await f.sync.preview(draft, 4);
    await f.sync.keep();
    assert.equal(f.settings.homeColumns, 2);
    assert.equal(f.settings.theme, 'onyx');
    assert.equal(f.saved.revision, 5);
    assert.equal(f.previews.at(-1), null);
    f.sync.update(customizationConfig(f.settings));
    assert.equal(f.writes.length, 1);
  } finally { f.cleanup(); }
});

test('failed Keep preserves the candidate and stale Keep reloads the committed layout', async () => {
  const f = previewFixture();
  try {
    await f.sync.start();
    await f.sync.preview({ ...config(), homeColumns: 2 }, 4);
    const candidate = f.previews.at(-1);
    f.failKeep(true);
    await assert.rejects(f.sync.keep(), error => error.code === 'storage');
    assert.equal(f.previews.at(-1).id, candidate.id);
    assert.equal(f.settings.homeColumns, 1);
    f.externalWrite(document(5, { ...config(), theme: 'mint' }));
    f.failKeep(false);
    await assert.rejects(f.sync.keep(), error => error.code === 'preview_mismatch');
    assert.equal(f.previews.at(-1), null);
    assert.equal(f.settings.theme, 'mint');
    assert.equal(f.writes.length, 0);
  } finally { f.cleanup(); }
});

test('disposed preview subscriptions cannot change the displayed layout', async () => {
  const f = previewFixture();
  try {
    await f.sync.start();
    assert.equal(f.listeners.size, 2);
    let finishRead;
    const previousInvoke = globalThis.__customizationTest.invoke;
    globalThis.__customizationTest.invoke = (command, args) => command === 'customization_preview_read' ? new Promise(resolve => { finishRead = resolve; }) : previousInvoke(command, args);
    f.listeners.get('customization-preview-changed')({ payload: null });
    const count = f.previews.length;
    f.sync.dispose();
    assert.equal(f.listeners.size, 0);
    finishRead({ id: 'late-preview', baseRevision: 4, config: { ...config(), homeColumns: 2 } });
    await flush();
    assert.equal(f.previews.length, count);
  } finally { f.cleanup(); }
});

test('plugin state, change events, and notices stay attached to the same customization owner', async () => {
  const settings = config();
  const listeners = new Map();
  const notices = [];
  const states = [];
  const state = (theme = 'ember') => ({
    plugins: [{ id: 'com.example.lounge', name: 'Lounge', version: '1.0.0', enabled: true, controls: ['theme'], settings: [], canEnable: false }],
    effective: { ...config(), theme },
  });
  globalThis.__customizationTest = {
    listen: async (event, callback) => { listeners.set(event, callback); return () => listeners.delete(event); },
    invoke: async command => {
      if (command === 'customization_initialize') return document(1);
      if (command === 'customization_preview_read') return null;
      if (command === 'customization_plugins_read') return structuredClone(state());
      assert.fail(`Unexpected command: ${command}`);
    },
  };
  const sync = createCustomizationSync(settings, value => notices.push(value), { plugins: value => states.push(value) });
  try {
    await sync.start();
    assert.equal(states.length, 1);
    assert.equal(states[0].plugins[0].name, 'Lounge');
    listeners.get('customization-plugins-changed')({ payload: state('onyx') });
    assert.equal(states.at(-1).effective.theme, 'onyx');
    listeners.get('customization-notice')({ payload: 'A plugin control was detached.' });
    assert.deepEqual(notices, ['A plugin control was detached.']);
    assert.equal(listeners.size, 4);
  } finally {
    sync.dispose();
    delete globalThis.__customizationTest;
  }
  assert.equal(listeners.size, 0);
});
