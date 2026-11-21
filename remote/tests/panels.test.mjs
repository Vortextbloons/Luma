import test from "node:test";
import assert from "node:assert/strict";
import { normalizePanels, panelActionEnabled, panelActionLabel } from "../src/lib/panels.ts";

const panel = {
  id: "com.example.lounge",
  name: "Lounge",
  title: "Lounge controls",
  actions: ["playback.toggle", "playback.next"],
  commands: [],
};

test("plugin panels keep only the bounded declared shape", () => {
  const panels = normalizePanels([
    panel,
    { ...panel },
    { ...panel, id: "com.example.next", actions: ["playback.previous", "playback.previous", "run.native"] },
    { ...panel, id: "com.example.bad", title: "" },
    null,
  ]);
  assert.deepEqual(panels, [
    panel,
    { id: "com.example.next", name: "Lounge", title: "Lounge controls", actions: ["playback.previous"], commands: [] },
  ]);
});

test("command panels keep bounded host-rendered labels only", () => {
  const panels = normalizePanels([
    {
      id: "com.example.weather",
      name: "Weather",
      title: "Weather controls",
      actions: [],
      version: "1.0.0",
      commands: [
        { id: "openForecast", label: " Refresh forecast " },
        { id: "openForecast", label: "Duplicate" },
        { id: "refresh", label: "" },
        { id: "second", label: "Second", html: "<script>" },
        { id: "third", label: "Third" },
        { id: "fourth", label: "Fourth" },
      ],
    },
    { id: "com.example.legacy", name: "Legacy", title: "Legacy controls", actions: [] },
  ]);
  assert.deepEqual(panels, [
    {
      id: "com.example.weather",
      name: "Weather",
      title: "Weather controls",
      actions: [],
      version: "1.0.0",
      commands: [
        { id: "openForecast", label: "Refresh forecast" },
        { id: "second", label: "Second" },
        { id: "third", label: "Third" },
      ],
    },
    {
      id: "com.example.legacy",
      name: "Legacy",
      title: "Legacy controls",
      actions: [],
      commands: [],
    },
  ]);
});

test("plugin panel actions follow the PC playback capabilities", () => {
  const music = {
    playing: true,
    hasArtwork: false,
    updatedAt: 0,
    canToggle: true,
    canSkipNext: false,
    canSkipPrevious: true,
  };
  assert.equal(panelActionEnabled("playback.toggle", music), true);
  assert.equal(panelActionEnabled("playback.next", music), false);
  assert.equal(panelActionEnabled("playback.previous", music), true);
  assert.equal(panelActionEnabled("playback.toggle", null), false);
});

test("plugin playback labels match the current state", () => {
  assert.equal(panelActionLabel("playback.toggle", { playing: true }), "Pause music");
  assert.equal(panelActionLabel("playback.toggle", { playing: false }), "Play music");
  assert.equal(panelActionLabel("playback.next", null), "Next track");
  assert.equal(panelActionLabel("playback.previous", null), "Previous track");
});
