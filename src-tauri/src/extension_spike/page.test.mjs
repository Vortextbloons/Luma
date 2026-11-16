import { readFileSync } from 'node:fs';
import vm from 'node:vm';
import test from 'node:test';
import assert from 'node:assert/strict';

const source = readFileSync(new URL('./page.js', import.meta.url), 'utf8');
const markup = readFileSync(new URL('./page.html', import.meta.url), 'utf8');

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
  'weather',
  'weather-state',
  'weather-location',
  'weather-units',
  'weather-load',
  'weather-reading',
  'weather-place',
  'weather-temperature',
  'weather-condition',
  'weather-observed',
  'weather-status',
  'ping',
  'probe',
  'crash',
  'close',
];

const focusOrder = [
  'playback-previous',
  'playback-toggle',
  'playback-next',
  'refresh',
  'weather-location',
  'weather-units',
  'weather-load',
  'ping',
  'probe',
  'crash',
  'close',
];

const selectOptions = {
  'weather-location': [
    ['seattle', 'Seattle'],
    ['los-angeles', 'Los Angeles'],
    ['chicago', 'Chicago'],
    ['new-york', 'New York'],
    ['toronto', 'Toronto'],
    ['london', 'London'],
    ['paris', 'Paris'],
    ['berlin', 'Berlin'],
    ['tokyo', 'Tokyo'],
    ['sydney', 'Sydney'],
  ],
  'weather-units': [
    ['c', 'Celsius'],
    ['f', 'Fahrenheit'],
  ],
};

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
    if (selectOptions[id]) {
      node.options = selectOptions[id].map(([value, textContent]) => ({ value, textContent }));
      node.selectedIndex = 0;
      Object.defineProperty(node, 'value', {
        get: () => node.options[node.selectedIndex]?.value ?? '',
        set(value) {
          const index = node.options.findIndex(option => option.value === value);
          if (index !== -1) node.selectedIndex = index;
        },
      });
      Object.defineProperty(node, 'selectedOptions', {
        get: () => [node.options[node.selectedIndex]],
      });
    }
    nodes.set(id, node);
  }

  document.querySelector = selector => nodes.get(selector.slice(1)) ?? null;
  document.querySelectorAll = selector => {
    assert.equal(selector, 'select:not(:disabled), button:not(:disabled)');
    return focusOrder.map(id => nodes.get(id)).filter(node => !node.disabled);
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
    select: (id, value) => {
      const node = nodes.get(id);
      node.value = value;
      node.listeners.get('change')();
    },
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

const cities = [
  'seattle',
  'los-angeles',
  'chicago',
  'new-york',
  'toronto',
  'london',
  'paris',
  'berlin',
  'tokyo',
  'sydney',
];

const reading = (temperature = 11.4, units = 'c', weatherCode = 61) => ({
  temperature,
  units,
  weatherCode,
  observedAt: 1700000000,
});

async function settled() {
  const page = createPage();
  page.pending.shift().resolve({ connected: false });
  await flush();
  return page;
}

function weatherSection() {
  const match = markup.match(/<section class="weather"[\s\S]*?<\/section>/);
  assert.ok(match, 'the page markup contains a weather section');
  return match[0];
}

test('the weather location select offers exactly the contract cities', () => {
  const match = weatherSection().match(/<select id="weather-location">([\s\S]*?)<\/select>/);
  assert.ok(match, 'the weather section contains the location select');
  const values = [...match[1].matchAll(/value="([^"]+)"/g)].map(entry => entry[1]);
  assert.deepEqual(values, cities);

  const units = weatherSection().match(/<select id="weather-units">([\s\S]*?)<\/select>/);
  assert.ok(units, 'the weather section contains the units select');
  assert.deepEqual(
    [...units[1].matchAll(/value="([^"]+)"/g)].map(entry => entry[1]),
    ['c', 'f'],
  );
});

test('the weather attribution is plain text with the fixed URL', () => {
  const section = weatherSection();
  assert.match(section, /Weather data by Open-Meteo\.com/);
  assert.match(section, /https:\/\/open-meteo\.com\//);
  assert.doesNotMatch(section, /<a[\s>]/i);
});

test('opening the page never requests weather by itself', async () => {
  const page = await settled();
  assert.deepEqual(page.callsFor('weather-read'), []);
  assert.equal(page.node('weather').dataset.state, 'idle');
  assert.equal(page.node('weather-state').textContent, 'Not loaded');
  assert.equal(page.node('weather-reading').hidden, true);
  assert.equal(page.node('weather-load').textContent, 'Load weather');
  assert.match(page.node('weather-status').textContent, /press Load weather\.$/);
});

test('changing the location or units never requests weather', async () => {
  const page = await settled();
  page.select('weather-location', 'tokyo');
  page.select('weather-units', 'f');
  assert.deepEqual(page.callsFor('weather-read'), []);
  assert.equal(page.node('weather').dataset.state, 'idle');
  assert.equal(page.node('weather-location').value, 'tokyo');
  assert.equal(page.node('weather-units').value, 'f');
});

test('Load sends exactly the selected city and units to the host bridge', async () => {
  const page = await settled();
  page.select('weather-location', 'tokyo');
  page.select('weather-units', 'f');
  page.click('weather-load');

  assert.deepEqual(page.callsFor('weather-read'), [
    {
      command: 'extension_spike_action',
      args: { action: 'weather-read', location: 'tokyo', units: 'f' },
    },
  ]);
  assert.equal(page.node('weather').dataset.state, 'loading');
  assert.equal(page.node('weather-state').textContent, 'Loading');
  assert.equal(page.node('weather-status').textContent, 'Loading weather for Tokyo in Fahrenheit…');
  assert.equal(page.node('weather-load').attributes['aria-disabled'], 'true');
  assert.equal(page.node('weather-load').attributes['aria-busy'], 'true');
  assert.equal(page.node('weather-load').classList.contains('is-loading'), true);
});

test('a reading shows the city, temperature, condition, and observed time it was loaded for', async () => {
  const page = await settled();
  page.click('weather-load');
  page.pending.shift().resolve(reading());
  await flush();

  assert.equal(page.node('weather').dataset.state, 'ready');
  assert.equal(page.node('weather-state').textContent, 'Loaded');
  assert.equal(page.node('weather-reading').hidden, false);
  assert.equal(page.node('weather-place').textContent, 'Seattle');
  assert.equal(page.node('weather-temperature').textContent, '11°C');
  assert.equal(page.node('weather-condition').textContent, 'Rain');
  assert.match(page.node('weather-observed').textContent, /^Observed /);
  assert.equal(
    page.node('weather-status').textContent,
    'Showing the result for Seattle. Press Refresh weather to check again.',
  );
  assert.equal(page.node('weather-load').textContent, 'Refresh weather');
});

test('a failed refresh keeps the last reading and labels it as the last result', async () => {
  const page = await settled();
  page.click('weather-load');
  page.pending.shift().resolve(reading());
  await flush();

  page.click('weather-load');
  assert.equal(page.node('weather-load').textContent, 'Refreshing weather…');
  page.pending.shift().reject({ code: 'unavailable', message: 'Weather is unavailable. Check your connection.' });
  await flush();

  assert.equal(page.node('weather').dataset.state, 'stale');
  assert.equal(page.node('weather-state').textContent, 'Last result');
  assert.equal(page.node('weather-reading').hidden, false);
  assert.equal(page.node('weather-place').textContent, 'Seattle');
  assert.equal(page.node('weather-temperature').textContent, '11°C');
  assert.match(page.node('weather-status').textContent, /^Could not load weather for Seattle in Celsius: Weather is unavailable\. Check your connection\. Showing the last result for Seattle\.$/);
});

test('an initial failure reports the sanitized message and recovers through Refresh', async () => {
  const page = await settled();
  page.select('weather-location', 'berlin');
  page.select('weather-units', 'f');
  page.click('weather-load');
  page.pending.shift().reject({ code: 'timeout', message: 'Weather took too long to load.' });
  await flush();

  assert.equal(page.node('weather').dataset.state, 'error');
  assert.equal(page.node('weather-state').textContent, 'Failed');
  assert.equal(page.node('weather-reading').hidden, true);
  assert.equal(
    page.node('weather-status').textContent,
    'Could not load weather for Berlin in Fahrenheit: Weather took too long to load.',
  );
  assert.doesNotMatch(page.node('weather-status').textContent, /object Object/);
  assert.equal(page.node('weather-location').value, 'berlin');
  assert.equal(page.node('weather-units').value, 'f');
  assert.equal(page.node('weather-load').textContent, 'Load weather');

  page.click('weather-load');
  assert.deepEqual(page.callsFor('weather-read')[1].args, {
    action: 'weather-read',
    location: 'berlin',
    units: 'f',
  });
  page.pending.shift().resolve(reading(4.6, 'f', 3));
  await flush();

  assert.equal(page.node('weather').dataset.state, 'ready');
  assert.equal(page.node('weather-place').textContent, 'Berlin');
  assert.equal(page.node('weather-temperature').textContent, '5°F');
  assert.equal(page.node('weather-condition').textContent, 'Overcast');
  assert.doesNotMatch(page.node('weather-status').textContent, /too long/);
});

test('weather keeps one request in flight and ignores repeated activation', async () => {
  const page = await settled();
  page.click('weather-load');
  page.click('weather-load');
  page.click('weather-load');
  assert.equal(page.callsFor('weather-read').length, 1);

  page.select('weather-location', 'tokyo');
  page.click('weather-load');
  assert.equal(page.callsFor('weather-read').length, 1);

  page.pending.shift().resolve(reading());
  await flush();
  page.click('weather-load');
  assert.equal(page.callsFor('weather-read').length, 2);
});

test('a slow result keeps the city and units it was requested for', async () => {
  const page = await settled();
  page.click('weather-load');
  page.select('weather-location', 'tokyo');
  page.select('weather-units', 'f');

  assert.equal(page.node('weather-status').textContent, 'Loading weather for Seattle in Celsius…');

  page.pending.shift().resolve(reading(16, 'c', 3));
  await flush();

  assert.equal(page.node('weather').dataset.state, 'stale');
  assert.equal(page.node('weather-state').textContent, 'Last result');
  assert.equal(page.node('weather-place').textContent, 'Seattle');
  assert.equal(page.node('weather-temperature').textContent, '16°C');
  assert.equal(page.node('weather-condition').textContent, 'Overcast');
  assert.equal(
    page.node('weather-status').textContent,
    'Showing the last result for Seattle in Celsius. Press Refresh weather for Tokyo in Fahrenheit.',
  );
});

test('changing the selection after a load marks the reading as the last result', async () => {
  const page = await settled();
  page.click('weather-load');
  page.pending.shift().resolve(reading(11.4, 'c'));
  await flush();
  assert.equal(page.node('weather').dataset.state, 'ready');

  page.select('weather-units', 'f');

  assert.equal(page.node('weather').dataset.state, 'stale');
  assert.equal(page.node('weather-state').textContent, 'Last result');
  assert.equal(page.node('weather-place').textContent, 'Seattle');
  assert.equal(page.node('weather-temperature').textContent, '11°C');
  assert.match(page.node('weather-status').textContent, /^Showing the last result for Seattle in Celsius\./);
  assert.equal(page.callsFor('weather-read').length, 1);
});

test('an unreadable reading is reported instead of a fabricated temperature', async () => {
  const page = await settled();
  page.click('weather-load');
  page.pending.shift().resolve({ temperature: 'warm', units: 'c', weatherCode: 3, observedAt: 1700000000 });
  await flush();

  assert.equal(page.node('weather').dataset.state, 'error');
  assert.equal(page.node('weather-reading').hidden, true);
  assert.equal(page.node('weather-temperature').textContent, '');
  assert.match(page.node('weather-status').textContent, /^Could not load weather for Seattle in Celsius: The host returned an unreadable reading\.$/);
});

test('arrow keys reach the weather controls and change a focused select without a request', async () => {
  const page = await settled();
  assert.equal(page.document.activeElement, page.node('close'));

  page.press('ArrowRight');
  assert.equal(page.document.activeElement, page.node('refresh'));
  page.press('ArrowRight');
  assert.equal(page.document.activeElement, page.node('weather-location'));
  page.press('ArrowRight');
  assert.equal(page.document.activeElement, page.node('weather-units'));
  page.press('ArrowRight');
  assert.equal(page.document.activeElement, page.node('weather-load'));
  page.press('ArrowLeft');
  assert.equal(page.document.activeElement, page.node('weather-units'));

  page.node('weather-location').focus();
  const location = page.node('weather-location');
  const up = page.press('ArrowUp');
  assert.equal(up.defaultPrevented, true);
  assert.equal(location.value, 'seattle');
  assert.equal(page.document.activeElement, location);

  page.press('ArrowDown');
  assert.equal(location.value, 'los-angeles');
  assert.equal(page.document.activeElement, location);
  page.press('ArrowUp');
  assert.equal(location.value, 'seattle');

  page.select('weather-location', 'sydney');
  page.press('ArrowDown');
  assert.equal(location.value, 'sydney');

  page.node('weather-units').focus();
  const units = page.node('weather-units');
  page.press('ArrowDown');
  assert.equal(units.value, 'f');
  page.press('ArrowDown');
  assert.equal(units.value, 'f');
  page.press('ArrowUp');
  assert.equal(units.value, 'c');

  assert.deepEqual(page.callsFor('weather-read'), []);
});

test('Load keeps focus through loading and restores it only when the control dropped it', async () => {
  const page = await settled();
  const load = page.node('weather-load');
  load.focus();
  page.click('weather-load');
  assert.equal(page.document.activeElement, load);

  page.pending.shift().resolve(reading());
  await flush();
  assert.equal(page.document.activeElement, load);

  page.click('weather-load');
  page.document.activeElement = page.document.body;
  page.pending.shift().resolve(reading(12, 'c'));
  await flush();
  assert.equal(page.document.activeElement, load);

  page.click('weather-load');
  page.node('close').focus();
  const focused = load.focusCount;
  page.pending.shift().reject({ code: 'unavailable', message: 'Weather is unavailable.' });
  await flush();
  assert.equal(page.document.activeElement, page.node('close'));
  assert.equal(load.focusCount, focused);
});
