import test from 'node:test';
import assert from 'node:assert/strict';
import { smoothHeight } from './motion.ts';

function setup(reduced = false) {
  let resize;
  let disconnected = false;
  let removed = false;
  const preference = { matches: reduced, addEventListener() {}, removeEventListener() { removed = true; } };
  class Element {
    height = 300;
    style = { overflow: '' };
    animations = [];
    getBoundingClientRect() { return { height: this.height }; }
    animate(frames) {
      const animation = { frames, cancelled: false, cancel() { this.cancelled = true; } };
      this.animations.push(animation);
      return animation;
    }
  }
  const originals = { HTMLElement: globalThis.HTMLElement, ResizeObserver: globalThis.ResizeObserver, window: globalThis.window };
  globalThis.HTMLElement = Element;
  globalThis.ResizeObserver = class { constructor(callback) { resize = callback; } observe() {} disconnect() { disconnected = true; } };
  globalThis.window = { matchMedia: () => preference };
  const outer = new Element();
  const inner = new Element();
  outer.firstElementChild = inner;
  const cleanup = smoothHeight(outer);
  return { outer, resize(height) { inner.height = height; resize(); }, cleanup() { cleanup(); Object.assign(globalThis, originals); }, disposed: () => disconnected && removed };
}

test('content resizing animates collapse and retargets rapid changes from the visible height', () => {
  const harness = setup();
  try {
    harness.resize(100);
    const first = harness.outer.animations[0];
    assert.deepEqual(first.frames, [{ height: '300px' }, { height: '100px' }]);
    harness.outer.height = 240;
    harness.resize(400);
    assert.equal(first.cancelled, true);
    assert.deepEqual(harness.outer.animations[1].frames, [{ height: '240px' }, { height: '400px' }]);
    harness.outer.animations[1].onfinish();
    assert.equal(harness.outer.style.overflow, '');
  } finally { harness.cleanup(); }
  assert.equal(harness.disposed(), true);
});

test('reduced motion updates immediately without creating an animation', () => {
  const harness = setup(true);
  try { harness.resize(100); assert.equal(harness.outer.animations.length, 0); }
  finally { harness.cleanup(); }
});
