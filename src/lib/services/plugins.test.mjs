import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { stripTypeScriptTypes } from 'node:module';

const source = stripTypeScriptTypes(readFileSync(new URL('./plugins.ts', import.meta.url), 'utf8').replace(/^import .*;\r?\n/gm, ''));
const { pluginControlSummary, pluginFieldLabel, pluginSettingValues, pluginRuntimeKind, pluginRuntimeLabel, pluginTriggers, pluginDefaultTrigger, pluginTriggerDescription, pluginTriggerSummary, pluginPermissionDescription, pluginRuntimeStateLabel, pluginRuntimeTone } = await import(`data:text/javascript;base64,${Buffer.from(source).toString('base64')}`);

const record = {
  id: 'com.example.lounge',
  name: 'Lounge',
  controls: ['theme', 'homeColumns', 'home'],
  settings: [
    { key: 'theme', value: 'ember' },
    { key: 'showFeatured', value: false },
    { key: 'columns', value: 2 },
  ],
};

test('plugin control summaries name the appearance fields', () => {
  assert.equal(pluginFieldLabel('homeColumns'), 'Home columns');
  assert.equal(pluginFieldLabel('unknown'), 'unknown');
  assert.equal(pluginControlSummary(record), 'Controls theme, Home columns, and Home layout');
  assert.equal(pluginControlSummary({ ...record, controls: [] }), 'No appearance controls attached');
  assert.equal(pluginControlSummary({ ...record, controls: ['theme'] }), 'Controls theme');
});

test('plugin settings values can be round-tripped through the editor', () => {
  assert.deepEqual(pluginSettingValues(record), { theme: 'ember', showFeatured: false, columns: 2 });
});

test('runtime kinds select the triggers offered during consent', () => {
  const isolated = { runtime: 'isolated-ui', executable: true, apiVersion: 2, trigger: null };
  const homeCards = { runtime: 'isolated-ui', executable: true, apiVersion: 3, trigger: null };
  const background = { runtime: 'background', executable: true, apiVersion: 2, trigger: null };
  assert.equal(pluginRuntimeKind(isolated), 'isolated-ui');
  assert.equal(pluginRuntimeLabel(isolated), 'Plugin UI');
  assert.equal(pluginRuntimeKind(background), 'background');
  assert.equal(pluginRuntimeLabel(background), 'Background service');
  assert.equal(pluginRuntimeKind({ runtime: 'future-kind', executable: true }), 'isolated-ui');
  assert.equal(pluginRuntimeKind({ runtime: 'declarative', executable: false }), 'declarative');
  assert.deepEqual(pluginTriggers(isolated), ['launcher-open']);
  assert.deepEqual(pluginTriggers(homeCards), ['on-demand']);
  assert.deepEqual(pluginTriggers(background), ['launcher-open', 'startup', 'always']);
  assert.deepEqual(pluginTriggers({ runtime: 'declarative', executable: false }), []);
  assert.equal(pluginDefaultTrigger({ ...background, trigger: 'always' }), 'always');
  assert.equal(pluginDefaultTrigger({ ...background, trigger: 'startup' }), 'startup');
  assert.equal(pluginDefaultTrigger({ ...background, trigger: 'invalid' }), 'launcher-open');
  assert.equal(pluginDefaultTrigger(isolated), 'launcher-open');
  assert.equal(pluginDefaultTrigger(homeCards), 'on-demand');
});

test('background triggers explain what keeps running outside the launcher', () => {
  assert.match(pluginTriggerDescription('on-demand'), /only when you choose its Home card/);
  assert.match(pluginTriggerDescription('on-demand'), /press Back/);
  assert.match(pluginTriggerSummary('on-demand'), /only when you open it/);
  assert.match(pluginTriggerDescription('launcher-open'), /Hiding Luma to the tray stops it/);
  assert.match(pluginTriggerSummary('launcher-open'), /while Luma is open/);
  assert.match(pluginTriggerDescription('startup'), /including when Luma opens in the tray/);
  assert.match(pluginTriggerDescription('startup'), /keeps running after the window closes/);
  assert.match(pluginTriggerDescription('always'), /while Luma is in the tray/);
  assert.match(pluginTriggerDescription('always'), /until you disable it/);
  assert.match(pluginTriggerSummary('always'), /including in the tray/);
});

test('permission review uses the exact declared capability names', () => {
  assert.match(pluginPermissionDescription('settings.read'), /own settings/);
  assert.match(pluginPermissionDescription('network.get'), /through Luma/);
  assert.match(pluginPermissionDescription('playback.read'), /what is playing/);
  assert.match(pluginPermissionDescription('playback.control'), /Plays, pauses, skips/);
  assert.equal(pluginPermissionDescription('custom.access'), 'Access declared by this package.');
});

test('failed runtime state is visible and recoverable through re-enable', () => {
  assert.equal(pluginRuntimeStateLabel('starting'), 'Starting');
  assert.equal(pluginRuntimeStateLabel('running'), 'Running');
  assert.equal(pluginRuntimeStateLabel('failed'), 'Failed');
  assert.equal(pluginRuntimeStateLabel('paused'), 'Paused');
  assert.equal(pluginRuntimeTone('running'), 'is-good');
  assert.equal(pluginRuntimeTone('failed'), 'is-warn');
  assert.equal(pluginRuntimeTone('starting'), 'is-muted');
});
