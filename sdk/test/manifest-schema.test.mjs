import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import test from "node:test";

import Ajv2020 from "ajv/dist/2020.js";

const root = fileURLToPath(new URL("../../", import.meta.url));
const schema = JSON.parse(
  readFileSync(join(root, "schemas/luma-extension.schema.json"), "utf8"),
);
const ajv = new Ajv2020({ allErrors: true, strict: true });
const validate = ajv.compile(schema);

const manifests = [
  join(root, "examples/plugins/com.example.music-remote/luma-extension.json"),
  join(root, "examples/plugins/com.example.weather/luma-extension.json"),
  join(root, "examples/plugins/com.example.weather-svelte/luma-extension.json"),
  join(root, "starters/typescript/luma-extension.json"),
  join(root, "starters/svelte/luma-extension.json"),
  join(root, "examples/plugins/com.luma.apple-tv-look/luma-extension.json"),
];

function read(path) {
  return JSON.parse(readFileSync(path, "utf8"));
}

function errors(path) {
  return (validate.errors ?? [])
    .map(error => `${path}${error.instancePath || "/"}: ${error.message}`)
    .join("\n");
}

test("the editor schema accepts every shipped manifest fixture", () => {
  for (const path of manifests) {
    assert.equal(validate(read(path)), true, errors(path));
  }
});

test("the editor schema accepts API version 3 Home command packages", () => {
  const manifest = {
    id: "com.example.weather-home",
    name: "Weather Home",
    version: "1.0.0",
    apiVersion: 3,
    minHostVersion: "0.8.0",
    runtime: { kind: "isolated-ui", entry: "ui/index.html" },
    permissions: ["settings.read"],
    contributes: {
      commands: ["openForecast"],
      cards: [{ id: "forecast", title: "Weather", command: "openForecast" }],
    },
  };

  assert.equal(validate(manifest), true, errors("v3 manifest"));

  const remote = {
    ...manifest,
    remote: {
      title: "Weather controls",
      commands: [{ command: "openForecast", label: "Refresh forecast" }],
    },
  };
  assert.equal(validate(remote), true, errors("v3 remote manifest"));
});

test("the editor schema rejects native manifest violations", () => {
  const fixture = read(manifests[1]);
  const declarative = read(manifests[0]);
  const v3 = {
    id: "com.example.weather-home",
    name: "Weather Home",
    version: "1.0.0",
    apiVersion: 3,
    minHostVersion: "0.8.0",
    runtime: { kind: "isolated-ui", entry: "ui/index.html" },
    permissions: ["settings.read"],
    contributes: {
      commands: ["openForecast"],
      cards: [{ id: "forecast", title: "Weather", command: "openForecast" }],
    },
  };

  const cases = [
    ["unknown field", { ...fixture, unexpected: true }],
    ["reserved id", { ...fixture, id: "core.weather" }],
    ["uppercase id", { ...fixture, id: "com.example.Weather" }],
    ["leading-zero version", { ...fixture, version: "01.0.0" }],
    ["api v1 with permissions", { ...fixture, apiVersion: 1 }],
    [
      "executable remote panel",
      {
        ...fixture,
        apiVersion: 2,
        contributes: {},
        remote: { title: "Controls", actions: [] },
      },
    ],
    ["executable contribution", { ...fixture, contributes: { theme: "ember" } }],
    ["host version on api v1", { ...declarative, minHostVersion: "0.8.0" }],
    [
      "api v1 with remote commands",
      {
        ...declarative,
        remote: { commands: [{ command: "openForecast", label: "Refresh" }] },
      },
    ],
    [
      "home cards on api v1",
      {
        ...declarative,
        contributes: {
          cards: [{ id: "forecast", title: "Weather", command: "openForecast" }],
        },
      },
    ],
    ["api v3 without host version", { ...v3, minHostVersion: undefined }],
    ["api v3 without cards", { ...v3, contributes: { commands: ["openForecast"] } }],
    [
      "api v3 with appearance",
      {
        ...v3,
        contributes: { ...v3.contributes, theme: "ember" },
      },
    ],
    ["api v3 with remote playback action", { ...v3, remote: { title: "Controls", actions: ["playback.toggle"] } }],
    ["api v3 with empty remote commands", { ...v3, remote: { title: "Controls", commands: [] } }],
    [
      "api v3 with long remote label",
      {
        ...v3,
        remote: {
          commands: [
            { command: "openForecast", label: "x".repeat(33) },
          ],
        },
      },
    ],
    [
      "api v3 with background runtime",
      { ...v3, runtime: { kind: "background", entry: "ui/index.html" } },
    ],
    [
      "network without permission",
      { ...fixture, permissions: ["settings.read"] },
    ],
    [
      "network.get without allowlist",
      { ...fixture, permissions: ["settings.read", "network.get"], network: undefined },
    ],
    [
      "private network origin",
      {
        ...fixture,
        network: { allow: ["https://localhost"] },
      },
    ],
    [
      "string default for boolean setting",
      {
        ...fixture,
        settings: [{ key: "flag", label: "Flag", type: "boolean", default: "true" }],
      },
    ],
    [
      "enum without options",
      {
        ...fixture,
        settings: [{ key: "mode", label: "Mode", type: "enum", default: "a" }],
      },
    ],
    ["parent path", { ...fixture, assets: ["../ui/page.js"] }],
    ["absolute path", { ...fixture, assets: ["/ui/page.js"] }],
    ["backslash path", { ...fixture, assets: ["ui\\page.js"] }],
  ];

  for (const [name, manifest] of cases) {
    assert.equal(validate(manifest), false, `${name} should be rejected`);
  }
});
