import { readFileSync } from 'node:fs';
import vm from 'node:vm';
import test from 'node:test';
import assert from 'node:assert/strict';

const source = readFileSync(new URL('./page.js', import.meta.url), 'utf8');

const ids = [
  'identity',
  'ping-result',
  'denial',
  'note',
  'now-playing',
  'playback-connection',
  'playback-title',
  'playback-artist',
  'playback-playstate',
  'playback-status',
  'playback-controls',
  'playback-previous',
  'playback-toggle',
  'playback-next',
  'playback-control-status',
  'refresh',
  'ping',
  'probe',
  'crash',
  'close',
];

const buttons = [
  'playback-previous',
  'playback-toggle',
  'playback-next',
  'refresh',
  'ping',
  'probe',
  'crash',
  'close',
];

const flush = () => new Promise(resolve => setImmediate(resolve));

function createPage() {
  const document = { activeElement: null, body: { id: 'body' } };
  document.activeElement = document.body;

  const nodes = new Map();
  for (const id of ids) {
    const classes = new Set();
    const listeners = new Map();
    const node = {
      id,
      textContent: '',
      hidden: false,
      disabled: false,
      dataset: {},
      attributes: {},
      focusCount: 0,
      listeners,
      classList: {
        contains: name => classes.has(name),
        toggle(name, on) { if (on) classes.add(name); else classes.delete(name); },
      },
      addEventListener: (name, listener) => listeners.set(name, listener),
      setAttribute(name, value) { node.attributes[name] = value; },
      focus() { node.focusCount++; document.activeElement = node; },
    };
    nodes.set(id, node);
  }

  document.querySelector = selector => nodes.get(selector.slice(1)) ?? null;
  document.querySelectorAll = selector => {
    assert.equal(selector, 'button:not(:disabled)');
    return buttons.map(id => nodes.get(id)).filter(node => !node.disabled);
  };

  const calls = [];
  const pending = [];
  const windowListeners = new Map();
  const window = {
    __TAURI_INTERNALS__: {
      invoke(command, args) {
        calls.push({ command, args: { ...args } });
        if (args.action === 'identity') {
          return Promise.resolve({ package: 'com.luma.spike', version: '1.0.0', origin: 'spike://page' });
        }
        if (args.action === 'close') return Promise.resolve();
        return new Promise((resolve, reject) => pending.push({ resolve, reject }));
      },
    },
    addEventListener: (name, listener) => windowListeners.set(name, listener),
  };

  vm.runInNewContext(source, { window, document, history: { pushState() {} } });

  const press = key => {
    const event = { key, defaultPrevented: false, preventDefault() { event.defaultPrevented = true; } };
    windowListeners.get('keydown')(event);
    return event;
  };

  return {
    document,
    node: id => nodes.get(id),
    calls,
    pending,
    callsFor: action => calls.filter(call => call.args.action === action),
    click: id => nodes.get(id).listeners.get('click')(),
    press,
  };
}

test('playback refresh asks the host for the snapshot without extra arguments', () => {
  const page = createPage();
  assert.deepEqual(page.callsFor('playback'), [
    { command: 'extension_spike_action', args: { action: 'playback' } },
  ]);
});

test('a linked playback snapshot fills the now playing card', async () => {
  const page = createPage();
  page.pending.shift().resolve({ connected: true, title: 'Blue in Green', artist: 'Miles Davis', playing: true });
  await flush();
  assert.equal(page.node('now-playing').dataset.state, 'live');
  assert.equal(page.node('playback-connection').textContent, 'Spotify account linked');
  assert.equal(page.node('playback-title').textContent, 'Blue in Green');
  assert.equal(page.node('playback-artist').textContent, 'Miles Davis');
  assert.equal(page.node('playback-playstate').hidden, false);
  assert.equal(page.node('playback-playstate').dataset.playing, 'true');
  assert.equal(page.node('playback-playstate').textContent, 'Playing');
  assert.match(page.node('playback-status').textContent, /^Read .+ Showing the last published host snapshot\.$/);
});

test('a connected host with nothing playing reports an empty card', async () => {
  const page = createPage();
  page.pending.shift().resolve({ connected: true, title: null, artist: null, playing: false });
  await flush();
  assert.equal(page.node('now-playing').dataset.state, 'empty');
  assert.equal(page.node('playback-title').textContent, 'Nothing playing');
  assert.equal(page.node('playback-artist').textContent, '');
  assert.equal(page.node('playback-playstate').hidden, true);
  assert.match(page.node('playback-status').textContent, /Nothing is playing\./);
});

