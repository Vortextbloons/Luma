import { readFileSync } from 'node:fs';
import vm from 'node:vm';
import test from 'node:test';
import assert from 'node:assert/strict';
const source = readFileSync(new URL('./inject.js', import.meta.url), 'utf8');

test('return buttons hide after two idle seconds and reappear on input', () => {
  for (const path of ['./inject.js', '../stremio/inject.js']) {
    const script = readFileSync(new URL(path, import.meta.url), 'utf8');
    const start = script.indexOf('  function autoHideReturnButton(');
    const end = script.indexOf('  function addReturnButton()', start);
    const events = new Map();
    const rootEvents = new Map();
    const video = { paused: false, ended: false };
    const host = { style: {} };
    const root = { activeElement: null, addEventListener: (name, fn) => rootEvents.set(name, fn) };
    let timeout;
    let delay;
    vm.runInNewContext(script.slice(start, end) + '\n autoHideReturnButton(host, root);', {
      host, root, document: {
        querySelectorAll: () => [video],
        addEventListener: (name, fn) => events.set(name, fn),
      },
      window: { addEventListener() {} },
      setTimeout: (fn, ms) => { timeout = fn; delay = ms; return 1; }, clearTimeout() {}, queueMicrotask: fn => fn(),
    });
    assert.equal(host.style.display, '');
    assert.equal(delay, 2000);
    timeout();
    assert.equal(host.style.display, 'none');
    assert.equal(host.inert, true);
    events.get('pointermove')({ movementX: 1 });
    assert.equal(host.style.display, '');
    assert.equal(host.inert, false);
    timeout();
    assert.equal(host.style.display, 'none');
    let blurred = false;
    root.activeElement = { blur() { blurred = true; root.activeElement = null; } };
    events.get('keydown')();
    timeout();
    assert.equal(blurred, true);
    assert.equal(host.style.display, 'none');
    video.paused = true;
    events.get('pointerdown')();
    assert.equal(host.style.display, '');
    timeout();
    assert.equal(host.style.display, 'none');
  }
});
test('remote search opens YouTube results without a focused text input', () => {
  const start = source.indexOf('  window.__lumaTypeSearch =');
  assert.notEqual(start, -1, 'YouTube must expose remote search');
  const end = source.indexOf('  window.__lumaNavigate', start);
  const context = vm.createContext({ window: {}, location: { hash: '' }, encodeURIComponent });
  vm.runInContext(source.slice(start, end), context);
  context.window.__lumaTypeSearch('music & live');
  assert.equal(context.location.hash, '#/search?q=music%20%26%20live');
});
