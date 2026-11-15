import type { AppEntry } from "./types";

export const RECENTS_KEY = "luma-remote-recents";

export interface StorageLike {
  getItem(key: string): string | null;
  setItem(key: string, value: string): void;
}

function browserStorage(): StorageLike | null {
  try {
    return globalThis.localStorage ?? null;
  } catch (_) {
    return null;
  }
}

export function readRecents(storage: StorageLike | null = browserStorage()): string[] {
  if (!storage) return [];
  try {
    const stored = JSON.parse(storage.getItem(RECENTS_KEY) || "[]");
    if (!Array.isArray(stored)) return [];
    return Array.from(new Set(stored.filter(value => typeof value === "string")));
  } catch (_) {
    return [];
  }
}

export function writeRecents(commands: string[], storage: StorageLike | null = browserStorage()): void {
  if (!storage) return;
  try {
    storage.setItem(RECENTS_KEY, JSON.stringify(commands));
  } catch (_) {}
}

export function rememberRecent(current: string[], command: string, limit = 4): string[] {
  return [command, ...current.filter(value => value !== command)].slice(0, limit);
}

export function recentApps(commands: string[], apps: AppEntry[]): AppEntry[] {
  return commands
    .map(command => apps.find(app => app.command === command))
    .filter((app): app is AppEntry => Boolean(app));
}
