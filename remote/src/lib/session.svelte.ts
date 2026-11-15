import { api } from "./api";
import { applyAppearance, readThemeChoice, saveThemeChoice, validAppearance } from "./appearance";
import { loadApps, syncAppOrder } from "./apps.svelte";
import { resetScreen } from "./screen.svelte";
import { createCommandSocket, type CommandSocket } from "./socket";
import { createSoundFeedback } from "./sound";
import { clearToast, say } from "./toast.svelte";
import type { Appearance, RemoteState, SoundPreset, StateResponse } from "./types";

const PAIRED_KEY = "luma-paired";
const PRESETS: SoundPreset[] = ["real", "google", "roku", "off"];

const sound = createSoundFeedback();
let knownPaired = readFlag(PAIRED_KEY);
let socket: CommandSocket | null = null;
let refreshBusy = false;
let reconnectTimer: ReturnType<typeof setTimeout> | null = null;
let reconnectDelay = 1000;
let heartbeat: ReturnType<typeof setInterval> | null = null;
let bound = false;
let bootCode = "";

export const session = $state({
  booted: false,
  bootError: false,
  paired: false,
  connecting: true,
  online: false,
  phoneOffline: typeof navigator === "undefined" ? false : !navigator.onLine,
  pcName: "Luma PC",
  state: null as RemoteState | null,
  pairNotice: "",
  pairingBusy: false,
  themeChoice: readThemeChoice(),
});

function readFlag(key: string): boolean {
  try {
    return localStorage.getItem(key) === "yes";
  } catch (_) {
    return false;
  }
}

function writeFlag(key: string, value: boolean): void {
  try {
    localStorage.setItem(key, value ? "yes" : "no");
  } catch (_) {}
}

