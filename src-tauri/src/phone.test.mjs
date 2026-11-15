import { readFileSync } from 'node:fs';
import vm from 'node:vm';
import test from 'node:test';
import assert from 'node:assert/strict';

const source = readFileSync(new URL('./phone.html', import.meta.url), 'utf8');
const transport = source.slice(source.indexOf('  let socket;'), source.indexOf('  async function send('));

test('the remote page ships installable web app metadata', () => {
  assert.match(source, /<title>Luma Remote<\/title>/);
  assert.match(source, /<link rel="manifest" href="\/manifest\.webmanifest">/);
  assert.match(source, /<link rel="apple-touch-icon" sizes="180x180" href="\/icons\/apple-touch-icon\.png">/);
  assert.match(source, /<meta name="apple-mobile-web-app-capable" content="yes">/);
  assert.match(source, /<meta name="theme-color" content="#0b0d12">/);
});

const pairingHashSource = source.slice(source.indexOf('  function pairingCodeFromHash('), source.indexOf('  function paintTokens('));
test('pairing codes are read only from a six-digit URL fragment', () => {
  const context = vm.createContext({ URLSearchParams });
  vm.runInContext(pairingHashSource, context);
  assert.equal(vm.runInContext('pairingCodeFromHash("#code=123456")', context), '123456');
  assert.equal(vm.runInContext('pairingCodeFromHash("#code=12345")', context), '');
  assert.equal(vm.runInContext('pairingCodeFromHash("#code=1234567")', context), '');
  assert.equal(vm.runInContext('pairingCodeFromHash("#other=123456")', context), '');
  assert.equal(vm.runInContext('pairingCodeFromHash("")', context), '');
});

function setup() {
  const connections = [];
  class WebSocket {
    static OPEN = 1;
    readyState = 1;
    sent = [];
    constructor() { connections.push(this); }
    send(value) { this.sent.push(JSON.parse(value)); }
    close() { this.readyState = 3; this.onclose(); }
  }
  const context = vm.createContext({ WebSocket, paired: true, document: { visibilityState: 'visible' }, location: { protocol: 'http:', host: 'localhost:9877' }, setTimeout, clearTimeout, applyState() {}, setLink() {}, stopStick() {}, scheduleReconnect() {}, reconnectDelay: 0 });
  vm.runInContext(transport, context);
  vm.runInContext('connectSocket()', context);
  return { context, connections, request: command => vm.runInContext(`socketRequest(${JSON.stringify(command)})`, context) };
}

test('web remote correlates acknowledgments and sends each command once', async () => {
  const { connections, request } = setup();
  const first = request('right'), second = request('activate');
  const connection = connections[0];
  assert.deepEqual(connection.sent.map(value => value.command), ['right', 'activate']);
  connection.onmessage({ data: JSON.stringify({ id: 2, state: {} }) });
  connection.onmessage({ data: JSON.stringify({ id: 1, state: {} }) });
  assert.equal((await first).id, 1);
  assert.equal((await second).id, 2);
});

test('disconnect rejects pending input without replaying it after reconnect', async () => {
  const { context, connections, request } = setup();
  const result = request('activate');
  connections[0].close();
  await assert.rejects(result, /Connection interrupted/);
  vm.runInContext('connectSocket()', context);
  assert.equal(connections.length, 2);
  assert.equal(connections[1].sent.length, 0);
  connections[1].close();
});

const pointerSource = source.slice(source.indexOf('  function releaseStick()'), source.indexOf('  function startStick('));
function pointerSetup() {
  const calls = [];
  const answers = [];
  const context = vm.createContext({
    request: (path, options) => {
      calls.push(JSON.parse(options.body));
      return new Promise(resolve => answers.push(resolve));
    },
    say() {},
    showPairing() {},
    stick: { active: true, released: false, needStart: true, x: 0, y: 0, heartbeat: 0 },
    pointerOwner: 'owner', pointerPending: null, pointerInFlight: false,
    pad: { classList: { remove() {} }, style: { removeProperty() {} } },
    clearInterval() {}, setInterval() { return 1; }
  });
  vm.runInContext(pointerSource, context);
  return { context, calls, answers };
}

test('joystick updates coalesce and a physical takeover stops the stick', async () => {
  const { context, calls, answers } = pointerSetup();
  vm.runInContext('queuePointer("start", 0, 0); queuePointer("move", 0.8, 0);', context);
  assert.equal(calls.length, 1);
  assert.equal(calls[0].action, 'start');
  answers.shift()({ response: { status: 200, ok: true }, data: { released: false } });
  await new Promise(resolve => setImmediate(resolve));
  assert.equal(calls.length, 2);
  assert.equal(calls[1].action, 'move');
  assert.equal(calls[1].x, 0.8);
  answers.shift()({ response: { status: 200, ok: true }, data: { released: true } });
  await new Promise(resolve => setImmediate(resolve));
  assert.equal(vm.runInContext('stick.released', context), true);
  assert.equal(vm.runInContext('pointerPending', context), null);
  assert.equal(calls.length, 2);
  vm.runInContext('queuePointer("move", 0.4, 0)', context);
  assert.equal(calls.length, 2);
});


