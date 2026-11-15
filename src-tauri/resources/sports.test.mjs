import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import vm from 'node:vm';

const script = readFileSync(new URL('./sports.js', import.meta.url), 'utf8');

function boot(sourceIndex = 0, framed = false, storage = new Map(), options = {}) {
  let now = 1000, timerId = 0;
  const timers = new Map();
  const selected = [], sent = [], listeners = {}, windowListeners = {};
  let document;
  const node = tag => ({ tag, children: [], dataset: {}, style: { cssText: '', setProperty() {} },
    setAttribute(key, value) { this[key] = value; }, getAttribute(key) { return this[key]; },
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
  let providerStarts = 0, nativePlays = 0;
  const media = options.provider ? Object.assign(node('video'), { addEventListener() {}, currentSrc: '', currentTime: 0, duration: 0, readyState: 0, paused: true, isConnected: true, seekable: { length: 0 }, getBoundingClientRect: () => ({ width: 1280, height: 720 }), play: async () => { nativePlays++; media.paused = false; }, pause: () => { media.paused = true; }, parentElement: document.body }) : null;
  if (media) {
    const previousQuery = document.querySelectorAll;
    document.querySelectorAll = selector => selector === 'video' ? [media] : selector === 'button[data-playpause], .play-wrapper[data-poster]' ? [{ click: () => { providerStarts++; media.currentSrc = 'blob:initialized'; media.readyState = 4; media.paused = false; } }] : previousQuery(selector);
  }
  const location = { protocol: 'https:', hostname: options.host || (framed ? 'player.example' : 'isportsurge.ws'), hash: options.hash || '', pathname: '/watch/nba/example', href: '' };
  const window = { __lumaSportsUrls: options.plans, __lumaSportsSource: sourceIndex, addEventListener: (name, callback) => { windowListeners[name] = callback; } };
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
  return { media, providerStarts: () => providerStarts, nativePlays: () => nativePlays, advance, storage, selected, sent, location, listeners, windowListeners, window, stream, overlay, controls: overlay?.querySelectorAll('button') ?? [] };
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

test('StreamEast advances through its alternatives before switching to Sportsurge', () => {
  const plans = ['https://golygoals.com/streame.html?key=one', 'https://streamec.st/embed/ch2.php', 'https://isportsurge.ws/watch/nfl/game'];
  const first = boot(0, false, new Map(), { host: 'golygoals.com', plans });
  first.advance(12000);
  assert.equal(first.location.href, plans[1] + '#luma-plan=1');
  const second = boot(0, false, new Map(), { host: 'streamec.st', plans, hash: '#luma-plan=1' });
  second.advance(12000);
  assert.equal(second.location.href, plans[2] + '#luma-plan=2');
  const last = boot(0, false, new Map(), { plans, hash: '#luma-plan=2' });
  last.advance(36000);
  assert.deepEqual(last.selected, [0, 1, 2]);
  assert.equal(last.location.href, '');
});

test('provider initialization uses its play control before controlling the raw video', async () => {
  const state = boot(0, true, new Map(), { provider: true });
  assert.equal(state.providerStarts(), 1);
  assert.equal(state.media.currentSrc, 'blob:initialized');
  assert.equal(state.nativePlays(), 0);
  const command = () => state.windowListeners.message({ source: state.window.parent, data: { channel: 'luma-sports-player-v1', type: 'command', target: 'frame-id', action: 'toggle' } });
  command();
  assert.equal(state.media.paused, true);
  command();
  await Promise.resolve();
  assert.equal(state.nativePlays(), 1);
  assert.equal(state.providerStarts(), 1);
});
