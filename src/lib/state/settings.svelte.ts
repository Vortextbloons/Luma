import type { AppId, Settings, WidgetPlacement } from '../types';
import { defaultHomeRows, legacyHomeRows, sanitizeHomeRows } from '../services/home';
import { resolveThemeId } from '../services/theme';

const executable = ['Steam', 'Spotify', 'Stremio', 'Netflix', 'Disney+', 'Prime Video', 'Apple TV+', 'Hulu', 'Max', 'Peacock', 'Custom'] as const;
const placements: WidgetPlacement[] = ['bar', 'top-right', 'bottom-right', 'bottom-left'];
const appOrder: AppId[] = ['YouTube', 'Spotify', 'Stremio', 'Steam', 'Netflix', 'Disney+', 'Prime Video', 'Apple TV+', 'Hulu', 'Max', 'Peacock', 'NBA', 'NFL', 'MLB', 'NHL', 'Custom'];
const legacyOrders: AppId[][] = [
  ['YouTube', 'Spotify', 'Stremio', 'Netflix', 'Disney+', 'Prime Video', 'Apple TV+', 'Hulu', 'Max', 'Peacock', 'Custom'],
  ['YouTube', 'Spotify', 'Steam', 'Stremio', 'Netflix', 'Disney+', 'Prime Video', 'Apple TV+', 'Hulu', 'Max', 'Peacock', 'Custom'],
];

function defaultSettings(): Settings {
  return {
    theme: 'violet-night',
    density: 'comfortable',
    appTileShape: 'wide',
    homeShortcut: 'Ctrl+W',
    fullscreen: false,
    fastAppSwitch: true,
    interfaceSounds: 'real',
    soundLevel: 'standard',
    textSize: 'standard',
    reducedMotion: false,
    home: defaultHomeRows(),
    homeColumns: 1,
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
      if (typeof data.fastAppSwitch === 'boolean') value.fastAppSwitch = data.fastAppSwitch;
      if (data.interfaceSounds === 'google' || data.interfaceSounds === 'roku' || data.interfaceSounds === 'off') value.interfaceSounds = data.interfaceSounds;
      if (data.soundLevel === 'soft' || data.soundLevel === 'loud') value.soundLevel = data.soundLevel;
      if (data.textSize === 'large' || data.textSize === 'larger') value.textSize = data.textSize;
      value.reducedMotion = data.reducedMotion === true;
      value.home = sanitizeHomeRows(data.home) ?? legacyHomeRows(data.sections);
      if (data.homeColumns === 2) value.homeColumns = 2;
      for (const id of executable) if (typeof data.paths?.[id] === 'string') value.paths[id] = data.paths[id];
      if (Array.isArray(data.order)) value.order = normalizeAppOrder(data.order, true);
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

export function defaultAppOrder(): AppId[] {
  return [...appOrder];
}

export function normalizeAppOrder(value: unknown, migrateLegacy = false): AppId[] {
  const source = Array.isArray(value) ? value : [];
  const known = source.filter((id): id is AppId => typeof id === 'string' && appOrder.includes(id as AppId) && id !== 'Custom');
  const order = [...new Set(known)];
  for (const id of appOrder) if (id !== 'Custom' && !order.includes(id)) order.push(id);
  order.push('Custom');
  return migrateLegacy && legacyOrders.some(legacy => order.join() === legacy.join()) ? [...appOrder] : order;
}

export function moveAppTo(settings: Settings, id: AppId, index: number) {
  if (id === 'Custom') return;
  const order = settings.order.filter(app => app !== 'Custom');
  const from = order.indexOf(id);
  if (from < 0) return;
  order.splice(from, 1);
  order.splice(Math.max(0, Math.min(index, order.length)), 0, id);
  settings.order = [...order, 'Custom'];
}
