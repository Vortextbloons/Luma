import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { compileModule } from 'svelte/compiler';
import ts from 'typescript';
import { loadCatalog, loadPopularShows } from './catalog.ts';
import { request } from '../../../remote/src/lib/api.ts';

let sequence = 0;
async function loadState(path, dependencies, component = false) {
  let source = readFileSync(new URL(path, import.meta.url), 'utf8');
  if (component) source = source.match(/<script lang="ts">([\s\S]*?)<\/script>/)[1]
    .replace('$props()', '({ native: true, autoConnect: true, onconnected: connected })') + '\nexport { refresh, poll };';
  source = ts.transpileModule(source, { compilerOptions: { target: ts.ScriptTarget.ESNext, module: ts.ModuleKind.ESNext } }).outputText
    .replace(/^import[\s\S]*?from ['"][^'"]+['"];?\r?\n/gm, '');
  globalThis.__lumaRecovery = dependencies;
  const compiled = compileModule(`const { ${Object.keys(dependencies).join(', ')} } = globalThis.__lumaRecovery;\n${source}`, { filename: `recovery-${++sequence}.svelte.js`, generate: 'client' }).js.code
    .replace(/['"]svelte\/internal\/client['"]/g, JSON.stringify(import.meta.resolve('svelte/internal/client')));
  try { return await import(`data:text/javascript;base64,${Buffer.from(compiled).toString('base64')}`); }
  finally { delete globalThis.__lumaRecovery; }
}

function deferred() {
  let resolve, reject;
  const promise = new Promise((yes, no) => { resolve = yes; reject = no; });
  return { promise, resolve, reject };
}

test('catalog keeps home rows, loads shows on demand, and recovers on retry', async () => {
  const original = globalThis.fetch;
  let failing = true;
  const requested = [];
  globalThis.fetch = async url => {
    requested.push(String(url));
    return { ok: !(failing && String(url).includes('imdbRating')), status: 503, json: async () => ({ metas: [{ id: 'tt1', name: 'Movie' }] }) };
  };
  try {
    const partial = await loadCatalog();
    assert.equal(partial.popular.length, 1);
    assert.equal(partial.featured, undefined);
    assert.deepEqual(partial.unavailable, ['Top-rated movies']);
    assert.equal(requested.some(url => url.includes('series/top')), false);
    assert.equal((await loadPopularShows()).length, 1);
    failing = false;
    const recovered = await loadCatalog();
    assert.equal(recovered.featured.length, 1);
    assert.deepEqual(recovered.unavailable, []);
  } finally { globalThis.fetch = original; }
});

test('catalog publishes each home row as soon as it settles', async () => {
  const original = globalThis.fetch;
  const requests = new Map();
  globalThis.fetch = url => new Promise(resolve => requests.set(String(url), resolve));
  const updates = [];
  const response = name => ({ ok: true, status: 200, json: async () => ({ metas: [{ id: 'tt1', name }] }) });
  const request = pattern => [...requests.entries()].find(([url]) => url.includes(pattern))[1];
  try {
    const pending = loadCatalog(update => updates.push(update));
    assert.equal(requests.size, 2);
    request('movie/top.json')(response('Popular'));
    await new Promise(resolve => setImmediate(resolve));
    assert.deepEqual(updates.map(update => Object.keys(update)[0]), ['popular']);
    let finished = false;
    void pending.then(() => { finished = true; });
    await new Promise(resolve => setImmediate(resolve));
    assert.equal(finished, false);
    request('imdbRating')(response('Featured'));
    const result = await pending;
    assert.equal(updates.length, 2);
    assert.equal(result.featured[0].title, 'Featured');
  } finally { globalThis.fetch = original; }
});

test('phone requests time out through body reads and can retry', async () => {
  const originalFetch = globalThis.fetch;
  const originalTimeout = globalThis.setTimeout;
  globalThis.setTimeout = (callback, delay) => originalTimeout(callback, delay === 15000 ? 5 : delay);
  globalThis.fetch = async (_url, options) => ({ json: () => new Promise((_resolve, reject) => options.signal.addEventListener('abort', () => reject(new Error('aborted')), { once: true })) });
  try {
    await assert.rejects(request('/phone/state'), /aborted/);
    globalThis.fetch = async () => ({ json: async () => ({ online: true }) });
    assert.deepEqual((await request('/phone/state')).data, { online: true });
  } finally { globalThis.fetch = originalFetch; globalThis.setTimeout = originalTimeout; }
});

test('Steam reuses fresh scans, forces return refresh, and retries failed scans', async () => {
  let scans = 0;
  let fail = false;
  const { createSteam } = await loadState('../state/steam.svelte.ts', {
    native: true,
    steamRecentGames: async () => { scans++; if (fail) throw new Error('Unavailable'); return []; },
  });
  const steam = createSteam();
  await steam.refresh();
  await steam.refresh();
  assert.equal(scans, 1);
  await steam.refresh(true);
  assert.equal(scans, 2);
  const other = createSteam();
  fail = true;
  await other.refresh();
  fail = false;
  await other.refresh();
  assert.equal(scans, 4);
});

test('Stremio completion after closing cannot start polling or report connection', async () => {
  const status = deferred();
  let cleanup;
  let intervals = 0;
  let connections = 0;
  let starts = 0;
  const previousWindow = globalThis.window;
  globalThis.window = { setInterval: () => { intervals++; return 1; }, clearInterval() {} };
  try {
    await loadState('../components/StremioIntegration.svelte', {
      onMount: callback => { cleanup = callback(); },
      connected: () => { connections++; },
      stremioStatus: () => status.promise,
      stremioLinkStart: async () => { starts++; return { linking: {} }; },
    }, true);
    cleanup();
    status.resolve({ connected: false, linking: null });
    await Promise.resolve();
    await Promise.resolve();
    assert.equal(starts, 0);
    assert.equal(intervals, 0);
    assert.equal(connections, 0);
  } finally { globalThis.window = previousWindow; }
});

test('Stremio pending poll cannot notify or restart after disposal', async () => {
  for (const failed of [false, true]) {
    const pending = deferred();
    let cleanup;
    let intervals = 0;
    let connections = 0;
    const previousWindow = globalThis.window;
    globalThis.window = { setInterval: () => { intervals++; return 1; }, clearInterval() {} };
    try {
      const integration = await loadState('../components/StremioIntegration.svelte', {
        onMount: callback => { cleanup = callback(); },
        connected: () => { connections++; },
        stremioStatus: async () => ({ connected: false, linking: {} }),
        stremioLinkPoll: () => pending.promise,
      }, true);
      await Promise.resolve();
      const polling = integration.poll();
      cleanup();
      if (failed) pending.reject(new Error('Network lost'));
      else pending.resolve({ connected: true, linking: null });
      await polling;
      assert.equal(intervals, 1);
      assert.equal(connections, 0);
    } finally { globalThis.window = previousWindow; }
  }
});

test('Spotify discards old library, queue, devices, and failures across disconnect and reconnect', async () => {
  const home = deferred(), queue = deferred(), devices = deferred();
  let homeCalls = 0;
  let authenticated = true;
  const { createSpotify } = await loadState('../state/spotify.svelte.ts', {
    native: true,
    spotifyPlayback: async () => ({ playing: false }),
    spotifyStatus: async () => ({ authenticated }),
    spotifyDisconnect: async () => { authenticated = false; return { authenticated }; },
    spotifyConnect: async () => { authenticated = true; return { authenticated }; },
    spotifyHome: () => ++homeCalls === 1 ? home.promise : Promise.resolve({ playlists: [{ id: 'new-account' }], recentlyPlayed: [], albums: [], tracks: [], devices: [], warnings: [] }),
    spotifyQueue: () => queue.promise,
    spotifyDevices: () => devices.promise,
    spotifyBrowse: async () => ({}),
  });
  const spotify = createSpotify();
  await spotify.refresh();
  const pendingHome = spotify.loadHome(), pendingQueue = spotify.loadQueue(), pendingDevices = spotify.loadDevices();
  await spotify.disconnect();
  await spotify.connect();
  home.resolve({ playlists: [{ id: 'old-account' }] });
  queue.resolve({ tracks: ['old-account'] });
  devices.reject(new Error('Old account failure'));
  await Promise.all([pendingHome, pendingQueue, pendingDevices]);
  assert.equal(spotify.home.playlists[0].id, 'new-account');
  assert.equal(spotify.queue, null);
  assert.deepEqual(spotify.devices, []);
  assert.equal(spotify.devicesError, '');
  assert.equal(spotify.homeLoading, false);
  assert.equal(spotify.devicesLoading, false);
});