function takePairingCode(): string {
  const params = new URLSearchParams(location.hash.replace(/^#/, ""));
  const code = params.get("code") ?? "";
  if (location.hash) {
    try {
      history.replaceState(history.state, "", location.pathname + location.search);
    } catch (_) {}
  }
  return /^\d{6}$/.test(code) ? code : "";
}

export function boot(): void {
  if (bound) return;
  bound = true;
  bootCode = takePairingCode();
  window.addEventListener("online", onOnline);
  window.addEventListener("offline", onOffline);
  document.addEventListener("visibilitychange", onVisibility);
  window.addEventListener("pagehide", onPageHide);
  heartbeat = setInterval(tick, 15000);
  void refresh();
}

export function destroy(): void {
  if (!bound) return;
  bound = false;
  if (heartbeat) clearInterval(heartbeat);
  heartbeat = null;
  resetReconnect();
  const previous = socket;
  socket = null;
  previous?.close();
  window.removeEventListener("online", onOnline);
  window.removeEventListener("offline", onOffline);
  document.removeEventListener("visibilitychange", onVisibility);
  window.removeEventListener("pagehide", onPageHide);
  clearToast();
}

export async function refresh(): Promise<void> {
  if (refreshBusy) return;
  refreshBusy = true;
  try {
    const { response, data } = await api.state();
    if (response.status === 401) {
      if (bootCode) {
        const code = bootCode;
        bootCode = "";
        refreshBusy = false;
        if (await pair(code)) await refresh();
        else showPairing(session.pairNotice || "This remote is no longer paired. Choose Connect in Luma for a new code.");
        return;
      }
      showPairing(knownPaired ? "This remote is no longer paired. Choose Connect in Luma for a new code." : "");
      return;
    }
    if (!response.ok) {
      if (session.paired) {
        setOnline(false);
        scheduleReconnect();
      } else {
        session.bootError = true;
        session.connecting = false;
      }
      return;
    }
    showRemote(data ?? {});
  } catch (_) {
    if (session.paired) {
      setOnline(false);
      scheduleReconnect();
    } else {
      session.bootError = true;
      session.connecting = false;
    }
  } finally {
    refreshBusy = false;
    session.booted = true;
  }
}

function showRemote(data: StateResponse): void {
  const first = !session.paired;
  session.paired = true;
  session.bootError = false;
  session.connecting = false;
  session.online = true;
  session.pairNotice = "";
  if (!knownPaired) {
    knownPaired = true;
    writeFlag(PAIRED_KEY, true);
  }
  session.pcName = typeof data.pcName === "string" && data.pcName.trim() ? data.pcName.trim() : "Luma PC";
  if (data.state) applyState(data.state);
  connectSocket();
  void loadApps();
  if (first) resetReconnect();
}

export function applyState(state: RemoteState): void {
  const previous = session.state;
  session.state = {
    ...state,
    appearance: state.appearance ?? previous?.appearance,
    settings: Array.isArray(state.settings) ? state.settings : previous?.settings ?? [],
    appOrder: Array.isArray(state.appOrder) ? state.appOrder : previous?.appOrder ?? [],
  };
  const preset = PRESETS.includes(state.sound) ? state.sound : "real";
  sound.setPreset(preset);
  if (state.appearance) applySessionAppearance(state.appearance);
  if (Array.isArray(state.appOrder)) syncAppOrder(state.appOrder);
}

function applySessionAppearance(appearance: Appearance): void {
  session.themeChoice = applyAppearance(appearance, session.themeChoice);
}

function applyAppearanceMessage(appearance: Appearance): void {
  if (!validAppearance(appearance)) return;
  applySessionAppearance(appearance);
  if (session.state) session.state = { ...session.state, appearance };
}

export function setTheme(choice: string): void {
  const appearance = session.state?.appearance;
  const known = choice === "follow" || Boolean(appearance?.themes.some(theme => theme.id === choice));
  session.themeChoice = known ? choice : "follow";
  saveThemeChoice(session.themeChoice);
  if (appearance) applySessionAppearance(appearance);
}

export async function pair(code: string): Promise<boolean> {
  if (session.pairingBusy) return false;
  const clean = code.replace(/\D/g, "").slice(0, 6);
  if (clean.length !== 6) {
    session.pairNotice = "Enter the six-digit code from Luma.";
    return false;
  }
  session.pairingBusy = true;
  session.pairNotice = "";
  try {
    const { response } = await api.pair(clean);
    if (response.ok) return true;
    session.pairNotice = response.status === 401 || response.status === 403
      ? "That code was not accepted. Choose Connect in Luma for a new web remote code."
      : "Luma could not pair this device. Try again.";
    return false;
  } catch (_) {
    session.pairNotice = "Luma is not reachable. Check Wi-Fi and that Luma is open.";
    return false;
  } finally {
    session.pairingBusy = false;
  }
}

export async function disconnect(): Promise<string | null> {
  try {
    const { response } = await api.disconnect();
    if (!response.ok) return "Could not disconnect. Check your connection and try again.";
    session.paired = false;
    location.reload();
    return null;
  } catch (_) {
    return "Could not disconnect. Check your connection and try again.";
  }
}

export interface SendResult {
  accepted: boolean;
  error?: string | null;
  state?: RemoteState;
}

export async function send(command: string): Promise<SendResult> {
  sound.play(command);
  try {
    if (socket?.isOpen()) {
      const reply = await socket.request(command);
      if (reply.appearance) applyAppearanceMessage(reply.appearance);
      if (reply.appOrder) syncAppOrder(reply.appOrder);
      if (reply.state) applyState(reply.state);
      if (reply.error) {
        say(reply.error, true);
        return { accepted: false, error: reply.error };
      }
      setOnline(true);
      return { accepted: true, state: reply.state };
    }

    const { response, data } = await api.command(command);
    if (response.status === 401) {
      showPairing("This remote is no longer paired. Choose Connect in Luma for a new code.");
      return { accepted: false, error: null };
    }
    if (!response.ok) {
      const offline = response.status >= 500;
      const message = offline
        ? "Luma is not reachable. Check Wi-Fi and that Luma is open."
        : "Luma did not accept that button.";
      if (offline) {
        setOnline(false);
        scheduleReconnect();
      }
      say(message, true);
      return { accepted: false, error: message };
    }
    if (data?.state) applyState(data.state);
    if (data?.error) {
      say(data.error, true);
      return { accepted: false, error: data.error };
    }
    setOnline(true);
    return { accepted: true, state: data?.state };
  } catch (_) {
    setOnline(false);
    scheduleReconnect();
    const message = "Luma is not reachable. Check Wi-Fi and that Luma is open.";
    say(message, true);
    return { accepted: false, error: message };
  }
}

export function showPairing(message = ""): void {
  session.paired = false;
  session.bootError = false;
  session.connecting = false;
  session.online = false;
  session.state = null;
  session.pairNotice = message;
  resetReconnect();
  const previous = socket;
  socket = null;
  previous?.close();
  resetScreen();
  clearToast();
}

export function retry(): void {
  session.bootError = false;
  session.connecting = true;
  resetReconnect();
  const previous = socket;
  socket = null;
  previous?.close();
  void refresh();
  connectSocket();
}

function connectSocket(): void {
  if (!session.paired || document.visibilityState !== "visible" || socket) return;
  let instance: CommandSocket;
  instance = createCommandSocket({
    create: () => new WebSocket(`${location.protocol === "https:" ? "wss:" : "ws:"}//${location.host}/phone/ws`),
    onOpen: () => {
      resetReconnect();
      setOnline(true);
    },
    onMessage: reply => {
      if (reply.appearance) applyAppearanceMessage(reply.appearance);
      if (reply.appOrder) syncAppOrder(reply.appOrder);
      if (reply.state) applyState(reply.state);
      setOnline(true);
    },
    onClose: () => {
      if (socket === instance) socket = null;
      if (document.visibilityState === "visible") setOnline(false);
      scheduleReconnect();
    },
  });
  socket = instance;
  instance.connect();
}

function setOnline(online: boolean): void {
  session.connecting = false;
  session.online = online;
  if (online) resetReconnect();
}

function resetReconnect(): void {
  reconnectDelay = 1000;
  if (reconnectTimer) {
    clearTimeout(reconnectTimer);
    reconnectTimer = null;
  }
}

function scheduleReconnect(): void {
  if (!session.paired || document.visibilityState !== "visible" || reconnectTimer) return;
  reconnectTimer = setTimeout(() => {
    reconnectTimer = null;
    if (!session.paired || document.visibilityState !== "visible") return;
    void refresh();
    connectSocket();
    reconnectDelay = Math.min(reconnectDelay * 2, 15000);
  }, reconnectDelay);
}

function tick(): void {
  if (!session.paired || document.visibilityState !== "visible") return;
  if (socket?.isOpen()) {
    if (!socket.hasPending()) void socket.request().catch(() => {});
  } else if (!reconnectTimer) {
    connectSocket();
    void refresh();
  }
}

function onOnline(): void {
  session.phoneOffline = false;
  if (!session.paired) return;
  resetReconnect();
  void refresh();
  connectSocket();
}

function onOffline(): void {
  session.phoneOffline = true;
  if (session.paired) setOnline(false);
}

function onVisibility(): void {
  if (document.visibilityState === "visible" && session.paired) {
    resetReconnect();
    connectSocket();
    void refresh();
  } else if (document.visibilityState !== "visible") {
    resetReconnect();
    socket?.close();
  }
}

function onPageHide(): void {
  socket?.close();
}
