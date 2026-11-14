import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import vm from 'node:vm';

const source = readFileSync(new URL('./inject.js', import.meta.url), 'utf8');

test('remote cursor stays hidden on stationary page events and returns on mouse movement', () => {
  const rust = readFileSync(new URL('../../src/remote/pointer.rs', import.meta.url), 'utf8');
  const hide = JSON.parse(rust.match(/const HIDE: &str = (".*");/)[1]);
  const show = JSON.parse(rust.match(/const SHOW: &str = (".*");/)[1]);
  const listeners = new Map();
  let style = null;
  const window = {
    addEventListener: (type, callback) => listeners.set(type, callback),
    removeEventListener: type => listeners.delete(type),
  };
  const document = {
    getElementById: () => style,
    createElement: () => ({ remove() { style = null; } }),
    documentElement: { appendChild(element) { style = element; } },
  };
  const context = vm.createContext({ window, document });
  vm.runInContext(hide, context);
  assert.ok(style);
  assert.equal(listeners.has('mousedown'), false);
  listeners.get('pointermove')({ type: 'pointermove', movementX: 0, movementY: 0 });
  assert.ok(style);
  listeners.get('pointermove')({ type: 'pointermove', movementX: 1, movementY: 0 });
  assert.equal(style, null);
  assert.equal(listeners.size, 0);
  vm.runInContext(hide, context);
  vm.runInContext(show, context);
  assert.equal(style, null);
  assert.equal(listeners.size, 0);
});

function boot({ index = 0, length = 1 } = {}) {
  const history = { length, back() { this.calls += 1; }, calls: 0 };
  const window = {
    top: null,
    innerWidth: 1920,
    innerHeight: 1080,
    navigation: { currentEntry: { index } },
    addEventListener() {},
  };
  window.top = window;
  const document = {
    readyState: 'complete',
    activeElement: null,
    documentElement: { appendChild() {} },
    head: { appendChild() {} },
    addEventListener() {},
    getElementById: () => null,
    querySelectorAll: () => [],
    createElement: () => ({ id: '', textContent: '', style: {} }),
    body: {},
  };
  vm.runInNewContext(source, { window, document, history, console });
  return { window, history, document };
}

test('directional focus picks the nearest candidate in the requested direction', () => {
  const { window } = boot();
  const current = { rect: { left: 0, top: 0, width: 100, height: 100 } };
  const right = { rect: { left: 200, top: 10, width: 100, height: 100 } };
  const below = { rect: { left: 10, top: 200, width: 100, height: 100 } };
  const left = { rect: { left: -200, top: 0, width: 100, height: 100 } };
  const candidates = [current, right, below, left];
  assert.equal(window.__lumaSpatial.choose(current, candidates, 'right'), right);
  assert.equal(window.__lumaSpatial.choose(current, candidates, 'down'), below);
  assert.equal(window.__lumaSpatial.choose(current, candidates, 'left'), left);
  assert.equal(window.__lumaSpatial.choose(current, candidates, 'up'), null);
  assert.equal(window.__lumaSpatial.choose(null, candidates, 'up'), current);
});

test('back returns to Luma only at the start of the service history', () => {
  const atStart = boot({ index: 0, length: 4 });
  assert.equal(atStart.window.__lumaBack(), 'home');
  assert.equal(atStart.history.calls, 0);
  const deeper = boot({ index: 2, length: 4 });
  assert.equal(deeper.window.__lumaBack(), 'back');
  assert.equal(deeper.history.calls, 1);
  const unknown = boot({ index: null, length: 1 });
  assert.equal(unknown.window.__lumaBack(), 'home');
});

test('a missing navigation entry falls back to history length', () => {
  assert.equal(boot({ index: null, length: 2 }).window.__lumaSpatial.backTarget(null, 2), 'back');
  assert.equal(boot({ index: null, length: 1 }).window.__lumaSpatial.backTarget(null, 1), 'home');
});

test('remote navigation preserves native editing and submission in form controls', () => {
  for (const tagName of ['INPUT', 'TEXTAREA', 'SELECT']) {
    const { window, document } = boot();
    document.activeElement = { tagName };
    document.querySelectorAll = () => { throw new Error('Editing must not collect navigation targets'); };
    for (const command of ['left', 'right', 'up', 'down', 'activate']) {
      assert.equal(window.__lumaNavigate(command), false, `${tagName}: ${command}`);
    }
  }
});
