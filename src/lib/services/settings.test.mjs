import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { stripTypeScriptTypes } from 'node:module';

const themes = ['violet-night', 'sapphire', 'mint', 'ember', 'graphite', 'onyx'].map(id => ({ id, name: id }));
const source = stripTypeScriptTypes(readFileSync(new URL('./settings.ts', import.meta.url), 'utf8').replace(/^import .*;\r?\n/gm, ''));
const moduleSource = `const defaultThemeId = 'violet-night'; const themes = ${JSON.stringify(themes)};\n${source}`;
const { applySettingValue, resetSettingChoices, settingChoices, settingValue } = await import(`data:text/javascript;base64,${Buffer.from(moduleSource).toString('base64')}`);

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
  smoothAppHover: true,
  home: [{ id: 'hero', component: 'hero', visible: true, span: 2 }],
  homeColumns: 1,
  widgets: { spotify: { enabled: true, placement: 'bar' } },
  paths: { Steam: '', Spotify: '', Stremio: '', Netflix: '', 'Disney+': '', 'Prime Video': '', 'Apple TV+': '', Hulu: '', Max: '', Peacock: '', Custom: '' },
  order: ['YouTube', 'Spotify', 'Stremio', 'Steam', 'Netflix', 'Disney+', 'Prime Video', 'Apple TV+', 'Hulu', 'Max', 'Peacock', 'Custom'],
});

test('desktop choices expose sections, kinds, and options', () => {
  assert.deepEqual(settingChoices.map(choice => choice.key), ['theme', 'appTileShape', 'density', 'textSize', 'reducedMotion', 'smoothAppHover', 'interfaceSounds', 'soundLevel']);
  assert.ok(settingChoices.every(choice => choice.section && choice.options.length > 0));
  assert.equal(settingChoices.find(choice => choice.key === 'reducedMotion').kind, 'toggle');
  assert.equal(settingChoices.find(choice => choice.key === 'smoothAppHover').kind, 'toggle');
});

test('native-owned choices stay separate from local window and shortcut preferences', () => {
  const value = settings();
  applySettingValue(value, 'theme', 'onyx');
  assert.equal(value.theme, 'onyx');
  assert.equal(value.fullscreen, false);
  assert.equal(value.fastAppSwitch, true);
  assert.equal(value.homeShortcut, 'Ctrl+W');
});

test('choice changes round-trip and invalid values are ignored', () => {
  const value = settings();
  assert.equal(applySettingValue(value, 'reducedMotion', 'true'), true);
  assert.equal(value.reducedMotion, true);
  assert.equal(settingValue(value, 'reducedMotion'), 'true');
  assert.equal(applySettingValue(value, 'reducedMotion', 'yes'), false);
  assert.equal(value.reducedMotion, true);
  assert.equal(applySettingValue(value, 'textSize', 'larger'), true);
  assert.equal(value.textSize, 'larger');
});

test('scoped resets leave unrelated preferences and accounts alone', () => {
  const value = settings();
  value.theme = 'onyx';
  value.density = 'compact';
  value.textSize = 'larger';
  value.reducedMotion = true;
  value.smoothAppHover = false;
  value.interfaceSounds = 'off';
  value.soundLevel = 'loud';
  resetSettingChoices(value, ['density', 'appTileShape', 'textSize', 'reducedMotion', 'smoothAppHover']);
  assert.equal(value.density, 'comfortable');
  assert.equal(value.appTileShape, 'wide');
  assert.equal(value.textSize, 'standard');
  assert.equal(value.reducedMotion, false);
  assert.equal(value.smoothAppHover, true);
  assert.equal(value.theme, 'onyx');
  assert.equal(value.interfaceSounds, 'off');
  assert.equal(value.soundLevel, 'loud');
  assert.equal(value.homeShortcut, 'Ctrl+W');
});
