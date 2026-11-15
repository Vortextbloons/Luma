import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import vm from 'node:vm';

const script = readFileSync(new URL('./sports.js', import.meta.url), 'utf8');

function boot(sourceIndex = 0, framed = false, storage = new Map()) {
  let now = 1000, timerId = 0;
  const timers = new Map();
  const selected = [], sent = [], listeners = {}, windowListeners = {};
  let document;
  const node = tag => ({ tag, children: [], dataset: {}, style: { cssText: '', setProperty() {} },
    setAttribute(key, value) { this[key] = value; },
    append(...children) { this.children.push(...children); },
    addEventListener(name, callback) { this[name] = callback; },
    querySelectorAll(tag) { return this.children.flatMap(child => [...(child.tag === tag ? [child] : []), ...child.querySelectorAll(tag)]); },
    contains(element) { return this.children.includes(element) || this.children.some(child => child.contains(element)); },
    focus() { document.activeElement = this; this.focusEvent?.(); }, blur() { document.activeElement = null; },
  });
  const stream = node('iframe'); stream.id = 'cx-iframe'; stream.contentWindow = { postMessage: value => sent.push(value) };
  document = { title: 'Example game', readyState: 'complete', activeElement: null, body: node('body'), documentElement: node('html'),
    addEventListener: (name, callback) => { listeners[name] = callback; }, createElement: node,
    getElementById: id => id === 'cx-iframe' ? stream : null,
    querySelectorAll: selector => selector === '[id^="stream-btn-"]' ? [0, 1, 2].map(index => ({ id: `stream-btn-${index}`, click: () => selected.push(index) })) : selector === 'iframe' ? [stream] : [],
  };
  stream.parentElement = document.body;
  const location = { protocol: 'https:', hostname: framed ? 'player.example' : 'isportsurge.ws', pathname: '/watch/nba/example', href: '' };
  const window = { __lumaSportsSource: sourceIndex, addEventListener: (name, callback) => { windowListeners[name] = callback; } };
  window.top = framed ? {} : window; window.parent = framed ? { postMessage: value => sent.push(value) } : window;
  vm.runInNewContext(script, { window, document, location, crypto: { randomUUID: () => 'frame-id' }, MutationObserver: class { observe() {} disconnect() {} }, setTimeout: (callback, delay) => { const id = ++timerId; timers.set(id, { callback, at: now + delay }); return id; }, clearTimeout: id => timers.delete(id), Date: class extends Date { static now() { return now; } }, localStorage: { getItem: key => storage.get(key), setItem: (key, value) => storage.set(key, value) }, console });
  const overlay = document.body.children[0];
  const advance = milliseconds => {
    const end = now + milliseconds;
    while (true) {
      const pending = [...timers].filter(([, timer]) => timer.at <= end).sort((a, b) => a[1].at - b[1].at)[0];
      if (!pending) break;
      now = pending[1].at;
      timers.delete(pending[0]);
      pending[1].callback();
    }
    now = end;
  };
  return { advance, storage, selected, sent, location, listeners, windowListeners, window, stream, overlay, controls: overlay?.querySelectorAll('button') ?? [] };
}

test('source selection fills the video frame and manual alternatives wrap', () => {
  const state = boot(2);
  assert.deepEqual(state.selected, [2]);
  assert.match(state.stream.style.cssText, /width:100vw!important/);
  assert.match(state.stream.style.cssText, /height:100vh!important/);
  state.controls[5].click();
  assert.deepEqual(state.selected, [2, 0]);
});

test('remote playback commands target the embedded video and expose its seek window', () => {
  const state = boot();
  state.windowListeners.message({ source: state.stream.contentWindow, data: { channel: 'luma-sports-player-v1', type: 'state', id: 'nested-video', area: 100000, paused: false, position: 20, duration: 120 } });
  state.window.__lumaSportsControl('toggle');
  assert.equal(state.sent.at(-1).target, 'nested-video');
  assert.equal(state.sent.at(-1).action, 'toggle');
  assert.deepEqual(JSON.parse(JSON.stringify(state.window.__lumaSportsTimeline(45))), { position: 45, duration: 120 });
  assert.equal(state.sent.at(-1).action, 'seek');
  assert.equal(state.sent.at(-1).position, 45);
});

test('unrelated windows cannot spoof playback state', () => {
  const state = boot();
  state.windowListeners.message({ source: {}, data: { channel: 'luma-sports-player-v1', type: 'state', id: 'ad', area: 100000, duration: 120 } });
  assert.equal(state.window.__lumaSportsTimeline(null), null);
});

test('Back dismisses controls before returning and Ctrl+W always returns', () => {
  const state = boot();
  state.controls[0].click();
  assert.equal(state.location.href, 'https://isportsurge.ws/__luma_close__');
  state.location.href = '';
  state.listeners.keydown({ key: 'w', ctrlKey: true, preventDefault() {}, stopImmediatePropagation() {} });
  assert.equal(state.location.href, 'https://isportsurge.ws/__luma_close__');
});

test('nested player frames do not receive duplicate Luma controls', () => {
  const state = boot(0, true);
  assert.equal(state.controls.length, 0);
  assert.deepEqual(state.selected, []);
});

test('automatic selection tries each stalled source once and stops when exhausted', () => {
  const state = boot();
  state.advance(36000);
  assert.deepEqual(state.selected, [0, 1, 2]);
  assert.match(state.overlay.children[1].textContent, /None of this game/);
  state.advance(60000);
  assert.deepEqual(state.selected, [0, 1, 2]);
});

test('actual video progress stops fallback and remembers the successful source', () => {
  const state = boot(1);
  const report = position => state.windowListeners.message({ source: state.stream.contentWindow, data: { channel: 'luma-sports-player-v1', type: 'state', id: 'working-video', area: 100000, paused: false, waiting: false, position: 10, playbackTime: position, duration: 120 } });
  report(10);
  state.advance(1000);
  report(11);
  state.advance(30000);
  assert.deepEqual(state.selected, [1]);
  const reopened = boot(0, false, state.storage);
  assert.deepEqual(reopened.selected, [1]);
});

test('ready metadata without video progress does not qualify as a working stream', () => {
  const state = boot();
  state.windowListeners.message({ source: state.stream.contentWindow, data: { channel: 'luma-sports-player-v1', type: 'state', id: 'stuck-video', area: 100000, paused: false, waiting: false, position: 0, playbackTime: 0, duration: 120 } });
  state.advance(12000);
  assert.deepEqual(state.selected, [0, 1]);
});
