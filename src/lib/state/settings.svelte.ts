import type { AppId, Settings } from '../types';

const executable = ['Spotify', 'Stremio', 'Netflix', 'Disney+', 'Prime Video', 'Apple TV+', 'Hulu', 'Max', 'Peacock', 'Custom'] as const;
const defaults: Settings = {
  accent: 'violet', density: 'comfortable', fullscreen: false, wallpaper: '',
  sections: { popular: true, apps: true, featured: true, continue: true, recent: true, recentApps: true },
  paths: { Spotify: '', Stremio: '', Netflix: '', 'Disney+': '', 'Prime Video': '', 'Apple TV+': '', Hulu: '', Max: '', Peacock: '', Custom: '' },
  order: ['YouTube', 'Spotify', 'Stremio', 'Netflix', 'Disney+', 'Prime Video', 'Apple TV+', 'Hulu', 'Max', 'Peacock', 'Custom'],
};

export function createSettings() {
  let value = structuredClone(defaults);
  let error = '';
  try {
    const saved: unknown = JSON.parse(localStorage.getItem('luma-v1') ?? 'null');
    if (saved && typeof saved === 'object') {
      const data = saved as Partial<Settings>;
      if (['violet', 'blue', 'mint', 'peach'].includes(data.accent ?? '')) value.accent = data.accent!;
      if (data.density === 'compact') value.density = data.density;
      value.fullscreen = data.fullscreen === true;
      if (typeof data.wallpaper === 'string' && /^data:image\/(png|jpeg|webp|gif);base64,/.test(data.wallpaper)) value.wallpaper = data.wallpaper;
      for (const id of executable) if (typeof data.paths?.[id] === 'string') value.paths[id] = data.paths[id];
      if (Array.isArray(data.order)) {
        const order = data.order.filter((id): id is AppId => defaults.order.includes(id) && typeof id === 'string');
        value.order = [...new Set(order), ...defaults.order.filter(id => !order.includes(id))];
      }
      if (data.sections && typeof data.sections === 'object') {
        for (const key of ['popular', 'apps', 'featured', 'continue', 'recent', 'recentApps'] as const) {
          if (typeof data.sections[key] === 'boolean') value.sections[key] = data.sections[key];
        }
      }
    }
  } catch { error = 'Saved preferences could not be read.'; }
  const state = $state({ value, error });
  return state;
}

export function moveApp(settings: Settings, id: AppId, direction: -1 | 1) {
  const index = settings.order.indexOf(id);
  const target = index + direction;
  if (target < 0 || target >= settings.order.length) return;
  [settings.order[index], settings.order[target]] = [settings.order[target], settings.order[index]];
}
