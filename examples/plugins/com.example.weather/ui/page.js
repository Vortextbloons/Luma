const invoke = (command, args) => window.__TAURI_INTERNALS__.invoke(command, args);
const bridge = (op, args) =>
  args === undefined ? invoke("extension_bridge", { op }) : invoke("extension_bridge", { op, args });

const surface = document.querySelector("#surface");
const place = document.querySelector("#place");
const stateLabel = document.querySelector("#state");
const temperature = document.querySelector("#temperature");
const condition = document.querySelector("#condition");
const observed = document.querySelector("#observed");
const status = document.querySelector("#status");
const refresh = document.querySelector("#refresh");
const close = document.querySelector("#close");
const identityElement = document.querySelector("#identity");

const locations = {
  seattle: { label: "Seattle", latitude: 47.6062, longitude: -122.3321 },
  "los-angeles": { label: "Los Angeles", latitude: 34.0522, longitude: -118.2437 },
  chicago: { label: "Chicago", latitude: 41.8781, longitude: -87.6298 },
  "new-york": { label: "New York", latitude: 40.7128, longitude: -74.0060 },
  toronto: { label: "Toronto", latitude: 43.6532, longitude: -79.3832 },
  london: { label: "London", latitude: 51.5074, longitude: -0.1278 },
  paris: { label: "Paris", latitude: 48.8566, longitude: 2.3522 },
  berlin: { label: "Berlin", latitude: 52.5200, longitude: 13.4050 },
  tokyo: { label: "Tokyo", latitude: 35.6762, longitude: 139.6503 },
  sydney: { label: "Sydney", latitude: -33.8688, longitude: 151.2093 },
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

const states = {
  loading: "Loading",
  ready: "Ready",
  stale: "Last reading",
  error: "Unavailable",
  empty: "No location",
};

let settings = null;
let settingsError = null;
let identity = null;
let identityError = null;
let reading = null;
let error = null;
let busy = false;
let attempted = false;

const unitName = units => (units === "f" ? "Fahrenheit" : "Celsius");
const unitMark = units => (units === "f" ? "°F" : "°C");
const conditionName = code => conditions.get(code) ?? "Weather";
const time = value => new Date(value * 1000).toLocaleTimeString();

function messageOf(value) {
  if (typeof value === "string" && value.trim()) return value.trim();
  if (value && typeof value === "object" && typeof value.message === "string" && value.message.trim()) {
    return value.message.trim();
  }
  return "The host rejected the request.";
}

function selection() {
  const values = settings?.values;
  if (!values || typeof values !== "object") return null;
  const location = locations[values.location];
  if (!location) return null;
  return {
    id: values.location,
    label: location.label,
    latitude: location.latitude,
    longitude: location.longitude,
    units: values.units === "f" ? "f" : "c",
  };
}

function surfaceState() {
  if (settingsError) return "error";
  if (!settings) return "loading";
  if (!selection()) return "empty";
  if (busy && !reading) return "loading";
  if (error) return reading ? "stale" : "error";
  return reading ? "ready" : "loading";
}

function describeStatus(current) {
  if (settingsError) return `Could not read this package's settings: ${settingsError}`;
  if (!settings) return "Reading package settings…";
  if (!current) return "Choose a location in Luma Settings > Plugins, then refresh.";
  if (busy) {
    return reading
      ? `Refreshing ${current.label} in ${unitName(current.units)}. The last reading stays visible.`
      : `Loading weather for ${current.label} in ${unitName(current.units)}…`;
  }
  if (error) {
    return reading
      ? `Couldn't refresh ${reading.label} in ${unitName(reading.units)}: ${error.message}`
      : error.message;
  }
  return reading
    ? `Showing ${reading.label} in ${unitName(reading.units)}. Press Refresh to read again.`
    : `Ready to read ${current.label} in ${unitName(current.units)}.`;
}

function render() {
  const current = selection();
  const state = surfaceState();
  surface.dataset.state = state;
  stateLabel.textContent = states[state];
  place.textContent = current?.label ?? (settingsError ? "Weather unavailable" : settings ? "Location not set" : "Reading settings…");
  temperature.textContent = reading
    ? `${Math.round(reading.temperature)}${unitMark(reading.units)}`
    : "--";
  condition.textContent = reading
    ? conditionName(reading.weatherCode)
    : busy
      ? "Reading the sky…"
      : error
        ? "No reading available"
        : "Waiting for the first reading";
  observed.textContent = reading ? `Observed ${time(reading.observedAt)}` : "";
  status.textContent = describeStatus(current);
  refresh.setAttribute("aria-disabled", String(busy));
  refresh.setAttribute("aria-busy", String(busy));
  refresh.classList.toggle("is-loading", busy);
  refresh.textContent = busy ? (reading ? "Refreshing…" : "Loading…") : "Refresh";
  identityElement.textContent = identity
    ? `${identity.id} ${identity.version} · API ${identity.apiVersion}`
    : identityError
      ? `Identity unavailable: ${identityError}`
      : "";
}

function toReading(value, current) {
  const payload = value?.current;
  if (!payload || typeof payload !== "object") return null;
  const valueTemperature = payload.temperature_2m;
  const weatherCode = payload.weather_code;
  const observedAt = payload.time;
  if (typeof valueTemperature !== "number" || !Number.isFinite(valueTemperature)) return null;
  if (typeof weatherCode !== "number" || !Number.isInteger(weatherCode) || weatherCode < 0 || weatherCode > 99) return null;
  if (typeof observedAt !== "number" || !Number.isFinite(observedAt) || observedAt <= 0) return null;
  return {
    label: current.label,
    units: current.units,
    temperature: valueTemperature,
    weatherCode,
    observedAt,
  };
}

async function loadWeather() {
  const current = selection();
  if (busy || !settings) return;
  attempted = true;
  if (!current) {
    error = { message: "Choose a location in Luma Settings > Plugins." };
    render();
    return;
  }
  const restoreFocus = document.activeElement === refresh;
  busy = true;
  error = null;
  render();
  try {
    const value = await bridge("net.get", {
      origin: "https://api.open-meteo.com",
      path: "/v1/forecast",
      query: [
        ["latitude", current.latitude.toFixed(4)],
        ["longitude", current.longitude.toFixed(4)],
        ["current", "temperature_2m,weather_code"],
        ["temperature_unit", current.units === "f" ? "fahrenheit" : "celsius"],
        ["timeformat", "unixtime"],
      ],
    });
    const next = toReading(value, current);
    if (!next) throw new Error("The host returned an unreadable weather response.");
    reading = next;
  } catch (cause) {
    error = { message: messageOf(cause) };
  } finally {
    busy = false;
    render();
    if (restoreFocus && document.activeElement === document.body) refresh.focus();
  }
}

function exit() {
  bridge("surface.close").catch(cause => {
    status.textContent = `Could not close this surface: ${messageOf(cause)}`;
  });
}

function focusables() {
  return Array.from(document.querySelectorAll("button"));
}

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

window.addEventListener("keydown", event => {
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

async function initialize() {
  render();
  const [identityResult, settingsResult] = await Promise.allSettled([
    bridge("identity"),
    bridge("settings.read"),
  ]);
  if (identityResult.status === "fulfilled") {
    identity = identityResult.value;
  } else {
    identityError = messageOf(identityResult.reason);
  }
  if (settingsResult.status === "fulfilled" && settingsResult.value?.values) {
    settings = { values: settingsResult.value.values };
  } else {
    settingsError = messageOf(settingsResult.reason);
  }
  render();
  refresh.focus();
  try {
    await bridge("surface.ready");
  } catch (cause) {
    error = { message: `This package surface could not start: ${messageOf(cause)}` };
    render();
    return;
  }
  if (settingsError) return;
  await loadWeather();
}

refresh.addEventListener("click", loadWeather);
close.addEventListener("click", exit);
window.addEventListener("luma:command", event => {
  if (event?.detail?.command === "openForecast" && !attempted) void loadWeather();
});
history.pushState({ surface: "weather" }, "");
window.addEventListener("popstate", exit);
initialize();
