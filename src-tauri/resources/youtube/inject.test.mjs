import { readFileSync } from 'node:fs';
import vm from 'node:vm';
import test from 'node:test';
import assert from 'node:assert/strict';
const source = readFileSync(new URL('./inject.js', import.meta.url), 'utf8');
test('remote search opens YouTube results without a focused text input', () => {
  const start = source.indexOf('  window.__lumaTypeSearch =');
  assert.notEqual(start, -1, 'YouTube must expose remote search');
  const end = source.indexOf('  window.__lumaNavigate', start);
  const context = vm.createContext({ window: {}, location: { hash: '' }, encodeURIComponent });
  vm.runInContext(source.slice(start, end), context);
  context.window.__lumaTypeSearch('music & live');
  assert.equal(context.location.hash, '#/search?query=music%20%26%20live');
});
