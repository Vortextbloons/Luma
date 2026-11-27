import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { stripTypeScriptTypes } from 'node:module';

const source = stripTypeScriptTypes(readFileSync(new URL('./widgetModel.ts', import.meta.url), 'utf8').replace(/^import .*;\r?\n/gm, ''));
const {
  addWidget, applyPlacementPreset, applyStylePreset, defaultWidgets, sanitizeWidgets,
  visibleInContext, widgetInstances,
} = await import(`data:text/javascript;base64,${Buffer.from(source).toString('base64')}`);

test('legacy Spotify widgets gain the instance defaults without losing their choices', () => {
  const widgets = sanitizeWidgets({ spotify: { enabled: false, placement: 'bottom-right', visibility: 'playing', position: { x: 125, y: 700 } } });
  assert.deepEqual(widgets.spotify, {
    type: 'spotify', enabled: false, placement: 'bottom-right', visibility: 'playing',
    position: { x: 125, y: 700 }, size: 'medium', variant: 'glass',
    presence: ['home', 'idle'], order: 0, settings: {},
  });
});

test('multiple instances keep their own identity, order, and settings', () => {
  const widgets = sanitizeWidgets({
    spotify: { type: 'spotify', placement: 'bar' },
    clock: { type: 'clock', placement: 'top-right', order: 2, settings: { hour12: true, removed: 'x' } },
    'clock-2': { type: 'clock', placement: 'rail-right', order: 1, settings: { showDate: false } },
  });
  assert.deepEqual(Object.keys(widgets), ['spotify', 'clock', 'clock-2']);
  assert.deepEqual(widgetInstances(widgets).map(item => item.id), ['spotify', 'clock-2', 'clock']);
  assert.deepEqual(widgets.clock.settings, { hour12: true, showDate: true });
  assert.deepEqual(widgets['clock-2'].settings, { hour12: false, showDate: false });
});

test('invalid widget values fall back to the type defaults', () => {
  const widgets = sanitizeWidgets({
    clock: {
      type: 'clock', placement: 'center', size: 'huge', variant: 'not a variant',
      presence: ['sometimes'], position: { x: 1001, y: 0 },
    },
  });
  assert.equal(widgets.clock.placement, 'top-right');
  assert.equal(widgets.clock.size, 'small');
  assert.equal(widgets.clock.variant, 'minimal');
  assert.deepEqual(widgets.clock.presence, ['home', 'music', 'idle']);
  assert.equal(widgets.clock.position, null);
});

test('presence distinguishes the player from idle', () => {
  const [item] = Object.values(sanitizeWidgets({ clock: { type: 'clock', presence: ['idle'] } }));
  assert.equal(visibleInContext(item, 'idle'), true);
  assert.equal(visibleInContext(item, 'home'), false);
  const [home] = Object.values(sanitizeWidgets({ spotify: { placement: 'bar' } }));
  assert.equal(visibleInContext(home, 'home'), true);
  assert.equal(visibleInContext(home, 'idle'), true);
  assert.equal(visibleInContext(home, 'music'), false);
});

test('placement and style presets update every instance and clear custom positions', () => {
  const widgets = sanitizeWidgets({
    spotify: { placement: 'bar', position: { x: 500, y: 900 } },
    clock: { type: 'clock', placement: 'bottom-left' },
    'clock-2': { type: 'clock', placement: 'top-center' },
  });
  applyPlacementPreset(widgets, 'dock');
  assert.equal(widgets.spotify.placement, 'bar');
  assert.equal(widgets.spotify.position, null);
  assert.equal(widgets.clock.placement, 'rail-right');
  applyStylePreset(widgets, 'cinematic');
  assert.equal(widgets.spotify.variant, 'cinematic');
  assert.equal(widgets.clock.variant, 'cinematic');
  delete widgets.spotify;
  applyPlacementPreset(widgets, 'dock');
  assert.equal(widgets.clock.placement, 'rail-right');
});

test('adding widgets respects limits and gives each instance a stable id', () => {
  const widgets = defaultWidgets();
  assert.equal(addWidget(widgets, 'spotify'), null);
  assert.equal(addWidget(widgets, 'clock'), 'clock');
  assert.equal(addWidget(widgets, 'clock'), 'clock-2');
  assert.equal(widgets['clock-2'].type, 'clock');
  assert.equal(widgets['clock-2'].order, 2);
});
