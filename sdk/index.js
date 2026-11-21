export const BRIDGE_COMMAND = "extension_bridge";
export const COMMAND_EVENT = "luma:command";
export const SETTINGS_EVENT = "luma:settings";
export const PLAYBACK_EVENT = "luma:playback";

export const PERMISSIONS = Object.freeze([
  "settings.read",
  "network.get",
  "playback.read",
  "playback.control",
]);

export const BRIDGE_OPERATIONS = Object.freeze([
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
  "development.report",
]);

export const OPERATION_PERMISSIONS = Object.freeze({
  identity: null,
  "settings.read": "settings.read",
  "settings.subscribe": "settings.read",
  "net.get": "network.get",
  "playback.read": "playback.read",
  "playback.subscribe": "playback.read",
  "playback.toggle": "playback.control",
  "playback.next": "playback.control",
  "playback.previous": "playback.control",
  "surface.ready": null,
  "surface.close": null,
  "commands.pending": null,
  "development.report": null,
});

export const BRIDGE_ERROR_CODES = Object.freeze([
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

export class LumaBridgeError extends Error {
  constructor(code, message, cause) {
    super(message, cause === undefined ? undefined : { cause });
    this.name = "LumaBridgeError";
    this.code = code;
  }
}

export function isBridgeError(value) {
  return value instanceof LumaBridgeError;
}

export function toBridgeError(cause) {
  if (isBridgeError(cause)) return cause;
  if (cause && typeof cause === "object") {
    const message =
      typeof cause.message === "string" && cause.message.trim()
        ? cause.message
        : "The host rejected the bridge request.";
    const code = BRIDGE_ERROR_CODES.includes(cause.code) ? cause.code : "unknown";
    return new LumaBridgeError(code, message, cause);
  }
  if (typeof cause === "string" && cause.trim()) {
    return new LumaBridgeError("unknown", cause, cause);
  }
  return new LumaBridgeError("unknown", "The host rejected the bridge request.", cause);
}

function hostInvoke(command, args) {
  const internals = globalThis.window?.__TAURI_INTERNALS__;
  if (!internals || typeof internals.invoke !== "function") {
    throw new LumaBridgeError(
      "unavailable",
      "The Luma plugin bridge is unavailable outside a plugin runtime.",
    );
  }
  return internals.invoke(command, args);
}

function detailListener(eventName, listener) {
  const target = globalThis.window;
  if (!target || typeof target.addEventListener !== "function") {
    return () => {};
  }
  const handler = event => {
    const detail = event?.detail;
    if (detail && typeof detail === "object") listener(detail);
  };
  target.addEventListener(eventName, handler);
  return () => {
    if (typeof target.removeEventListener === "function") {
      target.removeEventListener(eventName, handler);
    }
  };
}

export function onCommand(listener) {
  return detailListener(COMMAND_EVENT, detail => {
    if (typeof detail.command === "string") {
      listener(typeof detail.invocationId === "string"
        ? { command: detail.command, invocationId: detail.invocationId }
        : { command: detail.command });
    }
  });
}

export function onSettings(listener) {
  return detailListener(SETTINGS_EVENT, detail => {
    if (detail.values && typeof detail.values === "object") {
      listener({ values: detail.values });
    }
  });
}

export function onPlayback(listener) {
  return detailListener(PLAYBACK_EVENT, detail => {
    if (typeof detail.connected === "boolean" && typeof detail.playing === "boolean") {
      listener({
        connected: detail.connected,
        title: detail.title ?? null,
        artist: detail.artist ?? null,
        playing: detail.playing,
      });
    }
  });
}

export function createBridge(invoke = hostInvoke) {
  const request = async (op, args) => {
    try {
      return await invoke(BRIDGE_COMMAND, args === undefined ? { op } : { op, args });
    } catch (cause) {
      throw toBridgeError(cause);
    }
  };

  return Object.freeze({
    request,
    identity: () => request("identity"),
    readSettings: () => request("settings.read"),
    subscribeSettings: () => request("settings.subscribe"),
    get: args => request("net.get", args),
    readPlayback: () => request("playback.read"),
    subscribePlayback: () => request("playback.subscribe"),
    togglePlayback: () => request("playback.toggle"),
    nextTrack: () => request("playback.next"),
    previousTrack: () => request("playback.previous"),
    ready: () => request("surface.ready"),
    close: () => request("surface.close"),
    pendingCommand: () => request("commands.pending"),
    reportDevelopmentOutcome: args => request("development.report", args),
  });
}

export const bridge = createBridge();