test('a host without a Spotify account reports a disconnected card', async () => {
  const page = createPage();
  page.pending.shift().resolve({ connected: false });
  await flush();
  assert.equal(page.node('now-playing').dataset.state, 'disconnected');
  assert.equal(page.node('playback-connection').textContent, 'Spotify not connected');
  assert.equal(page.node('playback-playstate').hidden, true);
  assert.match(page.node('playback-status').textContent, /Spotify is not connected\./);
});

test('a failed refresh keeps the last successful snapshot on screen', async () => {
  const page = createPage();
  page.pending.shift().resolve({ connected: true, title: 'So What', artist: 'Miles Davis', playing: true });
  await flush();
  page.click('refresh');
  assert.equal(page.callsFor('playback').length, 2);
  page.pending.shift().reject('host offline');
  await flush();
  assert.equal(page.node('now-playing').dataset.state, 'error');
  assert.equal(page.node('playback-title').textContent, 'So What');
  assert.equal(page.node('playback-artist').textContent, 'Miles Davis');
  assert.equal(page.node('playback-connection').textContent, 'Spotify account linked');
  assert.equal(page.node('playback-playstate').hidden, true);
  assert.match(page.node('playback-status').textContent, /Refresh failed: host offline Showing the last read\.$/);
});

test('playback refresh disables its control and never overlaps requests', async () => {
  const page = createPage();
  const refresh = page.node('refresh');
  assert.equal(refresh.disabled, true);
  assert.equal(refresh.attributes['aria-busy'], 'true');
  assert.equal(refresh.classList.contains('is-loading'), true);
  page.click('refresh');
  assert.equal(page.callsFor('playback').length, 1);
  page.pending.shift().resolve({ connected: true, title: 'So What' });
  await flush();
  assert.equal(refresh.disabled, false);
  assert.equal(refresh.attributes['aria-busy'], 'false');
  assert.equal(refresh.classList.contains('is-loading'), false);
  page.click('refresh');
  assert.equal(page.callsFor('playback').length, 2);
  page.pending.shift().resolve({ connected: true, title: 'So What' });
  await flush();
});

test('a refresh restores focus when the disabled control dropped it to the page', async () => {
  const page = createPage();
  page.pending.shift().resolve({ connected: true });
  await flush();
  const refresh = page.node('refresh');
  refresh.focus();
  page.click('refresh');
  page.document.activeElement = page.document.body;
  page.pending.shift().resolve({ connected: true, title: 'So What' });
  await flush();
  assert.equal(page.document.activeElement, refresh);
});

test('a refresh leaves focus alone when another button already has it', async () => {
  const page = createPage();
  page.pending.shift().resolve({ connected: true });
  await flush();
  const refresh = page.node('refresh');
  const ping = page.node('ping');
  refresh.focus();
  page.click('refresh');
  ping.focus();
  const focused = refresh.focusCount;
  page.pending.shift().resolve({ connected: true, title: 'So What' });
  await flush();
  assert.equal(page.document.activeElement, ping);
  assert.equal(refresh.focusCount, focused);
});

test('arrow keys move between enabled buttons and wrap at both ends', async () => {
  const page = createPage();
  page.pending.shift().resolve({ connected: true });
  await flush();
  assert.equal(page.document.activeElement, page.node('close'));

  assert.equal(page.press('ArrowRight').defaultPrevented, true);
  assert.equal(page.document.activeElement, page.node('refresh'));
  assert.equal(page.press('ArrowLeft').defaultPrevented, true);
  assert.equal(page.document.activeElement, page.node('close'));

  page.node('crash').disabled = true;
  page.node('probe').focus();
  page.press('ArrowRight');
  assert.equal(page.document.activeElement, page.node('close'));
  page.press('ArrowLeft');
  assert.equal(page.document.activeElement, page.node('probe'));
});

test('escape prevents the default action and closes the host window', async () => {
  const page = createPage();
  page.pending.shift().resolve({ connected: false });
  await flush();
  const event = page.press('Escape');
  assert.equal(event.defaultPrevented, true);
  assert.deepEqual(page.callsFor('close'), [
    { command: 'extension_spike_action', args: { action: 'close' } },
  ]);
});


test('an initial playback failure can recover through Refresh', async () => {
  const page = createPage();
  page.pending.shift().reject('host unavailable');
  await flush();
  assert.equal(page.node('now-playing').dataset.state, 'error');
  assert.equal(page.node('playback-title').textContent, 'Playback unavailable');
  assert.equal(page.node('refresh').disabled, false);
  page.click('refresh');
  page.pending.shift().resolve({ connected: true, title: 'Recovered track', artist: 'Artist', playing: false });
  await flush();
  assert.equal(page.node('now-playing').dataset.state, 'live');
  assert.equal(page.node('playback-title').textContent, 'Recovered track');
  assert.equal(page.node('playback-playstate').textContent, 'Paused');
  assert.doesNotMatch(page.node('playback-status').textContent, /unavailable|failed/);
});

