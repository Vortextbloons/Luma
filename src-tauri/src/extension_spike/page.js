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
const playbackControlStatus = document.querySelector("#playback-control-status");
const previousButton = document.querySelector("#playback-previous");
const toggleButton = document.querySelector("#playback-toggle");
const nextButton = document.querySelector("#playback-next");
const refreshButton = document.querySelector("#refresh");
const weather = document.querySelector("#weather");
const weatherStateLabel = document.querySelector("#weather-state");
const weatherLocation = document.querySelector("#weather-location");
const weatherUnits = document.querySelector("#weather-units");
const weatherLoad = document.querySelector("#weather-load");
const weatherReadingElement = document.querySelector("#weather-reading");
const weatherPlace = document.querySelector("#weather-place");
const weatherTemperature = document.querySelector("#weather-temperature");
const weatherCondition = document.querySelector("#weather-condition");
const weatherObserved = document.querySelector("#weather-observed");
const weatherStatus = document.querySelector("#weather-status");

const show = (element, text) => {
  element.textContent = text;
};

const controlButtons = {
  previous: previousButton,
  toggle: toggleButton,
  next: nextButton,
};

let playback = null;
let playbackAt = null;
let playbackError = null;
let refreshing = false;
let controlBusy = null;
let controlFeedback = null;

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
  return `${stamp} Showing the last published host snapshot.`;
}

const playable = () => Boolean(playback?.connected && playback.title);

const controlLabel = (action) => {
  if (action === "previous") return "Previous track";
  if (action === "next") return "Next track";
  return playback?.playing ? "Pause" : "Play";
};

function describeControlStatus() {
  if (controlBusy) return `Sending ${controlBusy.name}…`;
  return controlFeedback?.text ?? "";
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
  show(playbackControlStatus, describeControlStatus());
  playbackControlStatus.dataset.tone = controlBusy
    ? "pending"
    : controlFeedback?.tone ?? "";
  const locked = !playable() || refreshing || controlBusy !== null;
  for (const [action, button] of Object.entries(controlButtons)) {
    button.disabled = locked;
    button.classList.toggle("is-loading", controlBusy?.action === action);
    button.setAttribute("aria-busy", String(controlBusy?.action === action));
  }
  show(toggleButton, playback?.playing ? "Pause" : "Play");
  refreshButton.disabled = refreshing || controlBusy !== null;
  refreshButton.classList.toggle("is-loading", refreshing);
  refreshButton.setAttribute("aria-busy", String(refreshing));
}

