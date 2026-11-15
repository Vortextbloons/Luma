import test from 'node:test';
import assert from 'node:assert/strict';
import { defaultHomeRows, legacyHomeRows, moveHomeRow, moveHomeRowTo, sanitizeHomeRows, toggleHomeRow } from './home.ts';

test('saved layouts without a hero gain one at the top', () => {
  const rows = sanitizeHomeRows([
    { id: 'apps', visible: true },
    { id: 'continue', visible: false },
  ]);
  assert.equal(rows?.[0].id, 'hero');
  assert.equal(rows?.[1].id, 'apps');
  assert.equal(rows?.[2].id, 'continue');
  assert.equal(rows?.[2].visible, false);
  assert.equal(rows?.length, 7);
});

test('legacy section flags become a complete home layout', () => {
  const rows = legacyHomeRows({ continue: false, apps: true, popular: false });
  assert.equal(rows[0].id, 'hero');
  assert.equal(rows.find(row => row.id === 'continue')?.visible, false);
  assert.equal(rows.find(row => row.id === 'apps')?.visible, true);
  assert.equal(rows.find(row => row.id === 'popular')?.visible, false);
  assert.equal(rows.find(row => row.id === 'featured')?.visible, true);
});

test('moving and toggling rows keeps every row exactly once', () => {
  const moved = moveHomeRow(defaultHomeRows(), 'hero', 1);
  assert.deepEqual(moved.slice(0, 2).map(row => row.id), ['continue', 'hero']);
  const restored = moveHomeRowTo(moved, 'hero', 0);
  assert.deepEqual(restored.map(row => row.id), defaultHomeRows().map(row => row.id));
  const toggled = toggleHomeRow(restored, 'hero');
  assert.equal(toggled.find(row => row.id === 'hero')?.visible, false);
  assert.equal(new Set(toggled.map(row => row.id)).size, toggled.length);
});

test('instance identity survives sanitizing, moving, and toggling independently of component type', () => {
  const rows = defaultHomeRows().map(row => ({ ...row, id: `personal-${row.id}` }));
  const saved = sanitizeHomeRows(rows);
  assert.deepEqual(saved, rows);
  const moved = moveHomeRowTo(saved, 'personal-apps', 0);
  assert.equal(moved[0].component, 'apps');
  assert.equal(moved[0].id, 'personal-apps');
  assert.equal(toggleHomeRow(moved, 'personal-apps')[0].visible, false);
});

test('legacy rows gain component identities and missing components do not duplicate instance ids', () => {
  const migrated = sanitizeHomeRows([{ id: 'apps', visible: false }]);
  assert.equal(migrated.find(row => row.component === 'apps').visible, false);
  assert.equal(migrated.find(row => row.component === 'apps').id, 'apps');
  const repaired = sanitizeHomeRows([{ id: 'hero', component: 'apps', visible: true }]);
  assert.equal(repaired.find(row => row.component === 'apps').id, 'hero');
  assert.equal(new Set(repaired.map(row => row.id)).size, 7);
  assert.equal(new Set(repaired.map(row => row.component)).size, 7);
});
