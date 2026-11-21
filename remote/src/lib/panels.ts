import type { MusicState, PanelAction, PanelCommand, RemotePanel } from "./types";

const PANEL_ACTIONS: PanelAction[] = [
  "playback.toggle",
  "playback.next",
  "playback.previous",
];

export function normalizePanels(value: unknown): RemotePanel[] {
  if (!Array.isArray(value)) return [];
  const seen = new Set<string>();
  const panels: RemotePanel[] = [];
  for (const entry of value) {
    if (!entry || typeof entry !== "object") continue;
    const record = entry as Record<string, unknown>;
    const { id, name, title, actions, commands, version } = record;
    if (typeof id !== "string" || !id || seen.has(id)) continue;
    if (typeof name !== "string" || !name) continue;
    if (typeof title !== "string" || !title) continue;
    if (!Array.isArray(actions)) continue;
    const declared = new Set<PanelAction>();
    for (const action of actions) {
      if (PANEL_ACTIONS.includes(action as PanelAction)) declared.add(action as PanelAction);
    }
    seen.add(id);
    panels.push({
      id,
      name,
      title,
      actions: [...declared].slice(0, 3),
      ...(typeof version === "string" && version
        ? { version, commands: normalizeCommands(commands) }
        : { commands: [] }),
    });
  }
  return panels;
}

function normalizeCommands(value: unknown): PanelCommand[] {
  if (!Array.isArray(value)) return [];
  const seen = new Set<string>();
  const commands: PanelCommand[] = [];
  for (const entry of value) {
    if (!entry || typeof entry !== "object") continue;
    const { id, label } = entry as Record<string, unknown>;
    if (typeof id !== "string" || !id || seen.has(id)) continue;
    if (typeof label !== "string" || !label.trim()) continue;
    seen.add(id);
    commands.push({ id, label: label.trim() });
  }
  return commands.slice(0, 3);
}

export function panelActionLabel(action: PanelAction, music: MusicState | null): string {
  if (action === "playback.next") return "Next track";
  if (action === "playback.previous") return "Previous track";
  return music?.playing ? "Pause music" : "Play music";
}

export function panelActionEnabled(action: PanelAction, music: MusicState | null): boolean {
  if (!music) return false;
  if (action === "playback.next") return music.canSkipNext;
  if (action === "playback.previous") return music.canSkipPrevious;
  return music.canToggle;
}
