<script lang="ts">
  import { onMount } from "svelte";
  import {
    bridge,
    onCommand,
    toBridgeError,
    type Identity,
    type NetworkRequest,
    type SettingValue,
  } from "@luma/plugin-sdk";

  interface LocationConfig {
    label: string;
    latitude: number;
    longitude: number;
  }

  interface WeatherReading {
    label: string;
    units: "c" | "f";
    temperature: number;
    weatherCode: number;
    observedAt: number;
  }

  const locations: Record<string, LocationConfig> = {
    seattle: { label: "Seattle", latitude: 47.6062, longitude: -122.3321 },
    "los-angeles": { label: "Los Angeles", latitude: 34.0522, longitude: -118.2437 },
    chicago: { label: "Chicago", latitude: 41.8781, longitude: -87.6298 },
    "new-york": { label: "New York", latitude: 40.7128, longitude: -74.006 },
    toronto: { label: "Toronto", latitude: 43.6532, longitude: -79.3832 },
    london: { label: "London", latitude: 51.5074, longitude: -0.1278 },
    paris: { label: "Paris", latitude: 48.8566, longitude: 2.3522 },
    berlin: { label: "Berlin", latitude: 52.52, longitude: 13.405 },
    tokyo: { label: "Tokyo", latitude: 35.6762, longitude: 139.6503 },
    sydney: { label: "Sydney", latitude: -33.8688, longitude: 151.2093 },
  };

  const conditions = new Map<number, string>([
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

  const stateLabels: Record<string, string> = {
    loading: "Loading",
    ready: "Ready",
    stale: "Last reading",
    error: "Unavailable",
    empty: "No location",
  };

  let settings = $state<{ values: Record<string, SettingValue> } | null>(null);
  let settingsError = $state<string | null>(null);
  let identity = $state<Identity | null>(null);
  let identityError = $state<string | null>(null);
  let reading = $state<WeatherReading | null>(null);
  let requestError = $state<string | null>(null);
  let busy = $state(false);
  let attempted = $state(false);
  let pendingWeather: Promise<void> | null = null;

  let refreshButton: HTMLButtonElement | null = $state(null);
  let closeButton: HTMLButtonElement | null = $state(null);

  const unitName = (units: "c" | "f") => (units === "f" ? "Fahrenheit" : "Celsius");
  const unitMark = (units: "c" | "f") => (units === "f" ? "°F" : "°C");
  const conditionName = (code: number) => conditions.get(code) ?? "Weather";
  const formatTime = (value: number) => new Date(value * 1000).toLocaleTimeString();

  function messageOf(cause: unknown): string {
    return toBridgeError(cause).message;
  }

  const selectedLocation = $derived.by(() => {
    const values = settings?.values;
    if (!values || typeof values !== "object") return null;
    const locationKey = values.location;
    if (typeof locationKey !== "string") return null;
    const location = locations[locationKey];
    if (!location) return null;
    return {
      id: locationKey,
      label: location.label,
      latitude: location.latitude,
      longitude: location.longitude,
      units: values.units === "f" ? ("f" as const) : ("c" as const),
    };
  });

  const surfaceState = $derived.by(() => {
    if (settingsError) return "error";
    if (!settings) return "loading";
    if (!selectedLocation) return "empty";
    if (busy && !reading) return "loading";
    if (requestError) return reading ? "stale" : "error";
    return reading ? "ready" : "loading";
  });

  const placeText = $derived.by(() => {
    if (selectedLocation) return selectedLocation.label;
    if (settingsError) return "Weather unavailable";
    if (settings) return "Location not set";
    return "Reading settings…";
  });

  const statusText = $derived.by(() => {
    if (settingsError) return `Could not read this package's settings: ${settingsError}`;
    if (!settings) return "Reading package settings…";
    if (!selectedLocation) return "Choose a location in Luma Settings > Plugins, then refresh.";
    if (busy) {
      return reading
        ? `Refreshing ${selectedLocation.label} in ${unitName(selectedLocation.units)}. The last reading stays visible.`
        : `Loading weather for ${selectedLocation.label} in ${unitName(selectedLocation.units)}…`;
    }
    if (requestError) {
      return reading
        ? `Couldn't refresh ${reading.label} in ${unitName(reading.units)}: ${requestError}`
        : requestError;
    }
    return reading
      ? `Showing ${reading.label} in ${unitName(reading.units)}. Press Refresh to read again.`
      : `Ready to read ${selectedLocation.label} in ${unitName(selectedLocation.units)}.`;
  });

  const temperatureText = $derived(
    reading ? `${Math.round(reading.temperature)}${unitMark(reading.units)}` : "--",
  );

  const conditionText = $derived.by(() => {
    if (reading) return conditionName(reading.weatherCode);
    if (busy) return "Reading the sky…";
    if (requestError) return "No reading available";
    return "Waiting for the first reading";
  });

  const observedText = $derived(
    reading ? `Observed ${formatTime(reading.observedAt)}` : "",
  );

  const identityText = $derived.by(() => {
    if (identity) return `${identity.id} ${identity.version} · API ${identity.apiVersion}`;
    if (identityError) return `Identity unavailable: ${identityError}`;
    return "";
  });

  function parseReading(
    value: unknown,
    current: NonNullable<typeof selectedLocation>,
  ): WeatherReading | null {
    if (!value || typeof value !== "object") return null;
    const payload = (value as { current?: Record<string, unknown> }).current;
    if (!payload || typeof payload !== "object") return null;
    const valueTemperature = payload.temperature_2m;
    const weatherCode = payload.weather_code;
    const observedAt = payload.time;
    if (typeof valueTemperature !== "number" || !Number.isFinite(valueTemperature)) return null;
    if (
      typeof weatherCode !== "number" ||
      !Number.isInteger(weatherCode) ||
      weatherCode < 0 ||
      weatherCode > 99
    ) {
      return null;
    }
    if (typeof observedAt !== "number" || !Number.isFinite(observedAt) || observedAt <= 0) {
      return null;
    }
    return {
      label: current.label,
      units: current.units,
      temperature: valueTemperature,
      weatherCode,
      observedAt,
    };
  }

  function loadWeather() {
    pendingWeather ??= fetchWeather().finally(() => { pendingWeather = null; });
    return pendingWeather;
  }

  async function fetchWeather() {
    const current = selectedLocation;
    if (busy || !settings) return;
    attempted = true;
    if (!current) {
      requestError = "Choose a location in Luma Settings > Plugins.";
      return;
    }
    busy = true;
    requestError = null;
    try {
      const request: NetworkRequest = {
        origin: "https://api.open-meteo.com",
        path: "/v1/forecast",
        query: [
          ["latitude", current.latitude.toFixed(4)],
          ["longitude", current.longitude.toFixed(4)],
          ["current", "temperature_2m,weather_code"],
          ["temperature_unit", current.units === "f" ? "fahrenheit" : "celsius"],
          ["timeformat", "unixtime"],
        ],
      };
      const value = await bridge.get(request);
      const next = parseReading(value, current);
      if (!next) throw new Error("The host returned an unreadable weather response.");
      reading = next;
    } catch (cause) {
      requestError = messageOf(cause);
    } finally {
      busy = false;
    }
  }

  function exit() {
    bridge.close().catch(cause => {
      requestError = `Could not close this surface: ${messageOf(cause)}`;
    });
  }

  function focusables(): HTMLButtonElement[] {
    const list: HTMLButtonElement[] = [];
    if (refreshButton) list.push(refreshButton);
    if (closeButton) list.push(closeButton);
    return list;
  }

  function moveFocus(step: number) {
    const items = focusables();
    if (!items.length) return;
    const current = items.indexOf(document.activeElement as HTMLButtonElement);
    const next =
      current === -1
        ? step > 0
          ? 0
          : items.length - 1
        : (current + step + items.length) % items.length;
    items[next].focus();
  }

  function handleKeydown(event: KeyboardEvent) {
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
  }

  onMount(() => {
    window.addEventListener("keydown", handleKeydown);
    history.pushState({ surface: "weather" }, "");
    window.addEventListener("popstate", exit);

    const unsubscribeCommand = onCommand(async ({ command, invocationId }) => {
      if (command !== "openForecast") return;
      await (attempted ? pendingWeather : loadWeather());
      if (invocationId) {
        const success = Boolean(reading && !requestError && !settingsError);
        await bridge.reportDevelopmentOutcome({
          invocationId,
          success,
          message: success ? "Forecast loaded." : settingsError ?? requestError ?? "Forecast unavailable.",
        }).catch(() => {});
      }
    });

    void initialize();

    return () => {
      window.removeEventListener("keydown", handleKeydown);
      window.removeEventListener("popstate", exit);
      unsubscribeCommand();
    };
  });

  async function initialize() {
    refreshButton?.focus();

    const [identityResult, settingsResult] = await Promise.allSettled([
      bridge.identity(),
      bridge.readSettings(),
    ]);

    if (identityResult.status === "fulfilled") {
      identity = identityResult.value;
    } else {
      identityError = messageOf(identityResult.reason);
    }

    if (settingsResult.status === "rejected") {
      settingsError = messageOf(settingsResult.reason);
    } else {
      settings = settingsResult.value;
    }

    try {
      await bridge.ready();
    } catch (cause) {
      requestError = `This package surface could not start: ${messageOf(cause)}`;
      return;
    }

    if (settingsError) return;
    await loadWeather();
  }
</script>

<main
  id="surface"
  class="surface flex flex-col gap-4.5 min-h-screen px-7.5 pt-7 pb-6"
  data-state={surfaceState}
>
  <header class="masthead flex items-start justify-between gap-4 pb-4 border-b border-[#aabed433]">
    <div>
      <p class="package m-0 mb-1.5 text-[13px] tracking-[0.04em] text-[#91a4ba]">Weather package</p>
      <h1 id="place" class="m-0 text-3xl sm:text-4xl font-semibold tracking-[-0.025em] leading-tight text-[#e8eef5]">
        {placeText}
      </h1>
    </div>
    <p class="connection flex items-center gap-2 mt-1 m-0 text-[13px] text-[#9db0c4] whitespace-nowrap">
      <span
        class="dot w-2.25 h-2.25 shrink-0 rounded-full"
        class:bg-[#8fd6b4]={surfaceState === "ready"}
        class:bg-[#86a9e6]={surfaceState === "loading"}
        class:bg-[#d9ae6b]={surfaceState === "stale"}
        class:bg-[#e09a91]={surfaceState === "error" || surfaceState === "empty"}
        aria-hidden="true"
      ></span>
      <span id="state">{stateLabels[surfaceState]}</span>
    </p>
  </header>

  <section class="reading flex items-end gap-4.5 py-1 pb-2" aria-live="polite">
    <p id="temperature" class="temperature m-0 font-light leading-[0.82] tracking-[-0.055em] tabular-nums text-[#f4f0e7] text-7xl sm:text-9xl">
      {temperatureText}
    </p>
    <div class="details grid gap-1 pb-2">
      <p id="condition" class="condition m-0 text-lg sm:text-2xl font-medium leading-tight text-[#e7d8bd]">
        {conditionText}
      </p>
      <p id="observed" class="observed m-0 min-h-[18px] text-[13px] text-[#91a4ba] tabular-nums">
        {observedText}
      </p>
    </div>
  </section>

  <p
    id="status"
    class="status m-0 min-h-[42px] text-sm leading-relaxed text-[#b8c6d4] break-words"
    class:text-[#e0bc84]={surfaceState === "stale"}
    class:text-[#e8b0a8]={surfaceState === "error" || surfaceState === "empty"}
    role="status"
  >
    {statusText}
  </p>

  <div class="actions flex flex-wrap gap-2.5">
    <button
      bind:this={refreshButton}
      type="button"
      id="refresh"
      class="primary min-h-11 px-4.5 py-2.5 rounded-lg border border-transparent bg-[#c79a52] text-[#1a140b] font-semibold text-sm cursor-pointer hover:bg-[#d7ad69] focus-visible:outline-3 focus-visible:outline-[#d9ae6b] focus-visible:outline-offset-3 aria-disabled:opacity-60 aria-disabled:cursor-default"
      class:is-loading={busy}
      aria-disabled={busy}
      aria-busy={busy}
      onclick={loadWeather}
    >
      {busy ? (reading ? "Refreshing…" : "Loading…") : "Refresh"}
    </button>
    <button
      bind:this={closeButton}
      type="button"
      id="close"
      class="min-h-11 px-4.5 py-2.5 rounded-lg border border-[#aabed452] bg-[#182332] text-[#e8eef5] text-sm cursor-pointer hover:border-[#cddbea94] focus-visible:outline-3 focus-visible:outline-[#d9ae6b] focus-visible:outline-offset-3"
      onclick={exit}
    >
      Close
    </button>
  </div>

  <footer class="footer flex items-end justify-between gap-4 mt-auto pt-4 border-t border-[#aabed433]">
    <p class="attribution m-0 text-xs leading-relaxed text-[#8397ac] break-words">
      Weather data by Open-Meteo.com<br />
      <span class="text-[#a5b6c8]">https://open-meteo.com/</span>
    </p>
    <p id="identity" class="identity m-0 text-xs leading-relaxed text-[#8397ac] text-right break-words">
      {identityText}
    </p>
  </footer>
</main>