async function refreshPlayback() {
  if (refreshing || controlBusy) return;
  const restoreFocus = document.activeElement === refreshButton;
  refreshing = true;
  playbackError = null;
  controlFeedback = null;
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

async function sendControl(action) {
  if (controlBusy || refreshing || !playable()) return;
  const button = controlButtons[action];
  const name = controlLabel(action);
  const restoreFocus = document.activeElement === button;
  controlBusy = { action, name };
  controlFeedback = null;
  renderPlayback();
  try {
    const value = await host(`playback-${action}`);
    if (value?.accepted !== true) throw "the host did not accept the command";
    controlFeedback = {
      tone: "success",
      text: `${name} accepted. Refresh playback to see the updated snapshot.`,
    };
  } catch (error) {
    controlFeedback = {
      tone: "error",
      text: `${name} failed: ${error}. The last read is still shown.`,
    };
  } finally {
    controlBusy = null;
    renderPlayback();
    if (restoreFocus && document.activeElement === document.body) {
      button.focus();
    }
  }
}

let weatherReading = null;
let weatherError = null;
let weatherBusy = false;
let weatherPending = null;

const weatherStates = {
  idle: "Not loaded",
  loading: "Loading",
  ready: "Loaded",
  stale: "Last result",
  error: "Failed",
};

const conditions = new Map([
  [0, "Clear"],
  [1, "Mainly clear"],
  [2, "Partly cloudy"],
  [3, "Overcast"],
  [45, "Fog"],
  [48, "Fog"],
  [51, "Drizzle"],
  [53, "Drizzle"],
  [55, "Drizzle"],
  [56, "Freezing drizzle"],
  [57, "Freezing drizzle"],
  [61, "Rain"],
  [63, "Rain"],
  [65, "Rain"],
  [66, "Freezing rain"],
  [67, "Freezing rain"],
  [71, "Snow"],
  [73, "Snow"],
  [75, "Snow"],
  [77, "Snow"],
  [80, "Rain showers"],
  [81, "Rain showers"],
  [82, "Rain showers"],
  [85, "Snow showers"],
  [86, "Snow showers"],
  [95, "Thunderstorm"],
  [96, "Thunderstorm"],
  [99, "Thunderstorm"],
]);

const unitName = (units) => (units === "f" ? "Fahrenheit" : "Celsius");
const unitMark = (units) => (units === "f" ? "°F" : "°C");
const conditionName = (code) => conditions.get(code) ?? "Weather";

function currentSelection() {
  const option = weatherLocation.selectedOptions[0];
  return {
    location: weatherLocation.value,
    label: option ? option.textContent : weatherLocation.value,
    units: weatherUnits.value === "f" ? "f" : "c",
  };
}

const selectionDiffers = (selection) =>
  selection.location !== weatherReading.location ||
  selection.units !== weatherReading.units;

function weatherCardState() {
  if (weatherBusy) return "loading";
  if (weatherError) return weatherReading ? "stale" : "error";
  if (!weatherReading) return "idle";
  return selectionDiffers(currentSelection()) ? "stale" : "ready";
}

function describeWeather() {
  if (weatherBusy && weatherPending) {
    const target = `${weatherPending.label} in ${unitName(weatherPending.units)}`;
    return weatherReading
      ? `Refreshing weather for ${target}. The last result is still shown.`
      : `Loading weather for ${target}…`;
  }
  if (weatherError) {
    const failure = `Could not load weather for ${weatherError.label} in ${unitName(weatherError.units)}: ${weatherError.message}`;
    return weatherReading
      ? `${failure} Showing the last result for ${weatherReading.label}.`
      : failure;
  }
  if (!weatherReading) return "Choose a city and press Load weather.";
  const selection = currentSelection();
  if (selectionDiffers(selection)) {
    return `Showing the last result for ${weatherReading.label} in ${unitName(weatherReading.units)}. Press Refresh weather for ${selection.label} in ${unitName(selection.units)}.`;
  }
  return `Showing the result for ${weatherReading.label}. Press Refresh weather to check again.`;
}

function renderWeather() {
  const state = weatherCardState();
  weather.dataset.state = state;
  show(weatherStateLabel, weatherStates[state]);
  weatherReadingElement.hidden = !weatherReading;
  if (weatherReading) {
    show(weatherPlace, weatherReading.label);
    show(weatherTemperature, `${Math.round(weatherReading.temperature)}${unitMark(weatherReading.units)}`);
    show(weatherCondition, conditionName(weatherReading.weatherCode));
    show(weatherObserved, `Observed ${time(new Date(weatherReading.observedAt * 1000))}`);
  } else {
    show(weatherPlace, "");
    show(weatherTemperature, "");
    show(weatherCondition, "");
    show(weatherObserved, "");
  }
  show(weatherStatus, describeWeather());
  const busy = weatherBusy;
  weatherLoad.setAttribute("aria-disabled", String(busy));
  weatherLoad.setAttribute("aria-busy", String(busy));
  weatherLoad.classList.toggle("is-loading", busy);
  show(
    weatherLoad,
    busy
      ? weatherReading
        ? "Refreshing weather…"
        : "Loading weather…"
      : weatherReading
        ? "Refresh weather"
        : "Load weather",
  );
}

function toReading(value, selection) {
  if (!value || typeof value !== "object") return null;
  if (value.units !== selection.units) return null;
  if (typeof value.temperature !== "number" || !Number.isFinite(value.temperature)) return null;
  if (typeof value.weatherCode !== "number" || !Number.isFinite(value.weatherCode)) return null;
  if (typeof value.observedAt !== "number" || !Number.isFinite(value.observedAt)) return null;
  if (value.observedAt <= 0) return null;
  return {
    ...selection,
    temperature: value.temperature,
    weatherCode: value.weatherCode,
    observedAt: value.observedAt,
  };
}

function messageOf(error) {
  if (typeof error === "string" && error.trim()) return error.trim();
  if (error && typeof error === "object" && typeof error.message === "string" && error.message.trim()) {
    return error.message.trim();
  }
  return "The host rejected the request.";
}

async function loadWeather() {
  if (weatherBusy) return;
  const selection = currentSelection();
  const restoreFocus = document.activeElement === weatherLoad;
  weatherBusy = true;
  weatherPending = selection;
  weatherError = null;
  renderWeather();
  try {
    const value = await invoke("extension_spike_action", {
      action: "weather-read",
      location: selection.location,
      units: selection.units,
    });
    const reading = toReading(value, selection);
    if (!reading) throw "The host returned an unreadable reading.";
    weatherReading = reading;
  } catch (error) {
    weatherError = {
      label: selection.label,
      units: selection.units,
      message: messageOf(error),
    };
  } finally {
    weatherBusy = false;
    weatherPending = null;
    renderWeather();
    if (restoreFocus && document.activeElement === document.body) {
      weatherLoad.focus();
    }
  }
}

function stepOption(select, step) {
  const count = select.options.length;
  if (!count) return;
  const next = Math.min(count - 1, Math.max(0, select.selectedIndex + step));
  if (next === select.selectedIndex) return;
  select.selectedIndex = next;
  renderWeather();
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
previousButton.addEventListener("click", () => sendControl("previous"));
toggleButton.addEventListener("click", () => sendControl("toggle"));
nextButton.addEventListener("click", () => sendControl("next"));
weatherLoad.addEventListener("click", loadWeather);
weatherLocation.addEventListener("change", renderWeather);
weatherUnits.addEventListener("change", renderWeather);
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

const focusables = () =>
  Array.from(document.querySelectorAll("select:not(:disabled), button:not(:disabled)"));

function moveFocus(step) {
  const items = focusables();
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

const selects = [weatherLocation, weatherUnits];

window.addEventListener("keydown", (event) => {
  if (event.key === "Escape" || event.key === "BrowserBack") {
    event.preventDefault();
    exit();
    return;
  }
  if (selects.includes(document.activeElement)) {
    if (event.key === "ArrowUp" || event.key === "ArrowDown") {
      event.preventDefault();
      stepOption(document.activeElement, event.key === "ArrowDown" ? 1 : -1);
      return;
    }
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
renderWeather();
loadIdentity();
refreshPlayback();
