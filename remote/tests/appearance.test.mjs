import test from "node:test";
import assert from "node:assert/strict";
import { resolveAppearance, themeColors, themeLabel, validAppearance } from "../src/lib/appearance.ts";

const appearance = {
  theme: "violet",
  tokens: { "--bg": "#080a10", "--accent": "#a79bff" },
  themes: [
    { id: "violet", name: "Violet Night", tokens: { "--bg": "#080a10", "--accent": "#a79bff" } },
    { id: "mint", name: "Mint", tokens: { "--bg": "#06120c", "--accent": "#7fe0ae" } },
  ],
};

test("a known theme overrides the shared tokens", () => {
  const resolved = resolveAppearance(appearance, "mint");
  assert.equal(resolved.choice, "mint");
  assert.equal(resolved.tokens["--bg"], "#06120c");
});

test("following or losing a theme falls back to the shared appearance", () => {
  assert.equal(resolveAppearance(appearance, "follow").choice, "follow");
  assert.deepEqual(resolveAppearance(appearance, "missing"), { tokens: appearance.tokens, choice: "follow" });
});

test("theme labels and swatches stay usable with partial token sets", () => {
  assert.equal(themeLabel(appearance, "mint"), "Mint");
  assert.equal(themeLabel(appearance, "missing"), "Follow TV");
  assert.deepEqual(themeColors(appearance.themes[1]), { background: "#06120c", accent: "#7fe0ae" });
  assert.deepEqual(themeColors({ id: "bad", name: "Bad", tokens: {} }), {
    background: "var(--surface)",
    accent: "var(--accent)",
  });
});

test("appearance payloads are checked before use", () => {
  assert.equal(validAppearance(appearance), true);
  assert.equal(validAppearance({ theme: "x", tokens: null, themes: [] }), false);
  assert.equal(validAppearance(null), false);
});
