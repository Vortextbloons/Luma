import type { AppId, Settings, WidgetPlacement } from '../types';
import { defaultHomeRows, legacyHomeRows, sanitizeHomeRows } from '../services/home';
import { resolveThemeId } from '../services/theme';

const executable = ['Steam', 'Spotify', 'Stremio', 'Netflix', 'Disney+', 'Prime Video', 'Apple TV+', 'Hulu', 'Max', 'Peacock', 'Custom'] as const;
const placements: WidgetPlacement[] = ['bar', 'top-right', 'bottom-right', 'bottom-left'];
const appOrder: AppId[] = ['YouTube', 'Steam', 'Spotify', 'Stremio', 'Netflix', 'Disney+', 'Prime Video', 'Apple TV+', 'Hulu', 'Max', 'Peacock', 'Custom'];

function defaultSettings(): Settings {
  return {
    theme: 'violet-night',
    density: 'comfortable',
    appTileShape: 'wide',
    homeShortcut: 'Ctrl+W',
    fullscreen: false,
    interfaceSounds: 'real',
    soundLevel: 'standard',
    home: defaultHomeRows(),
    widgets: { spotify: { enabled: true, placement: 'bar' } },
    paths: { Steam: '', Spotify: '', Stremio: '', Netflix: '', 'Disney+': '', 'Prime Video': '', 'Apple TV+': '', Hulu: '', Max: '', Peacock: '', Custom: '' },
    order: [...appOrder],
  };
}

export function createSettings() {
  let value = defaultSettings();
  let error = '';
  try {
    const saved: unknown = JSON.parse(localStorage.getItem('luma-v1') ?? 'null');
    if (saved && typeof saved === 'object') {
      const data = saved as Partial<Settings> & { shortcutVersion?: number; accent?: unknown; sections?: unknown };
      value.theme = resolveThemeId(data.theme, data.accent);
      if (data.density === 'compact') value.density = data.density;
      if (data.appTileShape === 'square') value.appTileShape = 'square';
      if (typeof data.homeShortcut === 'string' && /^(?:Ctrl\+(?:Alt\+)?|Alt\+)(?:Shift\+)?(?:[A-Z0-9]|Home|F(?:[1-9]|1[0-2]))$/.test(data.homeShortcut) && !data.homeShortcut.endsWith('+F11')) {
        value.homeShortcut = data.homeShortcut === 'Ctrl+H' && data.shortcutVersion !== 1 ? 'Ctrl+W' : data.homeShortcut;
      }
      value.fullscreen = data.fullscreen === true;
      if (data.interfaceSounds === 'google' || data.interfaceSounds === 'roku' || data.interfaceSounds === 'off') value.interfaceSounds = data.interfaceSounds;
      if (data.soundLevel === 'soft' || data.soundLevel === 'loud') value.soundLevel = data.soundLevel;
      value.home = sanitizeHomeRows(data.home) ?? legacyHomeRows(data.sections);
      for (const id of executable) if (typeof data.paths?.[id] === 'string') value.paths[id] = data.paths[id];
      if (Array.isArray(data.order)) {
        const order = data.order.filter((id): id is AppId => appOrder.includes(id) && typeof id === 'string');
        value.order = [...new Set([...order, ...appOrder.filter(id => !order.includes(id))])].filter(id => id !== 'Custom');
        value.order.push('Custom');
      }
      const spotify = data.widgets?.spotify;
      if (spotify && typeof spotify === 'object') {
        if (typeof spotify.enabled === 'boolean') value.widgets.spotify.enabled = spotify.enabled;
        if (placements.includes(spotify.placement)) value.widgets.spotify.placement = spotify.placement;
      }
    }
  } catch { error = 'Saved preferences could not be read.'; }
  const state = $state({ value, error });
  return state;
}

export function moveApp(settings: Settings, id: AppId, direction: -1 | 1) {
  const index = settings.order.indexOf(id);
  const target = index + direction;
  if (id === 'Custom' || target < 0 || target >= settings.order.length || settings.order[target] === 'Custom') return;
  [settings.order[index], settings.order[target]] = [settings.order[target], settings.order[index]];
}
