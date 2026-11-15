import { api } from "./api";
import { readRecents, recentApps, rememberRecent, writeRecents } from "./recents";
import type { AppEntry } from "./types";

export const apps = $state({
  list: [] as AppEntry[],
  shortcuts: [] as AppEntry[],
  recents: readRecents(),
  loading: false,
  attempted: false,
  error: false,
});

let loaded = false;
let lastOrder = "";

export async function loadApps(force = false): Promise<void> {
  if (apps.loading || (loaded && !force)) return;
  apps.loading = true;
  apps.attempted = true;
  apps.error = false;
  try {
    const { response, data } = await api.apps();
    if (!response.ok || !Array.isArray(data?.apps)) {
      apps.error = true;
      return;
    }
    apps.shortcuts = data.apps.filter(app => app && app.shortcut === true && typeof app.command === "string" && typeof app.name === "string");
    apps.list = data.apps.filter(
      app => app && !app.shortcut && typeof app.command === "string" && typeof app.name === "string",
    );
    loaded = apps.list.length > 0;
  } catch (_) {
    apps.error = true;
  } finally {
    apps.loading = false;
  }
}

export function syncAppOrder(order: string[]): void {
  if (!Array.isArray(order)) return;
  const signature = JSON.stringify(order);
  if (signature === lastOrder) return;
  lastOrder = signature;
  void loadApps(true);
}

export function rememberApp(command: string): void {
  apps.recents = rememberRecent(apps.recents, command);
  writeRecents(apps.recents);
}

export function recentAppsForSession(): AppEntry[] {
  return recentApps(apps.recents, apps.list);
}