test('pc fullscreen label only promises exit when the PC reports its window state', () => {
  const view = { textContent: '', hidden: false };
  const pcFullscreenButton = { textContent: '', attrs: {}, setAttribute(name, value) { this.attrs[name] = value; }, removeAttribute(name) { delete this.attrs[name]; } };
  const context = vm.createContext({ view, pcFullscreenButton, activeApp: '', applyTextMode() {}, soundState: { preset: 'real' }, loadSounds() {} });
  const applyState = source.match(/  function applyState\([\s\S]*?\r?\n  }/)[0];
  vm.runInContext(applyState, context);
  vm.runInContext('applyState({ view: "Home", window: "YouTube", windowFullscreen: true })', context);
  assert.equal(pcFullscreenButton.textContent, 'Exit');
  assert.equal(pcFullscreenButton.attrs['aria-pressed'], 'true');
  vm.runInContext('applyState({ view: "Home", window: "YouTube", windowFullscreen: false })', context);
  assert.equal(pcFullscreenButton.textContent, 'Fullscreen');
  vm.runInContext('applyState({ view: "Home", window: "Netflix" })', context);
  assert.equal(pcFullscreenButton.textContent, 'Toggle');
  assert.equal(pcFullscreenButton.attrs['aria-pressed'], undefined);
});

test('remote follows the TV sound preset', () => {
  const view = { textContent: '', hidden: false };
  const pcFullscreenButton = { textContent: '', setAttribute() {}, removeAttribute() {} };
  const loaded = [];
  const context = vm.createContext({ view, pcFullscreenButton, activeApp: '', applyTextMode() {}, soundState: { preset: 'real' }, loadSounds: preset => loaded.push(preset) });
  const applyState = source.match(/  function applyState\([\s\S]*?\r?\n  }/)[0];
  vm.runInContext(applyState, context);
  vm.runInContext('applyState({ view: "Home", sound: "google" })', context);
  assert.equal(vm.runInContext('soundState.preset', context), 'google');
  assert.deepEqual(loaded, ['google']);
  vm.runInContext('applyState({ view: "Home", sound: "off" })', context);
  assert.equal(vm.runInContext('soundState.preset', context), 'off');
  assert.deepEqual(loaded, ['google', 'off']);
});

test('joystick starts at the center, follows dragging, and only taps click', () => {
  const sent = [];
  const moves = [];
  const context = vm.createContext({
    joystickMode: true, stick: { id: null }, performance: { now: () => 100 },
    pad: { clientWidth: 268, setPointerCapture() {}, classList: { add() {} }, style: { setProperty() {} } },
    queuePointer: (...args) => moves.push(args), send: command => sent.push(command),
    stopStick() {}, clearInterval() {}, setInterval() { return 1; }
  });
  const gestures = source.slice(source.indexOf('  function startStick('), source.indexOf('  function saveJoystick('));
  vm.runInContext(gestures, context);
  const event = { pointerId: 1, pointerType: 'touch', clientX: 100, clientY: 100, type: 'pointerup', preventDefault() {} };
  context.event = event;
  vm.runInContext('startStick(event); endStick(event)', context);
  assert.deepEqual(sent, ['click']);
  vm.runInContext('stick.id = null; startStick(event)', context);
  event.clientX = 150;
  vm.runInContext('moveStick(event); endStick(event)', context);
  assert.equal(sent.length, 1);
  assert.ok(moves.at(-1)[1] > 0.8);
});


test('navigation sends on press without a duplicate on release and preserves keyboard clicks', () => {
  const handlers = {};
  const sent = [];
  const button = { dataset: { command: 'right' }, disabled: false, closest: () => null };
  const context = vm.createContext({
    document: { addEventListener: (type, handler) => {
      const previous = handlers[type];
      handlers[type] = event => { previous?.(event); handler(event); };
    } },
    moreMenu: { hidden: true },
    joystickMode: false, pad: { contains: () => true }, send: command => sent.push(command)
  });
  vm.runInContext(source.slice(source.indexOf('  const immediateCommands'), source.indexOf("  pairForm.addEventListener('submit'")), context);
  const event = { target: { closest: () => button }, isPrimary: true, button: 0, detail: 1, preventDefault() {} };
  handlers.pointerdown(event);
  assert.deepEqual(sent, ['right']);
  handlers.click(event);
  assert.deepEqual(sent, ['right']);
  handlers.click({ ...event, detail: 0 });
  assert.deepEqual(sent, ['right', 'right']);
});

