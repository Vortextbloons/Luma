import test from 'node:test';
import assert from 'node:assert/strict';
import {
  controllerButtonLabel,
  controllerFamily,
  createController,
  defaultControllerBinds,
  sanitizeControllerBinds,
} from './controller.ts';

test('controller binds sanitize invalid values and preserve valid custom binds', () => {
  const defaults = defaultControllerBinds();
  assert.equal(defaults.activate, 0);
  assert.equal(defaults.back, 1);
  assert.equal(defaults.home, 8);
  assert.equal(defaults.tabprev, 4);
  assert.equal(defaults.tabnext, 5);
  assert.equal(defaults.search, 3);
  assert.equal(defaults.settings, 9);
  assert.equal(defaults.playpause, 2);
  assert.equal(defaults.trackprev, 6);
  assert.equal(defaults.tracknext, 7);
  assert.equal(defaults.scores, 11);

  const sanitized = sanitizeControllerBinds({
    activate: 2,
    back: null,
    home: 99,
    up: 'bad',
    tabnext: 0,
    scores: null,
    playpause: 6,
  });
  assert.equal(sanitized.activate, 2);
  assert.equal(sanitized.back, null);
  assert.equal(sanitized.home, 8);
  assert.equal(sanitized.up, 12);
  assert.equal(sanitized.tabnext, 0);
  assert.equal(sanitized.scores, null);
  assert.equal(sanitized.playpause, 6);
  assert.equal(controllerButtonLabel(2), 'X / Square');
  assert.equal(controllerButtonLabel(2, 'playstation'), 'Square');
  assert.equal(controllerButtonLabel(0, 'switch'), 'B');
  assert.equal(controllerButtonLabel(null), 'Unbound');
  assert.equal(controllerButtonLabel(22), 'Button 22');
  assert.equal(controllerFamily('DualSense Wireless Controller'), 'playstation');
  assert.equal(controllerFamily('Nintendo Switch Pro Controller'), 'switch');
  assert.equal(controllerFamily('Xbox Wireless Controller'), 'xbox');
});

test('controller dispatches custom binds, stick repeat, and button capture', () => {
  const frames = new Map();
  let nextFrameId = 1;
  const listeners = new Map();
  let pads = [];
  let focused = true;

  const originals = {
    window: globalThis.window,
    document: globalThis.document,
    navigatorDescriptor: Object.getOwnPropertyDescriptor(globalThis, 'navigator'),
    requestAnimationFrame: globalThis.requestAnimationFrame,
    cancelAnimationFrame: globalThis.cancelAnimationFrame,
  };

  globalThis.window = {
    addEventListener(type, fn) { listeners.set(type, fn); },
    removeEventListener(type) { listeners.delete(type); },
  };
  globalThis.document = { hasFocus: () => focused };
  Object.defineProperty(globalThis, 'navigator', { value: { getGamepads: () => pads }, configurable: true });
  globalThis.requestAnimationFrame = fn => {
    const id = nextFrameId++;
    frames.set(id, fn);
    return id;
  };
  globalThis.cancelAnimationFrame = id => { frames.delete(id); };

  const commands = [];
  const captured = [];
  let captureActive = false;
  let binds = { ...defaultControllerBinds(), activate: 3, search: null };

  const controller = createController({
    enabled: () => true,
    binds: () => binds,
    oncommand: command => commands.push(command),
    oncapture: button => {
      if (!captureActive) return false;
      captured.push(button);
      captureActive = false;
      return true;
    },
  });

  const step = time => {
    const entries = [...frames.entries()];
    frames.clear();
    for (const [, fn] of entries) fn(time);
  };

  try {
    const buttons = Array.from({ length: 17 }, () => ({ pressed: false }));
    buttons[3] = { pressed: true };
    pads = [{ index: 0, id: 'Xbox Controller', connected: true, buttons, axes: [0, 0] }];
    listeners.get('gamepadconnected')?.();
    step(100);
    assert.deepEqual(commands, ['activate']);

    // Release and test left stick repeat
    buttons[3] = { pressed: false };
    pads[0].axes = [-0.8, 0];
    step(120);
    assert.deepEqual(commands, ['activate', 'left']);
    step(300);
    assert.deepEqual(commands, ['activate', 'left']);
    step(500);
    assert.deepEqual(commands, ['activate', 'left', 'left']);

    // Capture mode intercepts the next button press without dispatching
    pads[0].axes = [0, 0];
    captureActive = true;
    buttons[1] = { pressed: true };
    step(550);
    assert.deepEqual(captured, [1]);
    assert.deepEqual(commands, ['activate', 'left', 'left']);

    // Unfocused window ignores input
    focused = false;
    buttons[0] = { pressed: true };
    step(600);
    assert.deepEqual(commands, ['activate', 'left', 'left']);
  } finally {
    controller.destroy();
    globalThis.window = originals.window;
    globalThis.document = originals.document;
    globalThis.requestAnimationFrame = originals.requestAnimationFrame;
    globalThis.cancelAnimationFrame = originals.cancelAnimationFrame;
    if (originals.navigatorDescriptor) Object.defineProperty(globalThis, 'navigator', originals.navigatorDescriptor);
  }
});
