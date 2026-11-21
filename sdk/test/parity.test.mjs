import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import test from "node:test";

import {
  BRIDGE_ERROR_CODES,
  BRIDGE_OPERATIONS,
  COMMAND_EVENT,
  PERMISSIONS,
  PLAYBACK_EVENT,
  SETTINGS_EVENT,
} from "../index.js";

const root = fileURLToPath(new URL("../../", import.meta.url));

function read(relative) {
  return readFileSync(join(root, relative), "utf8");
}

function block(source, start) {
  const from = source.indexOf(start);
  assert.notEqual(from, -1, `missing ${start}`);
  const to = source.indexOf("\n}", from);
  assert.notEqual(to, -1, `unterminated ${start}`);
  return source.slice(from, to);
}

function serdeRenames(source) {
  return [...source.matchAll(/#\[serde\(rename = "([^"]+)"\)\]/g)].map(match => match[1]);
}

function rustArray(source, name) {
  const match = source.match(new RegExp(`const ${name}: \\[&str; \\d+\\] = \\[([\\s\\S]*?)\\];`));
  assert.ok(match, `missing ${name}`);
  return [...match[1].matchAll(/"([^"]+)"/g)].map(item => item[1]);
}

function rustLimit(source, name) {
  const match = source.match(new RegExp(`const ${name}: (?:u64|usize) = ([0-9_]+);`));
  assert.ok(match, `missing ${name}`);
  return Number(match[1].replaceAll("_", ""));
}

test("SDK permissions match the Rust manifest contract", () => {
  const plugin = read("src-tauri/src/customization/plugin.rs");
  const permission = block(plugin, "pub enum Permission {");
  assert.deepEqual(PERMISSIONS, serdeRenames(permission));
});

test("SDK bridge operations match the Rust bridge contract", () => {
  const spike = read("src-tauri/src/extension_spike.rs");
  const operations = block(spike, "enum Op {");
  assert.deepEqual(BRIDGE_OPERATIONS, serdeRenames(operations));
});

test("SDK event names match the Rust host notifications", () => {
  const spike = read("src-tauri/src/extension_spike.rs");
  const constant = name => {
    const match = spike.match(new RegExp(`const ${name}: &str = "([^"]+)";`));
    assert.ok(match, `missing ${name}`);
    return match[1];
  };
  assert.equal(COMMAND_EVENT, "luma:command");
  assert.equal(SETTINGS_EVENT, constant("SETTINGS_EVENT"));
  assert.equal(PLAYBACK_EVENT, constant("PLAYBACK_EVENT"));
});

test("SDK error codes cover every Rust bridge failure", () => {
  const spike = read("src-tauri/src/extension_spike.rs");
  const net = read("src-tauri/src/extension_spike/net.rs");
  const codes = new Set([
    ...[...spike.matchAll(/failure\("([a-z_]+)"/g)].map(match => match[1]),
    ...[...net.matchAll(/code: "([a-z_]+)"/g)].map(match => match[1]),
  ]);
  for (const code of codes) {
    assert.ok(BRIDGE_ERROR_CODES.includes(code), `${code} is missing from the SDK`);
  }
});

test("the editor schema uses the Rust manifest limits and enumerations", () => {
  const plugin = read("src-tauri/src/customization/plugin.rs");
  const schema = JSON.parse(read("schemas/luma-extension.schema.json"));
  const remoteActions = serdeRenames(block(plugin, "pub enum RemoteAction {"));

  assert.equal(schema.properties.settings.maxItems, rustLimit(plugin, "MAX_SETTINGS"));
  assert.equal(
    schema.$defs.setting.properties.options.maxItems,
    rustLimit(plugin, "MAX_SETTING_OPTIONS"),
  );
  assert.equal(
    schema.properties.assets.maxItems,
    rustLimit(plugin, "MAX_SOURCE_FILES"),
  );
  assert.equal(
    schema.properties.source.maxItems,
    rustLimit(plugin, "MAX_SOURCE_FILES"),
  );
  assert.equal(
    schema.properties.permissions.maxItems,
    rustLimit(plugin, "MAX_PERMISSIONS"),
  );
  assert.equal(
    schema.$defs.network.properties.allow.maxItems,
    rustLimit(plugin, "MAX_NETWORK_ORIGINS"),
  );
  assert.equal(
    schema.$defs.network.properties.allow.items.maxLength,
    rustLimit(plugin, "MAX_ORIGIN_BYTES"),
  );
  assert.equal(schema.$defs.path.maxLength, rustLimit(plugin, "MAX_PATH_BYTES"));
  assert.equal(
    schema.$defs.remote.properties.actions.maxItems,
    rustLimit(plugin, "MAX_REMOTE_ACTIONS"),
  );
  assert.equal(
    schema.$defs.remote.properties.title.maxLength,
    rustLimit(plugin, "MAX_REMOTE_TITLE"),
  );
  assert.equal(
    schema.$defs.remote.properties.commands.maxItems,
    rustLimit(plugin, "MAX_REMOTE_COMMANDS"),
  );
  assert.equal(
    schema.$defs.remoteCommand.properties.label.maxLength,
    rustLimit(plugin, "MAX_REMOTE_LABEL"),
  );
  assert.equal(
    schema.$defs.contributions.properties.cards.maxItems,
    rustLimit(plugin, "MAX_HOME_CARDS"),
  );
  assert.equal(
    schema.$defs.contributions.properties.commands.maxItems,
    rustLimit(plugin, "MAX_COMMANDS"),
  );
  assert.equal(
    schema.$defs.homeCard.properties.title.maxLength,
    rustLimit(plugin, "MAX_CARD_TITLE"),
  );
  assert.equal(schema.$defs.localId.maxLength, rustLimit(plugin, "MAX_LOCAL_ID"));

  assert.deepEqual(schema.$defs.themeBound.oneOf[0].enum, rustArray(plugin, "BUILTIN_THEMES"));
  assert.deepEqual(schema.$defs.densityBound.oneOf[0].enum, rustArray(plugin, "DENSITIES"));
  assert.deepEqual(schema.$defs.tileShapeBound.oneOf[0].enum, rustArray(plugin, "TILE_SHAPES"));
  assert.deepEqual(schema.$defs.textSizeBound.oneOf[0].enum, rustArray(plugin, "TEXT_SIZES"));
  assert.deepEqual(schema.$defs.homeComponent.enum, rustArray(plugin, "COMPONENTS"));
  assert.deepEqual(
    schema.$defs.widgets.properties.spotifyPlacement.oneOf[0].enum,
    rustArray(plugin, "PLACEMENTS"),
  );
  assert.deepEqual(schema.$defs.remote.properties.actions.items.enum, remoteActions);
});