test('BrowserBack prevents navigation and requests host closure', () => {
  const page = createPage();
  assert.equal(page.press('BrowserBack').defaultPrevented, true);
  assert.deepEqual(page.callsFor('close'), [
    { command: 'extension_spike_action', args: { action: 'close' } },
  ]);
});

const controlIds = ['playback-previous', 'playback-toggle', 'playback-next'];

test('playback controls stay disabled until a linked track is on the card', async () => {
  const page = createPage();
  for (const id of controlIds) assert.equal(page.node(id).disabled, true);

  page.pending.shift().resolve({ connected: false });
  await flush();
  for (const id of controlIds) assert.equal(page.node(id).disabled, true);

  page.click('refresh');
  page.pending.shift().resolve({ connected: true, title: null, playing: false });
  await flush();
  for (const id of controlIds) assert.equal(page.node(id).disabled, true);

  page.click('refresh');
  page.pending.shift().resolve({ connected: true, title: 'Blue in Green', playing: false });
  await flush();
  for (const id of controlIds) assert.equal(page.node(id).disabled, false);
});

test('controls ignore clicks until a linked track is available', () => {
  const page = createPage();
  page.click('playback-toggle');
  page.click('playback-next');
  assert.equal(page.callsFor('playback-toggle').length, 0);
  assert.equal(page.callsFor('playback-next').length, 0);
});

test('a control sends one argument-free command and reports acceptance without inventing state', async () => {
  const page = createPage();
  page.pending.shift().resolve({ connected: true, title: 'Blue in Green', artist: 'Miles Davis', playing: true });
  await flush();
  page.node('playback-toggle').focus();
  page.click('playback-toggle');

  assert.deepEqual(page.callsFor('playback-toggle'), [
    { command: 'extension_spike_action', args: { action: 'playback-toggle' } },
  ]);
  assert.equal(page.node('playback-control-status').textContent, 'Sending Pause…');
  assert.equal(page.node('playback-control-status').dataset.tone, 'pending');
  for (const id of controlIds) assert.equal(page.node(id).disabled, true);
  assert.equal(page.node('refresh').disabled, true);
  assert.equal(page.node('playback-toggle').attributes['aria-busy'], 'true');
  assert.equal(page.node('playback-playstate').textContent, 'Playing');
  assert.equal(page.node('playback-toggle').textContent, 'Pause');

  page.pending.shift().resolve({ accepted: true });
  await flush();

  assert.equal(
    page.node('playback-control-status').textContent,
    'Pause accepted. Refresh playback to see the updated snapshot.',
  );
  assert.equal(page.node('playback-control-status').dataset.tone, 'success');
  assert.equal(page.node('playback-playstate').textContent, 'Playing');
  assert.equal(page.node('playback-toggle').textContent, 'Pause');
  assert.equal(page.node('now-playing').dataset.state, 'live');
  assert.equal(page.node('playback-title').textContent, 'Blue in Green');
  assert.equal(page.callsFor('playback').length, 1);
  assert.equal(page.node('refresh').disabled, false);
  for (const id of controlIds) assert.equal(page.node(id).disabled, false);
  assert.equal(page.node('playback-toggle').attributes['aria-busy'], 'false');
  assert.equal(page.document.activeElement, page.node('playback-toggle'));
});

test('previous and next send their own argument-free commands after the card settles', async () => {
  const page = createPage();
  page.pending.shift().resolve({ connected: true, title: 'So What', playing: false });
  await flush();

  page.click('playback-previous');
  page.pending.shift().resolve({ accepted: true });
  await flush();
  page.click('playback-next');
  page.pending.shift().resolve({ accepted: true });
  await flush();

  assert.deepEqual(page.callsFor('playback-previous').map(call => call.args), [{ action: 'playback-previous' }]);
  assert.deepEqual(page.callsFor('playback-next').map(call => call.args), [{ action: 'playback-next' }]);
  assert.equal(page.callsFor('playback').length, 1);
  assert.match(page.node('playback-control-status').textContent, /^Next track accepted\./);
});

test('control requests never overlap while one is in flight', async () => {
  const page = createPage();
  page.pending.shift().resolve({ connected: true, title: 'So What', playing: false });
  await flush();

  page.click('playback-previous');
  page.click('playback-next');
  page.click('playback-toggle');
  page.click('playback-previous');

  assert.equal(page.callsFor('playback-previous').length, 1);
  assert.equal(page.callsFor('playback-next').length, 0);
  assert.equal(page.callsFor('playback-toggle').length, 0);

  page.pending.shift().resolve({ accepted: true });
  await flush();
  page.click('playback-next');
  assert.equal(page.callsFor('playback-next').length, 1);
  page.pending.shift().resolve({ accepted: true });
  await flush();
});

