import { readFileSync } from 'node:fs';
import vm from 'node:vm';
import test from 'node:test';
import assert from 'node:assert/strict';

const source = readFileSync(new URL('./phone.html', import.meta.url), 'utf8');
const transport = source.slice(source.indexOf('  let socket;'), source.indexOf('  async function send('));
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
  const context = vm.createContext({ WebSocket, paired: true, document: { visibilityState: 'visible' }, location: { protocol: 'http:', host: 'localhost:9877' }, setTimeout, clearTimeout, applyState() {}, setLink() {}, stopStick() {} });
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
  const context = vm.createContext({ view, pcFullscreenButton });
  const applyState = source.slice(source.indexOf('  function applyState('), source.indexOf('  let appsOpen = true;'));
  vm.runInContext(applyState, context);
  vm.runInContext('applyState({ view: "Home", window: "YouTube", windowFullscreen: true })', context);
  assert.equal(pcFullscreenButton.textContent, 'Exit PC fullscreen');
  assert.equal(pcFullscreenButton.attrs['aria-pressed'], 'true');
  vm.runInContext('applyState({ view: "Home", window: "YouTube", windowFullscreen: false })', context);
  assert.equal(pcFullscreenButton.textContent, 'Fullscreen PC');
  vm.runInContext('applyState({ view: "Home", window: "Netflix" })', context);
  assert.equal(pcFullscreenButton.textContent, 'Toggle PC fullscreen');
  assert.equal(pcFullscreenButton.attrs['aria-pressed'], undefined);
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
  const button = { dataset: { command: 'right' }, disabled: false };
  const context = vm.createContext({
    document: { addEventListener: (type, handler) => handlers[type] = handler },
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
