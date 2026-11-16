import { invoke } from '@tauri-apps/api/core';
import type { AppId, Settings, WidgetPlacement } from '../types';
import { defaultHomeRows, legacyHomeRows, sanitizeHomeRows } from '../services/home';
import { resolveThemeId, themeById, applyTheme, appearanceFor } from '../services/theme';
import { setSoundSettings } from '../services/sound';
import { native, setFastAppSwitch } from '../services/tauri';
import {
  createCustomizationSync, customizationConfig,
  type CustomizationDocument, type CustomizationPreview, type HomeDraft,
} from '../services/customization';
import type { PluginsState } from '../services/plugins';

const executable = ['Steam', 'Spotify', 'Stremio', 'Netflix', 'Disney+', 'Prime Video', 'Apple TV+', 'Hulu', 'Max', 'Peacock', 'Custom'] as const;
const placements: WidgetPlacement[] = ['bar', 'top-left', 'top-right', 'bottom-right', 'bottom-left'];
const appOrder: AppId[] = ['YouTube', 'Spotify', 'Stremio', 'Steam', 'NBA', 'NFL', 'MLB', 'NHL', 'Netflix', 'Disney+', 'Prime Video', 'Apple TV+', 'Hulu', 'Max', 'Peacock', 'Custom'];
const legacyOrders: AppId[][] = [
  ['YouTube', 'Spotify', 'Stremio', 'Steam', 'Netflix', 'Disney+', 'Prime Video', 'Apple TV+', 'Hulu', 'Max', 'Peacock', 'NBA', 'NFL', 'MLB', 'NHL', 'Custom'],
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
    widgets: { spotify: { enabled: true, placement: 'bar', visibility: 'available', position: null } },
    paths: { Steam: '', Spotify: '', Stremio: '', Netflix: '', 'Disney+': '', 'Prime Video': '', 'Apple TV+': '', Hulu: '', Max: '', Peacock: '', Custom: '' },
    order: [...appOrder],
  };
}

function readSettings() {
  const value = defaultSettings();
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
        if (['always', 'playing', 'available'].includes(spotify.visibility)) value.widgets.spotify.visibility = spotify.visibility;
        if (spotify.position && Number.isInteger(spotify.position.x) && Number.isInteger(spotify.position.y) && spotify.position.x >= 0 && spotify.position.x <= 1000 && spotify.position.y >= 0 && spotify.position.y <= 1000) value.widgets.spotify.position = spotify.position;
      }
    }
  } catch { error = 'Saved preferences could not be read.'; }
  return { value, error };
}

class SettingsStore {
  value = $state<Settings>(defaultSettings());
  error: string;
  notice = $state('');
  revision = $state(0);
  preview = $state.raw<CustomizationPreview | null>(null);
  plugins = $state.raw<PluginsState | null>(null);
  pluginsLoading = $state(true);
  pluginsError = $state('');
  reviewBusy = $state(false);
  reviewError = $state('');
  previewFromEditor = $state(false);
  layout = $derived(this.preview?.config ?? customizationConfig(this.value));
  private sync: ReturnType<typeof createCustomizationSync>;
  private started = false;
  private startPromise: Promise<void> | undefined;
  private stopEffects: (() => void) | undefined;

  constructor(value: Settings, error: string) {
    Object.assign(this.value, value);
    this.error = error;
    this.notice = error;
    this.sync = createCustomizationSync(this.value, message => { this.notice = message; }, {
      document: document => this.applyDocument(document),
      preview: candidate => this.applyPreview(candidate),
      plugins: state => {
        this.plugins = state;
        this.pluginsLoading = false;
        this.pluginsError = '';
      },
      pluginsError: message => {
        this.pluginsLoading = false;
        this.pluginsError = message;
      },
    });
  }

  private applyDocument(document: CustomizationDocument) {
    this.revision = document.revision;
    if (this.preview && document.revision > this.preview.baseRevision && !this.reviewBusy) {
      this.notice = 'The saved layout changed elsewhere. Preview your draft again before keeping it.';
    }
  }