test('a rejected control reports the failure and keeps the last snapshot', async () => {
  const page = createPage();
  page.pending.shift().resolve({ connected: true, title: 'So What', artist: 'Miles Davis', playing: false });
  await flush();

  page.click('playback-next');
  page.pending.shift().reject('host offline');
  await flush();

  assert.equal(
    page.node('playback-control-status').textContent,
    'Next track failed: host offline. The last read is still shown.',
  );
  assert.equal(page.node('playback-control-status').dataset.tone, 'error');
  assert.equal(page.node('now-playing').dataset.state, 'live');
  assert.equal(page.node('playback-title').textContent, 'So What');
  assert.equal(page.node('playback-artist').textContent, 'Miles Davis');
  assert.equal(page.node('playback-playstate').textContent, 'Paused');
  assert.equal(page.node('refresh').disabled, false);
  for (const id of controlIds) assert.equal(page.node(id).disabled, false);
  assert.equal(page.callsFor('playback').length, 1);
});

test('a host response without acceptance is reported as a failed control', async () => {
  const page = createPage();
  page.pending.shift().resolve({ connected: true, title: 'So What', playing: false });
  await flush();

  page.click('playback-previous');
  page.pending.shift().resolve({ accepted: false });
  await flush();

  assert.equal(
    page.node('playback-control-status').textContent,
    'Previous track failed: the host did not accept the command. The last read is still shown.',
  );
  assert.equal(page.node('playback-control-status').dataset.tone, 'error');
});

test('a refresh disables the controls and clears stale control feedback', async () => {
  const page = createPage();
  page.pending.shift().resolve({ connected: true, title: 'So What', playing: false });
  await flush();

  page.click('playback-next');
  page.pending.shift().resolve({ accepted: true });
  await flush();
  assert.match(page.node('playback-control-status').textContent, /accepted\./);

  page.click('refresh');
  for (const id of controlIds) assert.equal(page.node(id).disabled, true);
  assert.equal(page.node('playback-control-status').textContent, '');

  page.pending.shift().resolve({ connected: true, title: 'So What', playing: true });
  await flush();
  for (const id of controlIds) assert.equal(page.node(id).disabled, false);
  assert.equal(page.node('playback-toggle').textContent, 'Pause');
});

test('a finished control restores focus only when it dropped focus to the page', async () => {
  const page = createPage();
  page.pending.shift().resolve({ connected: true, title: 'So What', playing: false });
  await flush();

  const next = page.node('playback-next');
  next.focus();
  page.click('playback-next');
  page.document.activeElement = page.document.body;
  page.pending.shift().resolve({ accepted: true });
  await flush();
  assert.equal(page.document.activeElement, next);

  page.click('playback-previous');
  page.node('ping').focus();
  const focused = next.focusCount;
  page.pending.shift().resolve({ accepted: true });
  await flush();
  assert.equal(page.document.activeElement, page.node('ping'));
  assert.equal(next.focusCount, focused);
  assert.equal(page.node('playback-previous').focusCount, 0);
});

test('arrow keys traverse enabled playback controls in card order', async () => {
  const page = createPage();
  page.pending.shift().resolve({ connected: true, title: 'So What', playing: false });
  await flush();
  assert.equal(page.document.activeElement, page.node('close'));

  page.press('ArrowRight');
  assert.equal(page.document.activeElement, page.node('playback-previous'));
  page.press('ArrowRight');
  assert.equal(page.document.activeElement, page.node('playback-toggle'));
  page.press('ArrowRight');
  assert.equal(page.document.activeElement, page.node('playback-next'));
  page.press('ArrowRight');
  assert.equal(page.document.activeElement, page.node('refresh'));
  page.press('ArrowLeft');
  assert.equal(page.document.activeElement, page.node('playback-next'));

  page.click('playback-next');
  for (const id of controlIds) assert.equal(page.node(id).disabled, true);
  page.press('ArrowLeft');
  assert.equal(page.document.activeElement, page.node('close'));

  page.pending.shift().resolve({ accepted: true });
  await flush();
  assert.equal(page.document.activeElement, page.node('close'));
  for (const id of controlIds) assert.equal(page.node(id).disabled, false);
});



test('refresh cannot overlap an in-flight playback control', async () => {
  const page = createPage();
  page.pending.shift().resolve({ connected: true, title: 'Track', playing: true });
  await flush();
  page.click('playback-next');
  page.click('refresh');
  assert.equal(page.callsFor('playback').length, 1);
  assert.equal(page.pending.length, 1);
  page.pending.shift().resolve({ accepted: true });
  await flush();
  page.click('refresh');
  assert.equal(page.callsFor('playback').length, 2);
  page.pending.shift().resolve({ connected: true, title: 'Next track', playing: true });
  await flush();
  assert.equal(page.node('playback-title').textContent, 'Next track');
});
