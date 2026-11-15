import type { Appearance, ThemeOption } from "./types";

export const APPEARANCE_CACHE_KEY = "luma-remote-appearance";
export const THEME_CHOICE_KEY = "luma-remote-theme";

export const APPEARANCE_TOKENS = new Set([
  "--bg",
  "--bg-soft",
  "--ambient",
  "--surface",
  "--surface-2",
  "--surface-3",
  "--line",
  "--line-strong",
  "--text",
  "--text-dim",
  "--muted",
  "--dim",
  "--accent",
  "--accent-strong",
  "--accent-ink",
  "--accent-soft",
  "--good",
  "--warn",
  "--danger",
  "--rating",
]);

export const HEX = /^#[0-9a-f]{6}(?:[0-9a-f]{2})?$/i;

interface StorageLike {
  getItem(key: string): string | null;
  setItem(key: string, value: string): void;
}

function storage(): StorageLike | null {
  try {
    return globalThis.localStorage ?? null;
  } catch (_) {
    return null;
  }
}

export function readThemeChoice(store: StorageLike | null = storage()): string {
  try {
    return store?.getItem(THEME_CHOICE_KEY) || "follow";
  } catch (_) {
    return "follow";
  }
}

export function saveThemeChoice(choice: string, store: StorageLike | null = storage()): void {
  try {
    store?.setItem(THEME_CHOICE_KEY, choice);
  } catch (_) {}
}

export function validAppearance(value: unknown): value is Appearance {
  if (!value || typeof value !== "object" || Array.isArray(value)) return false;
  const appearance = value as Appearance;
  return typeof appearance.theme === "string"
    && Array.isArray(appearance.themes)
    && Boolean(appearance.tokens)
    && typeof appearance.tokens === "object"
    && !Array.isArray(appearance.tokens);
}

export function resolveAppearance(
  appearance: Appearance,
  choice: string,
): { tokens: Record<string, string>; choice: string } {
  if (choice !== "follow") {
    const theme = appearance.themes.find(item => item && item.id === choice);
    if (theme && theme.tokens && typeof theme.tokens === "object") {
      return { tokens: theme.tokens, choice };
    }
  }
  return { tokens: appearance.tokens, choice: "follow" };
}

export function themeLabel(appearance: Appearance | null | undefined, choice: string): string {
  if (choice === "follow" || !appearance) return "Follow TV";
  return appearance.themes.find(theme => theme && theme.id === choice)?.name || "Follow TV";
}

export function themeColors(theme: ThemeOption): { background: string; accent: string } {
  const background = theme.tokens?.["--bg"];
  const accent = theme.tokens?.["--accent"];
  return {
    background: typeof background === "string" && HEX.test(background) ? background : "var(--surface)",
    accent: typeof accent === "string" && HEX.test(accent) ? accent : "var(--accent)",
  };
}

export function paintTokens(tokens: Record<string, string> | null | undefined): void {
  if (!tokens || typeof tokens !== "object" || Array.isArray(tokens)) return;
  if (typeof document === "undefined") return;
  const root = document.documentElement;
  for (const [name, value] of Object.entries(tokens)) {
    if (!APPEARANCE_TOKENS.has(name) || typeof value !== "string" || !HEX.test(value)) continue;
    root.style.setProperty(name, value);
  }
  const meta = document.querySelector('meta[name="theme-color"]');
  const background = tokens["--bg"];
  if (meta && typeof background === "string" && HEX.test(background)) {
    meta.setAttribute("content", background);
  }
}

let appliedSignature = "";

export function applyAppearance(appearance: Appearance, choice: string): string {
  if (!validAppearance(appearance)) return "follow";
  const resolved = resolveAppearance(appearance, choice);
  const signature = `${JSON.stringify(appearance)}\u0000${resolved.choice}`;
  if (signature !== appliedSignature) {
    appliedSignature = signature;
    paintTokens(resolved.tokens);
    try {
      localStorage.setItem(APPEARANCE_CACHE_KEY, JSON.stringify(appearance));
    } catch (_) {}
  }
  if (resolved.choice !== choice) saveThemeChoice("follow");
  return resolved.choice;
}