const settingsSource = source.slice(source.indexOf("  let settingsSignature = ''"), source.indexOf('  function applyState(state)'));
test('remote renders app settings from the shared catalog and restores a rejected change', async () => {
  class Element {
    children = [];
    listeners = {};
    dataset = {};
    append(...items) { this.children.push(...items); }
    replaceChildren(fragment) { this.children = fragment.children; }
    contains() { return false; }
    addEventListener(name, handler) { this.listeners[name] = handler; }
    querySelectorAll(tag) {
      return this.children.flatMap(child => [...(child.tag === tag ? [child] : []), ...child.querySelectorAll(tag)]);
    }
  }
  const slots = { theme: new Element(), display: new Element(), sound: new Element() };
  const note = new Element();
  const requests = [];
  const context = vm.createContext({
    document: {
      activeElement: null,
      getElementById: id => {
        if (id === 'app-settings-note') return note;
        if (id.startsWith('app-settings-')) return slots[id.slice('app-settings-'.length)] ?? null;
        return null;
      },
      createDocumentFragment: () => new Element(),
      createElement: tag => Object.assign(new Element(), { tag }),
    },
    request: async (path, options) => { requests.push([path, JSON.parse(options.body)]); return { response: { ok: false } }; },
  });
  vm.runInContext(settingsSource, context);
  vm.runInContext(`syncSettings([{ key: 'density', group: 'Display', label: 'Layout density', description: 'Choose spacing', value: 'comfortable', options: [{ value: 'comfortable', label: 'Comfortable' }, { value: 'compact', label: 'Compact' }] }])`, context);
  assert.equal(slots.theme.querySelectorAll('select').length, 0);
  assert.equal(slots.sound.querySelectorAll('select').length, 0);
  const [select] = slots.display.querySelectorAll('select');
  assert.equal(select.value, 'comfortable');
  assert.equal(select.children.length, 2);
  select.value = 'compact';
  await select.listeners.change();
  assert.deepEqual(requests, [['/phone/settings', { key: 'density', value: 'compact' }]]);
  assert.equal(select.value, 'comfortable');
  assert.equal(select.disabled, false);
  assert.match(note.textContent, /Could not save/);
});

const movieSource = source.slice(source.indexOf("  const moviePosition = document.getElementById('movie-position');"), source.indexOf('  setInterval(() => { if (!movieDragging)'));
function movieSetup() {
  const events = new Map();
  const slider = { value: '0', max: '1', disabled: true, setAttribute() {}, addEventListener: (name, handler) => events.set(name, handler) };
  const time = { textContent: '' };
  const calls = [], answers = [], notices = [];
  const context = vm.createContext({ paired: true, document: { visibilityState: 'visible', getElementById: id => id === 'movie-position' ? slider : time }, request: (path, options) => {
    calls.push(options?.body ? JSON.parse(options.body).position : null);
    return new Promise(resolve => answers.push(resolve));
  }, say: message => notices.push(message) });
  vm.runInContext(movieSource, context);
  const answer = data => answers.shift()({ response: { ok: true }, data });
  return { slider, time, calls, notices, events, answer, refresh: () => vm.runInContext('refreshMovie()', context) };
}
test('movie slider preserves a drag and sends its seek after the pending refresh', async () => {
  const movie = movieSetup();
  const refresh = movie.refresh();
  movie.slider.max = '7200';
  movie.slider.value = '3600';
  movie.events.get('input')();
  assert.equal(movie.calls.length, 1);
  assert.match(movie.time.textContent, /60:00/);
  movie.events.get('change')();
  movie.answer({ timeline: { position: 10, duration: 7200 } });
  await refresh;
  assert.equal(movie.slider.value, '3600');
  assert.deepEqual(movie.calls, [null, 3600]);
  movie.answer({ timeline: { position: 3600, duration: 7200 } });
  await new Promise(resolve => setImmediate(resolve));
  assert.equal(movie.slider.value, '3600');
  assert.equal(movie.slider.disabled, false);
});
test('movie slider reports a player seek rejection and recovers on a fresh timeline', async () => {
  const movie = movieSetup();
  movie.slider.value = '90';
  movie.events.get('change')();
  movie.answer({ timeline: { error: 'The player declined the seek.' } });
  await new Promise(resolve => setImmediate(resolve));
  assert.equal(movie.slider.disabled, true);
  assert.deepEqual(movie.notices, ['The player declined the seek.']);
  const refresh = movie.refresh();
  movie.answer({ timeline: { position: 20, duration: 120 } });
  await refresh;
  assert.equal(movie.slider.disabled, false);
  assert.equal(movie.slider.value, '20');
});
