import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import vm from 'node:vm';

const source = readFileSync(new URL('./inject.js', import.meta.url), 'utf8');
const account = { token: 'test-session', userId: 'linked-user' };

async function boot({ linked = account, current = null, managed = false, outcome = 'success', origin = 'https://web.stremio.com' } = {}) {
  const nodes = new Map();
  const storage = new Map(managed ? [['luma-stremio-account', 'previous-user']] : []);
  const timers = new Map();
  const calls = [];
  const events = [];
  const location = { origin, hostname: new URL(origin).hostname, hash: '#/detail/movie/tt123' };
  const window = { addEventListener() {} };
  window.top = window;
  const document = {
    readyState: 'loading',
    addEventListener() {},
    getElementById: id => nodes.get(id),
    documentElement: { append: node => nodes.set(node.id, node) },
    createElement: () => ({ setAttribute() {}, style: {}, remove() { nodes.delete(this.id); } }),
  };
  vm.runInNewContext(source.replace('__LUMA_STREMIO_SESSION__', JSON.stringify({ account: linked, target: location.hash })), {
    window, location, document, queueMicrotask,
    localStorage: { getItem: key => storage.get(key), setItem: (key, value) => storage.set(key, value), removeItem: key => storage.delete(key) },
    setTimeout: (fn, delay) => { const id = timers.size + 1; timers.set(id, { fn, delay }); return id; },
    clearTimeout: id => timers.delete(id),
  });
  window.core = {
    getState: async () => ({ profile: { auth: current ? { user: { _id: current } } : null } }),
    dispatch: async action => {
      calls.push(JSON.parse(JSON.stringify(action)));
      if (action.args.action !== 'Authenticate' || outcome === 'timeout') return;
      queueMicrotask(() => window.onCoreEvent({ name: 'CoreEvent', args: outcome === 'success'
        ? { event: 'UserAuthenticated', args: {} }
        : { event: 'Error', args: { source: { event: 'UserAuthenticated' } } } }));
    },
  };
  window.onCoreEvent = event => events.push(event);
  await new Promise(resolve => setImmediate(resolve));
  return { nodes, storage, timers, calls, events, location, window };
}

test('linked account authenticates through Stremio and restores the requested title', async () => {
  const state = await boot();
  assert.deepEqual(state.calls.map(call => call.args.action), ['Logout', 'Authenticate']);
  assert.deepEqual(state.calls[1].args.args, { type: 'LoginWithToken', token: account.token });
  assert.equal(state.storage.get('luma-stremio-account'), account.userId);
  assert.equal(state.nodes.size, 0);
  assert.equal(state.events.length, 1);
  state.location.hash = '#/intro';
  for (const timer of state.timers.values()) timer.fn();
  assert.equal(state.location.hash, '#/detail/movie/tt123');
});

test('persisted matching account is reused without signing in again', async () => {
  const state = await boot({ current: account.userId });
  assert.equal(state.calls.length, 0);
  assert.equal(state.storage.get('luma-stremio-account'), account.userId);
});

test('a different embedded account is replaced by the linked account', async () => {
  const state = await boot({ current: 'previous-user', managed: true });
  assert.deepEqual(state.calls.map(call => call.args.action), ['Logout', 'Authenticate']);
  assert.equal(state.storage.get('luma-stremio-account'), account.userId);
});

test('disconnect clears a managed session but preserves independent manual sign-in', async () => {
  const managed = await boot({ linked: null, current: 'previous-user', managed: true });
  assert.deepEqual(managed.calls.map(call => call.args.action), ['Logout']);
  assert.equal(managed.storage.size, 0);
  assert.equal(managed.location.hash, '#/intro');
  const manual = await boot({ linked: null, current: 'manual-user' });
  assert.equal(manual.calls.length, 0);
});

test('authentication errors and timeouts show recovery feedback without the credential', async () => {
  for (const outcome of ['error', 'timeout']) {
    const state = await boot({ outcome });
    if (outcome === 'timeout') for (const timer of state.timers.values()) timer.fn();
    const message = state.nodes.get('luma-account-status').textContent;
    assert.match(message, /reconnect Stremio/);
    assert.equal(message.includes(account.token), false);
  }
});

test('account handoff is restricted to the official web app origin', async () => {
  const state = await boot({ origin: 'https://other.stremio.com' });
  assert.equal(state.calls.length, 0);
  assert.equal(state.storage.size, 0);
});


test('reopening after embedded sign-out signs in again and keeps the latest title', async () => {
  const state = await boot({ current: account.userId });
  state.window.core.getState = async () => ({ profile: { auth: null } });
  await state.window.__lumaSyncAccount('#/detail/series/tt456');
  await new Promise(resolve => setImmediate(resolve));
  assert.deepEqual(state.calls.map(call => call.args.action), ['Logout', 'Authenticate']);
  for (const timer of state.timers.values()) timer.fn();
  assert.equal(state.location.hash, '#/detail/series/tt456');
});
