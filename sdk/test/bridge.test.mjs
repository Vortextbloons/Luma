import assert from "node:assert/strict";
import test from "node:test";

import {
  BRIDGE_COMMAND,
  BRIDGE_ERROR_CODES,
  COMMAND_EVENT,
  LumaBridgeError,
  OPERATION_PERMISSIONS,
  PERMISSIONS,
  PLAYBACK_EVENT,
  SETTINGS_EVENT,
  bridge,
  createBridge,
  isBridgeError,
  onCommand,
  onPlayback,
  onSettings,
  toBridgeError,
} from "../index.js";

const identity = {
  id: "com.example.weather",
  name: "Weather",
  version: "1.0.0",
  apiVersion: 2,
  runtime: "isolated-ui",
  surface: "desktop",
  origin: "http://luma-extension.s0.localhost/token",
};

function recorder(result) {
  const calls = [];
  const invoke = async (command, args) => {
    calls.push({ command, args });
    return typeof result === "function" ? result(command, args) : result;
  };
  return { calls, bridge: createBridge(invoke) };
}

test("operations use the extension bridge protocol", async () => {
  const { calls, bridge: client } = recorder(identity);

  await client.identity();
  await client.readSettings();
  await client.subscribeSettings();
  await client.get({
    origin: "https://api.open-meteo.com",
    path: "/v1/forecast",
    query: [["latitude", "47.6062"]],
  });
  await client.readPlayback();
  await client.subscribePlayback();
  await client.togglePlayback();
  await client.nextTrack();
  await client.previousTrack();
  await client.ready();
  await client.close();
  await client.pendingCommand();

  assert.deepEqual(
    calls.map(call => call.command),
    Array(12).fill(BRIDGE_COMMAND),
  );
  assert.deepEqual(
    calls.map(call => call.args.op),
    [
      "identity",
      "settings.read",
      "settings.subscribe",
      "net.get",
      "playback.read",
      "playback.subscribe",
      "playback.toggle",
      "playback.next",
      "playback.previous",
      "surface.ready",
      "surface.close",
      "commands.pending",
    ],
  );
  assert.deepEqual(calls[3].args.args, {
    origin: "https://api.open-meteo.com",
    path: "/v1/forecast",
    query: [["latitude", "47.6062"]],
  });
});

test("operations without arguments omit the args field", async () => {
  const { calls, bridge: client } = recorder({ ready: true });
  await client.ready();
  assert.deepEqual(calls, [{ command: BRIDGE_COMMAND, args: { op: "surface.ready" } }]);
});

test("plain JavaScript can import the runtime package", () => {
  assert.equal(typeof bridge.identity, "function");
  assert.equal(typeof bridge.get, "function");
  assert.deepEqual(PERMISSIONS, [
    "settings.read",
    "network.get",
    "playback.read",
    "playback.control",
  ]);
  assert.equal(OPERATION_PERMISSIONS["net.get"], "network.get");
  assert.equal(OPERATION_PERMISSIONS["surface.close"], null);
});

test("the default client uses the host invoke function", async () => {
  const calls = [];
  globalThis.window = {
    __TAURI_INTERNALS__: {
      invoke: async (command, args) => {
        calls.push({ command, args });
        return { ready: true };
      },
    },
  };

  try {
    assert.deepEqual(await bridge.ready(), { ready: true });
    assert.deepEqual(calls, [{ command: BRIDGE_COMMAND, args: { op: "surface.ready" } }]);
  } finally {
    delete globalThis.window;
  }
});

test("command delivery returns the pending command or null", async () => {
  const { bridge: client } = recorder(() => ({ command: "openForecast" }));
  assert.deepEqual(await client.pendingCommand(), { command: "openForecast" });

  const { bridge: empty } = recorder(null);
  assert.equal(await empty.pendingCommand(), null);
});

test("command events use the host event contract", () => {
  const listeners = new Map();
  globalThis.window = {
    addEventListener(name, listener) {
      listeners.set(name, listener);
    },
    removeEventListener(name) {
      listeners.delete(name);
    },
  };

  try {
    const received = [];
    const stop = onCommand(delivery => received.push(delivery.command));
    listeners.get(COMMAND_EVENT)({ detail: { command: "openForecast" } });
    assert.deepEqual(received, ["openForecast"]);
    stop();
    assert.equal(listeners.has("luma:command"), false);
  } finally {
    delete globalThis.window;
  }
});

test("subscription events use the host event contract", () => {
  const listeners = new Map();
  globalThis.window = {
    addEventListener(name, listener) {
      listeners.set(name, listener);
    },
    removeEventListener(name) {
      listeners.delete(name);
    },
  };

  try {
    const settings = [];
    const playback = [];
    const stopSettings = onSettings(snapshot => settings.push(snapshot.values));
    const stopPlayback = onPlayback(state => playback.push(state));
    listeners.get(SETTINGS_EVENT)({ detail: { values: { units: "f" } } });
    listeners.get(PLAYBACK_EVENT)({
      detail: { connected: true, title: "Song", artist: null, playing: true },
    });
    assert.deepEqual(settings, [{ units: "f" }]);
    assert.deepEqual(playback, [
      { connected: true, title: "Song", artist: null, playing: true },
    ]);
    stopSettings();
    stopPlayback();
    assert.equal(listeners.has(SETTINGS_EVENT), false);
    assert.equal(listeners.has(PLAYBACK_EVENT), false);
  } finally {
    delete globalThis.window;
  }
});

test("host failures become typed bridge errors", async () => {
  const { bridge: client } = recorder(() => {
    throw { code: "denied", message: "This package is not allowed to reach that origin." };
  });

  await assert.rejects(client.get({ origin: "https://example.com", path: "/" }), error => {
    assert.ok(error instanceof LumaBridgeError);
    assert.ok(isBridgeError(error));
    assert.equal(error.code, "denied");
    assert.equal(error.message, "This package is not allowed to reach that origin.");
    return true;
  });
});

test("unknown host failures keep the message and use the unknown code", () => {
  assert.equal(toBridgeError("Unexpected command.").code, "unknown");
  assert.equal(toBridgeError("Unexpected command.").message, "Unexpected command.");

  const futureCode = toBridgeError({ code: "future_code", message: "A new host failure." });
  assert.equal(futureCode.code, "unknown");
  assert.equal(futureCode.message, "A new host failure.");
});

test("every bridge error code is exposed to package authors", () => {
  assert.deepEqual(BRIDGE_ERROR_CODES, [
    "invalid",
    "denied",
    "busy",
    "stale",
    "revoked",
    "cancelled",
    "timeout",
    "too_large",
    "malformed",
    "unavailable",
  ]);
});
