export type ScreenName = "remote" | "apps" | "search";

const SCREEN_KEY = "lumaRemoteScreen";

export const screen = $state({
  current: "remote" as ScreenName,
  restoreAppsFocus: false,
  restoreSearchFocus: false,
});

function currentState(): Record<string, unknown> {
  const value = history.state;
  return value && typeof value === "object" ? (value as Record<string, unknown>) : {};
}

export function syncScreen(): void {
  const previous = screen.current;
  const current = currentState()[SCREEN_KEY];
  screen.current = current === "apps" || current === "search" ? current : "remote";
  if (previous === "apps" && screen.current === "remote") screen.restoreAppsFocus = true;
  if (previous === "search" && screen.current === "remote") screen.restoreSearchFocus = true;
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
  screen.restoreSearchFocus = false;
  screen.current = "remote";
  screen.restoreAppsFocus = false;
  const state = currentState();
  if (state[SCREEN_KEY] !== "remote") {
    try {
      history.replaceState({ ...state, [SCREEN_KEY]: "remote" }, "");
    } catch (_) {}
  }
}

export function openSearch(): void {
  if (screen.current === "search") return;
  screen.restoreSearchFocus = false;
  try { history.pushState({ ...currentState(), [SCREEN_KEY]: "search" }, ""); } catch (_) {}
  screen.current = "search";
}

export function closeSearch(): void {
  if (screen.current !== "search") return;
  screen.restoreSearchFocus = true;
  if (currentState()[SCREEN_KEY] === "search") {
    try { history.back(); } catch (_) {}
  }
  screen.current = "remote";
}
