import { readFileSync } from "node:fs";
import vm from "node:vm";
import test from "node:test";
import assert from "node:assert/strict";

const source = readFileSync(new URL("./com.example.weather/ui/page.js", import.meta.url), "utf8");
const markup = readFileSync(new URL("./com.example.weather/ui/index.html", import.meta.url), "utf8");
const manifest = JSON.parse(readFileSync(new URL("./com.example.weather/luma-extension.json", import.meta.url), "utf8"));

const ids = [
  "surface",
  "place",
  "state",
  "temperature",
  "condition",
  "observed",
  "status",
  "refresh",
  "close",
  "identity",
];

const flush = () => new Promise(resolve => setImmediate(resolve));

function createPage(options = {}) {
  const document = { activeElement: null, body: { id: "body" } };
  document.activeElement = document.body;

  const nodes = new Map();
  for (const id of ids) {
    const classes = new Set();
    const listeners = new Map();
    const node = {
      id,
      textContent: "",
      dataset: {},
      attributes: {},
      focusCount: 0,
      listeners,
      classList: {
        contains: name => classes.has(name),
        toggle(name, on) {
          if (on) classes.add(name);
          else classes.delete(name);
        },
      },
      addEventListener: (name, listener) => listeners.set(name, listener),
      setAttribute(name, value) {
        node.attributes[name] = value;
      },
      focus() {
        node.focusCount++;
        document.activeElement = node;
      },
    };
    nodes.set(id, node);
  }

  document.querySelector = selector => nodes.get(selector.slice(1)) ?? null;
  document.querySelectorAll = selector => {
    assert.equal(selector, "button");
    return [nodes.get("refresh"), nodes.get("close")];
  };

  const calls = [];
  const pending = [];
  const listeners = new Map();
  const settings = options.settings ?? {
    values: { location: "seattle", units: "c" },
  };
  const identity = options.identity ?? {
    id: "com.example.weather",
    name: "Weather",
    version: "1.0.0",
    apiVersion: 2,
    runtime: "isolated-ui",
    surface: "desktop",
  };
  const window = {
    __TAURI_INTERNALS__: {
      invoke(command, args) {
        calls.push({ command, args: { ...args } });
        if (command !== "extension_bridge") return Promise.reject("Unexpected command.");
        if (args.op === "identity") {
          return options.identityFailure
            ? Promise.reject(options.identityFailure)
            : Promise.resolve(identity);
        }
        if (args.op === "settings.read") {
          return options.settingsFailure
            ? Promise.reject(options.settingsFailure)
            : Promise.resolve(settings);
        }
        if (args.op === "surface.ready") return Promise.resolve({ ready: true });
        if (args.op === "surface.close") return Promise.resolve({ closed: true });
        if (args.op === "net.get") {
          return new Promise((resolve, reject) => pending.push({ resolve, reject }));
        }
        return Promise.reject("Unknown bridge operation.");
      },
    },
    addEventListener: (name, listener) => listeners.set(name, listener),
  };

  vm.runInNewContext(source, {
    window,
    document,
    history: { pushState() {} },
    URL,
  });

  const press = key => {
    const event = {
      key,
      defaultPrevented: false,
      preventDefault() {
        event.defaultPrevented = true;
      },
    };
    listeners.get("keydown")(event);
    return event;
  };

  return {
    document,
    node: id => nodes.get(id),
    calls,
    pending,
    press,
    click: id => nodes.get(id).listeners.get("click")(),
    netCalls: () => calls.filter(call => call.args.op === "net.get"),
    callsFor: op => calls.filter(call => call.args.op === op),
    resolveNet(value) {
      const next = pending.shift();
      assert.ok(next, "a network request is waiting");
      next.resolve(value);
    },
    rejectNet(value) {
      const next = pending.shift();
      assert.ok(next, "a network request is waiting");
      next.reject(value);
    },
  };
}

async function settled(options) {
  const page = createPage(options);
  for (let index = 0; index < 4; index += 1) await flush();
  return page;
}

const payload = (temperature = 11.4, weatherCode = 61, observedAt = 1700000000) => ({
  current: {
    temperature_2m: temperature,
    weather_code: weatherCode,
    time: observedAt,
  },
});

