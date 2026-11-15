const ENABLED_KEY = "luma-music-enabled";
const COLLAPSED_KEY = "luma-music-collapsed";

function read(key: string, fallback: boolean): boolean {
  try {
    const value = localStorage.getItem(key);
    return value === "yes" ? true : value === "no" ? false : fallback;
  } catch (_) {
    return fallback;
  }
}

function write(key: string, value: boolean): void {
  try {
    localStorage.setItem(key, value ? "yes" : "no");
  } catch (_) {}
}

export const music = $state({
  enabled: read(ENABLED_KEY, true),
  collapsed: read(COLLAPSED_KEY, false),
});

export function setMusicEnabled(value: boolean): void {
  music.enabled = value;
  write(ENABLED_KEY, value);
}

export function setMusicCollapsed(value: boolean): void {
  music.collapsed = value;
  write(COLLAPSED_KEY, value);
}
