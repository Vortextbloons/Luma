import test from "node:test";
import assert from "node:assert/strict";
import { readRecents, recentApps, rememberRecent, writeRecents } from "../src/lib/recents.ts";

function storage(initial = null) {
  let value = initial;
  return {
    getItem: () => value,
    setItem: (_key, next) => {
      value = next;
    },
    value: () => value,
  };
}

const apps = [
  { command: "youtube", name: "YouTube", icon: "/apps/youtube/icon" },
  { command: "netflix", name: "Netflix", icon: "/apps/netflix/icon" },
  { command: "spotify", name: "Spotify", icon: "/apps/spotify/icon" },
];

test("recents keep the newest command first without duplicates", () => {
  assert.deepEqual(rememberRecent(["netflix", "youtube"], "youtube"), ["youtube", "netflix"]);
  assert.deepEqual(rememberRecent(["a", "b", "c", "d"], "e"), ["e", "a", "b", "c"]);
});

test("recents are read and written through the shared key", () => {
  const store = storage();
  writeRecents(["youtube", "netflix"], store);
  assert.deepEqual(readRecents(store), ["youtube", "netflix"]);
});

test("recents ignore unknown or malformed entries", () => {
  assert.deepEqual(readRecents(storage("not json")), []);
  assert.deepEqual(readRecents(storage(JSON.stringify(["youtube", 3, "netflix", "youtube"]))), ["youtube", "netflix"]);
  assert.deepEqual(recentApps(["missing", "netflix"], apps).map(app => app.command), ["netflix"]);
});
