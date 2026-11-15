export type ScreenName = "remote" | "apps";

const SCREEN_KEY = "lumaRemoteScreen";

export const screen = $state({
  current: "remote" as ScreenName,
  restoreAppsFocus: false,
});

function currentState(): Record<string, unknown> {
  const value = history.state;
  return value && typeof value === "object" ? (value as Record<string, unknown>) : {};
}

export function syncScreen(): void {
  const previous = screen.current;
  screen.current = currentState()[SCREEN_KEY] === "apps" ? "apps" : "remote";
  if (previous === "apps" && screen.current === "remote") screen.restoreAppsFocus = true;
}

export function initScreen(): () => void {
  const state = currentState();
  if (state[SCREEN_KEY] !== "remote") {
    try {
      history.replaceState({ ...state, [SCREEN_KEY]: "remote" }, "");
    } catch (_) {}
  }
  window.addEventListener("popstate", syncScreen);
  return () => window.removeEventListener("popstate", syncScreen);
}

export function openApps(): void {
  if (screen.current === "apps") return;
  screen.restoreAppsFocus = false;
  try {
    history.pushState({ ...currentState(), [SCREEN_KEY]: "apps" }, "");
  } catch (_) {}
  screen.current = "apps";
}

export function closeApps(): void {
  if (screen.current !== "apps") return;
  screen.restoreAppsFocus = true;
  if (currentState()[SCREEN_KEY] === "apps") {
    try {
      history.back();
    } catch (_) {}
  }
  screen.current = "remote";
}

export function resetScreen(): void {
  screen.current = "remote";
  screen.restoreAppsFocus = false;
  const state = currentState();
  if (state[SCREEN_KEY] !== "remote") {
    try {
      history.replaceState({ ...state, [SCREEN_KEY]: "remote" }, "");
    } catch (_) {}
  }
}
