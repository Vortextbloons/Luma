import type { AppId, Settings } from '../types';

const defaults: Settings = {
  accent: 'violet', density: 'comfortable', fullscreen: false, wallpaper: '',
  sections: { popular: true, apps: true, featured: true },
  paths: { Spotify: '', Stremio: '', Custom: '' }, order: ['YouTube', 'Spotify', 'Stremio', 'Custom'],
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
      for (const id of ['Spotify', 'Stremio', 'Custom'] as const) if (typeof data.paths?.[id] === 'string') value.paths[id] = data.paths[id];
      if (Array.isArray(data.order) && data.order.length === 4 && new Set(data.order).size === 4 && data.order.every(id => defaults.order.includes(id))) value.order = data.order;
      if (data.sections && typeof data.sections === 'object') {
        for (const key of ['popular', 'apps', 'featured'] as const) if (typeof data.sections[key] === 'boolean') value.sections[key] = data.sections[key];
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