test("the package reads its identity, settings, and readiness before one weather request", async () => {
  const page = await settled();
  assert.deepEqual(page.callsFor("identity"), [
    { command: "extension_bridge", args: { op: "identity" } },
  ]);
  assert.deepEqual(page.callsFor("settings.read"), [
    { command: "extension_bridge", args: { op: "settings.read" } },
  ]);
  assert.deepEqual(page.callsFor("surface.ready"), [
    { command: "extension_bridge", args: { op: "surface.ready" } },
  ]);
  assert.equal(page.node("identity").textContent, "com.example.weather 1.0.0 · API 2");
  assert.equal(page.node("surface").dataset.state, "loading");
  assert.equal(page.node("state").textContent, "Loading");
  assert.equal(page.node("place").textContent, "Seattle");
  assert.equal(page.node("refresh").attributes["aria-busy"], "true");
  assert.equal(page.netCalls().length, 1);
  assert.deepEqual(JSON.parse(JSON.stringify(page.netCalls()[0])), {
    command: "extension_bridge",
    args: {
      op: "net.get",
      args: {
        origin: "https://api.open-meteo.com",
        path: "/v1/forecast",
        query: [
          ["latitude", "47.6062"],
          ["longitude", "-122.3321"],
          ["current", "temperature_2m,weather_code"],
          ["temperature_unit", "celsius"],
          ["timeformat", "unixtime"],
        ],
      },
    },
  });
});

test("a reading renders only the package-controlled values", async () => {
  const page = await settled();
  page.resolveNet(payload());
  await flush();
  assert.equal(page.node("surface").dataset.state, "ready");
  assert.equal(page.node("state").textContent, "Ready");
  assert.equal(page.node("temperature").textContent, "11°C");
  assert.equal(page.node("condition").textContent, "Rain");
  assert.match(page.node("observed").textContent, /^Observed /);
  assert.match(page.node("status").textContent, /^Showing Seattle in Celsius\./);
  assert.equal(page.node("refresh").attributes["aria-busy"], "false");
  assert.equal(page.node("refresh").textContent, "Refresh");
  assert.equal(page.document.activeElement, page.node("refresh"));
});

test("the package settings choose the request units and display units", async () => {
  const page = await settled({
    settings: { values: { location: "tokyo", units: "f" } },
  });
  assert.equal(page.node("place").textContent, "Tokyo");
  assert.deepEqual(JSON.parse(JSON.stringify(page.netCalls()[0].args.args.query)), [
    ["latitude", "35.6762"],
    ["longitude", "139.6503"],
    ["current", "temperature_2m,weather_code"],
    ["temperature_unit", "fahrenheit"],
    ["timeformat", "unixtime"],
  ]);
  page.resolveNet(payload(46, 3));
  await flush();
  assert.equal(page.node("temperature").textContent, "46°F");
  assert.equal(page.node("condition").textContent, "Overcast");
});

test("a missing location shows an empty state without a request", async () => {
  const page = await settled({ settings: { values: { location: null, units: "c" } } });
  assert.equal(page.netCalls().length, 0);
  assert.equal(page.node("surface").dataset.state, "empty");
  assert.equal(page.node("state").textContent, "No location");
  assert.equal(page.node("place").textContent, "Location not set");
  assert.equal(page.node("status").textContent, "Choose a location in Luma Settings > Plugins, then refresh.");
});

test("a failed settings read blocks weather and explains the failure", async () => {
  const page = await settled({
    settingsFailure: { code: "denied", message: "This package has not declared the settings.read permission." },
  });
  assert.equal(page.netCalls().length, 0);
  assert.equal(page.node("surface").dataset.state, "error");
  assert.equal(page.node("status").textContent, "Could not read this package's settings: This package has not declared the settings.read permission.");
});

