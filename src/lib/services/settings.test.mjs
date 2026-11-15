import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { stripTypeScriptTypes } from 'node:module';

const themes = ['violet-night', 'sapphire', 'mint', 'ember', 'graphite', 'onyx'].map(id => ({ id, name: id }));
const source = stripTypeScriptTypes(readFileSync(new URL('./settings.ts', import.meta.url), 'utf8').replace(/^import .*;\r?\n/gm, ''));
const moduleSource = `const defaultThemeId = 'violet-night'; const themes = ${JSON.stringify(themes)};\n${source}`;
const { applyRemoteSetting, remoteSettings, resetSettingChoices, settingChoices, settingValue } = await import(`data:text/javascript;base64,${Buffer.from(moduleSource).toString('base64')}`);

const settings = () => ({
  theme: 'violet-night',
  density: 'comfortable',
  appTileShape: 'wide',
  homeShortcut: 'Ctrl+W',
  fullscreen: false,
  fastAppSwitch: true,
  interfaceSounds: 'real',
  soundLevel: 'standard',
  textSize: 'standard',
  reducedMotion: false,
  home: [{ id: 'hero', component: 'hero', visible: true, span: 2 }],
  homeColumns: 1,
  widgets: { spotify: { enabled: true, placement: 'bar' } },
  paths: { Steam: '', Spotify: '', Stremio: '', Netflix: '', 'Disney+': '', 'Prime Video': '', 'Apple TV+': '', Hulu: '', Max: '', Peacock: '', Custom: '' },
  order: ['YouTube', 'Spotify', 'Stremio', 'Steam', 'Netflix', 'Disney+', 'Prime Video', 'Apple TV+', 'Hulu', 'Max', 'Peacock', 'Custom'],
});

test('ordinary settings expose sections, kinds, and values for every surface', () => {
  const value = settings();
  const remote = remoteSettings(value);
  const keys = remote.map(choice => choice.key);
  assert.ok(settingChoices.every(choice => keys.includes(choice.key)));
  assert.ok(keys.includes('textSize'));
  assert.ok(keys.includes('reducedMotion'));
  assert.ok(keys.includes('homeColumns'));
  assert.ok(keys.includes('fullscreen'));
  assert.ok(keys.includes('fastAppSwitch'));
  assert.equal(remote.find(choice => choice.key === 'reducedMotion').kind, 'toggle');
  assert.equal(remote.find(choice => choice.key === 'reducedMotion').value, 'false');
  assert.equal(remote.find(choice => choice.key === 'homeColumns').value, '1');
  assert.equal(remote.find(choice => choice.key === 'fullscreen').value, 'false');
  assert.ok(remote.every(choice => choice.section && choice.options.length > 0));
});

test('remote-only settings round-trip through the same contract', () => {
  const value = settings();
  assert.equal(applyRemoteSetting(value, { key: 'fullscreen', value: 'true' }), true);
  assert.equal(value.fullscreen, true);
  assert.equal(applyRemoteSetting(value, { key: 'fastAppSwitch', value: 'false' }), true);
  assert.equal(value.fastAppSwitch, false);
  assert.equal(applyRemoteSetting(value, { key: 'homeColumns', value: '2' }), true);
  assert.equal(value.homeColumns, 2);
  assert.equal(applyRemoteSetting(value, { key: 'fullscreen', value: 'yes' }), false);
  assert.equal(value.fullscreen, true);
  assert.equal(applyRemoteSetting(value, { key: 'homeColumns', value: '3' }), false);
  assert.equal(value.homeColumns, 2);
});

test('remote boolean changes round-trip and invalid values are ignored', () => {
  const value = settings();
  assert.equal(applyRemoteSetting(value, { key: 'reducedMotion', value: 'true' }), true);
  assert.equal(value.reducedMotion, true);
  assert.equal(settingValue(value, 'reducedMotion'), 'true');
  assert.equal(applyRemoteSetting(value, { key: 'reducedMotion', value: 'yes' }), false);
  assert.equal(value.reducedMotion, true);
  assert.equal(applyRemoteSetting(value, { key: 'textSize', value: 'larger' }), true);
  assert.equal(value.textSize, 'larger');
});

test('scoped resets leave unrelated preferences and accounts alone', () => {
  const value = settings();
  value.theme = 'onyx';
  value.density = 'compact';
  value.textSize = 'larger';
  value.reducedMotion = true;
  value.interfaceSounds = 'off';
  value.soundLevel = 'loud';
  resetSettingChoices(value, ['density', 'appTileShape', 'textSize', 'reducedMotion']);
  assert.equal(value.density, 'comfortable');
  assert.equal(value.appTileShape, 'wide');
  assert.equal(value.textSize, 'standard');
  assert.equal(value.reducedMotion, false);
  assert.equal(value.theme, 'onyx');
  assert.equal(value.interfaceSounds, 'off');
  assert.equal(value.soundLevel, 'loud');
  assert.equal(value.homeShortcut, 'Ctrl+W');
});
