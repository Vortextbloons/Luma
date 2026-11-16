const invoke = (command, args) => window.__TAURI_INTERNALS__.invoke(command, args);
const host = (action) => invoke("extension_spike_action", { action });

const identity = document.querySelector("#identity");
const pingResult = document.querySelector("#ping-result");
const denial = document.querySelector("#denial");
const note = document.querySelector("#note");
const nowPlaying = document.querySelector("#now-playing");
const playbackConnection = document.querySelector("#playback-connection");
const playbackTitle = document.querySelector("#playback-title");
const playbackArtist = document.querySelector("#playback-artist");
const playbackStateTag = document.querySelector("#playback-playstate");
const playbackStatus = document.querySelector("#playback-status");
const refreshButton = document.querySelector("#refresh");

const show = (element, text) => {
  element.textContent = text;
};

let playback = null;
let playbackAt = null;
let playbackError = null;
let refreshing = false;

const time = (value) => value.toLocaleTimeString();

function describeConnection() {
  if (!playback) return "Not checked yet";
  return playback.connected ? "Spotify account linked" : "Spotify not connected";
}

function describeTitle() {
  if (!playback) {
    if (refreshing) return "Reading playback…";
    return playbackError ? "Playback unavailable" : "Not read yet";
  }
  return playback.title || "Nothing playing";
}

function describeStatus() {
  const stamp = playbackAt ? `Read ${time(playbackAt)}.` : "";
  if (refreshing) {
    return playback ? `${stamp} Refreshing…` : "Reading the host playback snapshot…";
  }
  if (playbackError) {
    return playback
      ? `${stamp} Refresh failed: ${playbackError} Showing the last read.`
      : `Could not read playback: ${playbackError}`;
  }
  if (!playback) return "Read the current track with Refresh playback.";
  if (!playback.connected) return `${stamp} Spotify is not connected. Connect it in Luma Settings, then refresh.`;
  if (!playback.title) return `${stamp} Nothing is playing. Start playback in Luma, then refresh.`;
  return `${stamp} Showing the last published host snapshot. Read-only.`;
}

function playbackCardState() {
  if (refreshing) return "loading";
  if (playbackError) return "error";
  if (!playback) return "idle";
  if (!playback.connected) return "disconnected";
  if (!playback.title) return "empty";
  return "live";
}

function renderPlayback() {
  nowPlaying.dataset.state = playbackCardState();
  show(playbackConnection, describeConnection());
  show(playbackTitle, describeTitle());
  show(playbackArtist, playback?.title ? playback.artist ?? "" : "");
  playbackStateTag.hidden = !Boolean(playback?.connected && playback?.title && !playbackError);
  playbackStateTag.dataset.playing = String(Boolean(playback?.playing));
  show(playbackStateTag, playback?.playing ? "Playing" : "Paused");
  show(playbackStatus, describeStatus());
  refreshButton.disabled = refreshing;
  refreshButton.classList.toggle("is-loading", refreshing);
  refreshButton.setAttribute("aria-busy", String(refreshing));
}

async function refreshPlayback() {
  if (refreshing) return;
  const restoreFocus = document.activeElement === refreshButton;
  refreshing = true;
  playbackError = null;
  renderPlayback();
  try {
    const value = await host("playback");
    playback = {
      connected: value.connected === true,
      title: value.title ?? null,
      artist: value.artist ?? null,
      playing: value.playing === true,
    };
    playbackAt = new Date();
  } catch (error) {
    playbackError = String(error);
  } finally {
    refreshing = false;
    renderPlayback();
    if (restoreFocus && document.activeElement === document.body) {
      refreshButton.focus();
    }
  }
}

async function loadIdentity() {
  try {
    const value = await host("identity");
    show(identity, `${value.package} ${value.version} · ${value.origin}`);
  } catch (error) {
    show(identity, `Denied: ${error}`);
  }
}

async function ping() {
  try {
    const value = await host("ping");
    show(pingResult, `Host pong ${value.pongs} from ${value.source}`);
  } catch (error) {
    show(pingResult, `Denied: ${error}`);
  }
}

async function probe() {
  const attempts = [
    ["app command launch_app", () => invoke("launch_app", { id: "Custom", fullscreen: false })],
    ["core window close", () => invoke("plugin:window|close", { label: "extension-spike" })],
    ["core event emit", () => invoke("plugin:event|emit", { event: "spike", payload: null })],
    ["spoofed identity", () => invoke("extension_spike_action", { action: "identity", package: "com.example.evil" })],
  ];
  const lines = [];
  for (const [label, attempt] of attempts) {
    try {
      await attempt();
      lines.push(`${label}: ALLOWED`);
    } catch (error) {
      lines.push(`${label}: denied (${error})`);
    }
  }
  show(denial, lines.join("\n"));
}

function exit() {
  host("close").catch((error) => {
    show(note, `Could not close: ${error}`);
  });
}

document.querySelector("#ping").addEventListener("click", ping);
refreshButton.addEventListener("click", refreshPlayback);
document.querySelector("#probe").addEventListener("click", probe);
document.querySelector("#close").addEventListener("click", exit);
document.querySelector("#crash").addEventListener("click", () => {
  show(note, "Throwing page JavaScript. This is not a renderer crash test.");
  setTimeout(() => {
    throw new Error("Extension spike crash test");
  }, 0);
});

window.addEventListener("error", (event) => {
  show(note, `Uncaught: ${event.message}. Check that Home, Settings, and the tray still respond.`);
});

const buttons = () => Array.from(document.querySelectorAll("button:not(:disabled)"));

function moveFocus(step) {
  const items = buttons();
  if (!items.length) return;
  const current = items.indexOf(document.activeElement);
  const next =
    current === -1
      ? step > 0
        ? 0
        : items.length - 1
      : (current + step + items.length) % items.length;
  items[next].focus();
}

window.addEventListener("keydown", (event) => {
  if (event.key === "Escape" || event.key === "BrowserBack") {
    event.preventDefault();
    exit();
    return;
  }
  const step =
    event.key === "ArrowRight" || event.key === "ArrowDown"
      ? 1
      : event.key === "ArrowLeft" || event.key === "ArrowUp"
        ? -1
        : 0;
  if (step === 0) return;
  event.preventDefault();
  moveFocus(step);
});

history.pushState({ spike: true }, "");
window.addEventListener("popstate", exit);

document.querySelector("#close").focus();
loadIdentity();
refreshPlayback();