test("a failed refresh keeps the last reading and a retry recovers", async () => {
  const page = await settled();
  page.resolveNet(payload());
  await flush();
  page.click("refresh");
  assert.equal(page.netCalls().length, 2);
  assert.equal(page.node("refresh").textContent, "Refreshing…");
  page.rejectNet({ code: "unavailable", message: "The host could not reach that service." });
  await flush();
  assert.equal(page.node("surface").dataset.state, "stale");
  assert.equal(page.node("state").textContent, "Last reading");
  assert.equal(page.node("temperature").textContent, "11°C");
  assert.match(page.node("status").textContent, /^Couldn't refresh Seattle in Celsius: The host could not reach that service\.$/);

  page.click("refresh");
  page.resolveNet(payload(14, 3));
  await flush();
  assert.equal(page.node("surface").dataset.state, "ready");
  assert.equal(page.node("temperature").textContent, "14°C");
});

test("an initial failure is distinct and recovers through Refresh", async () => {
  const page = await settled();
  page.rejectNet({ code: "timeout", message: "The network request took too long." });
  await flush();
  assert.equal(page.node("surface").dataset.state, "error");
  assert.equal(page.node("temperature").textContent, "--");
  assert.equal(page.node("condition").textContent, "No reading available");
  assert.equal(page.node("status").textContent, "The network request took too long.");

  page.click("refresh");
  page.resolveNet(payload(4.6));
  await flush();
  assert.equal(page.node("surface").dataset.state, "ready");
  assert.equal(page.node("temperature").textContent, "5°C");
});

test("an unreadable provider response is not displayed as weather", async () => {
  const page = await settled();
  page.resolveNet({ current: { temperature_2m: "warm", weather_code: 3, time: 1700000000 } });
  await flush();
  assert.equal(page.node("surface").dataset.state, "error");
  assert.equal(page.node("temperature").textContent, "--");
  assert.equal(page.node("status").textContent, "The host returned an unreadable weather response.");
});

test("refresh never overlaps an in-flight request", async () => {
  const page = await settled();
  page.click("refresh");
  page.click("refresh");
  assert.equal(page.netCalls().length, 1);
  page.resolveNet(payload());
  await flush();
  page.click("refresh");
  assert.equal(page.netCalls().length, 2);
});

test("focus stays on controls and Escape or BrowserBack closes the host surface", async () => {
  const page = await settled();
  assert.equal(page.document.activeElement, page.node("refresh"));
  assert.equal(page.press("ArrowRight").defaultPrevented, true);
  assert.equal(page.document.activeElement, page.node("close"));
  assert.equal(page.press("ArrowLeft").defaultPrevented, true);
  assert.equal(page.document.activeElement, page.node("refresh"));

  assert.equal(page.press("Escape").defaultPrevented, true);
  assert.deepEqual(page.callsFor("surface.close"), [
    { command: "extension_bridge", args: { op: "surface.close" } },
  ]);
  assert.equal(page.press("BrowserBack").defaultPrevented, true);
  assert.equal(page.callsFor("surface.close").length, 2);
});

test("the package markup uses relative assets and a non-navigating attribution", () => {
  assert.match(markup, /<script src="\.\/page\.js"><\/script>/);
  assert.match(markup, /<link rel="stylesheet" href="\.\/page\.css" \/>/);
  assert.match(markup, /Weather data by Open-Meteo\.com/);
  assert.match(markup, /https:\/\/open-meteo\.com\//);
  assert.doesNotMatch(markup, /<a[\s>]/i);
  assert.doesNotMatch(markup, /<(?:script|img|iframe)[^>]+src="https?:/i);
});

test("the example manifest declares only the public runtime bridge it needs", () => {
  assert.equal(manifest.apiVersion, 2);
  assert.deepEqual(manifest.runtime, { kind: "isolated-ui", entry: "ui/index.html" });
  assert.deepEqual(manifest.permissions, ["settings.read", "network.get"]);
  assert.deepEqual(manifest.network, { allow: ["https://api.open-meteo.com"] });
  assert.deepEqual(manifest.assets, ["ui/page.js", "ui/page.css"]);
  assert.equal(manifest.settings[0].key, "location");
  assert.equal(manifest.settings[1].key, "units");
  assert.equal(source.includes("weather_read"), false);
  assert.equal(source.includes("extension_spike_action"), false);
  assert.equal(source.includes("fetch("), false);
});