  private applyPreview(candidate: CustomizationPreview | null) {
    const changed = this.preview?.id !== candidate?.id;
    this.preview = candidate;
    if (changed) this.previewFromEditor = false;
  }

  start() {
    this.startPromise ??= this.begin();
    return this.startPromise;
  }

  private async begin() {
    if (this.started) return;
    this.started = true;
    this.stopEffects = $effect.root(() => {
      $effect(() => {
        const layout = this.layout;
        applyTheme(themeById(layout.theme).tokens);
        document.documentElement.dataset.theme = layout.theme;
        document.documentElement.dataset.density = layout.density;
        document.documentElement.dataset.appTileShape = layout.appTileShape;
        document.documentElement.dataset.textSize = this.value.textSize;
        document.documentElement.dataset.reducedMotion = String(this.value.reducedMotion);
      });
      $effect(() => { setSoundSettings(this.value.interfaceSounds, this.value.soundLevel); });
      $effect(() => { this.persist(); });
      $effect(() => {
        if (native) void setFastAppSwitch(this.value.fastAppSwitch).catch(error => this.notice = String(error));
      });
      $effect(() => {
        if (native) this.sync.update(customizationConfig(this.value));
      });
      $effect(() => {
        if (!native) return;
        invoke('update_appearance', { appearance: appearanceFor(themeById(this.value.theme)) }).catch(() => {});
      });
    });
    if (native) await this.sync.start();
  }

  private persist() {
    try {
      localStorage.setItem('luma-v1', JSON.stringify({
        homeShortcut: this.value.homeShortcut, shortcutVersion: 1, theme: this.value.theme,
        density: this.value.density, appTileShape: this.value.appTileShape, fullscreen: this.value.fullscreen,
        fastAppSwitch: this.value.fastAppSwitch, interfaceSounds: this.value.interfaceSounds,
        soundLevel: this.value.soundLevel, textSize: this.value.textSize, reducedMotion: this.value.reducedMotion,
        home: this.value.home, homeColumns: this.value.homeColumns, widgets: this.value.widgets, order: this.value.order,
      }));
    } catch { this.notice = 'Preferences could not be saved. Free some local storage and try again.'; }
  }

  dispose() {
    this.stopEffects?.();
    this.stopEffects = undefined;
    this.sync.dispose();
    this.started = false;
    this.startPromise = undefined;
  }

  async previewLayout(draft: HomeDraft) {
    this.previewFromEditor = true;
    try {
      await this.sync.preview({ ...customizationConfig(this.value), home: JSON.parse(JSON.stringify(draft.home)), homeColumns: draft.homeColumns }, draft.baseRevision);
      this.previewFromEditor = true;
    } catch (error) {
      this.previewFromEditor = false;
      throw error;
    }
  }

  async keepLayout() {
    if (this.reviewBusy) return false;
    this.reviewError = '';
    this.reviewBusy = true;
    try {
      await this.sync.keep();
      return true;
    } catch (error) {
      this.reviewError = errorMessage(error);
      this.notice = this.reviewError;
      return false;
    } finally {
      this.reviewBusy = false;
    }
  }

  async cancelReview() {
    if (this.reviewBusy) return false;
    this.reviewError = '';
    this.reviewBusy = true;
    try {
      await this.sync.cancel();
      return true;
    } catch (error) {
      this.reviewError = errorMessage(error);
      this.notice = this.reviewError;
      return false;
    } finally {
      this.reviewBusy = false;
    }
  }

  async restoreLayout() {
    try {
      await this.sync.restoreLayout();
    } catch (error) {
      this.reviewError = errorMessage(error);
      throw error;
    }
  }

  refreshPlugins() {
    return this.sync.refreshPlugins();
  }
}

function errorMessage(error: unknown): string {
  return error && typeof error === 'object' && 'message' in error ? String(error.message) : String(error);
}

export function createSettings() {
  const { value, error } = readSettings();
  return new SettingsStore(value, error);
}

export type SettingsState = ReturnType<typeof createSettings>;

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
