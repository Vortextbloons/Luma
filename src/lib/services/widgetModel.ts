import type {
  Settings, WidgetConfig, WidgetContext, WidgetPlacement, WidgetPresence, WidgetSize,
  WidgetVisibility,
} from '../types';

export interface WidgetSettingField {
  key: string;
  label: string;
  description?: string;
  type: 'boolean' | 'choice';
  options?: { value: string; label: string }[];
}

export interface WidgetConditionOption {
  value: WidgetVisibility;
  label: string;
}

export interface WidgetProps {
  config: WidgetConfig;
  context: WidgetContext;
  compact?: boolean;
  moving?: boolean;
  onopen: () => void;
}

export interface WidgetMeta {
  type: string;
  name: string;
  description: string;
  glyph: string;
  placements: WidgetPlacement[];
  sizes: WidgetSize[];
  variants: string[];
  contexts: WidgetPresence[];
  conditions: WidgetConditionOption[];
  defaultPlacement: WidgetPlacement;
  defaultSize: WidgetSize;
  defaultVariant: string;
  defaultPresence: WidgetPresence[];
  defaultVisibility: WidgetVisibility;
  defaultSettings: Record<string, unknown>;
  settings: WidgetSettingField[];
  maxInstances?: number;
}

export const widgetPlacements: WidgetPlacement[] = [
  'bar', 'top-left', 'top-center', 'top-right',
  'bottom-left', 'bottom-center', 'bottom-right',
  'rail-left', 'rail-right',
];
export const widgetSizes: WidgetSize[] = ['small', 'medium', 'large'];
export const widgetPresences: WidgetPresence[] = ['home', 'music', 'idle'];
export const widgetVariants = ['glass', 'cinematic', 'ticker', 'minimal'];

export const placementLabels: Record<WidgetPlacement, string> = {
  bar: 'Bottom bar',
  'top-left': 'Top left',
  'top-center': 'Top center',
  'top-right': 'Top right',
  'bottom-left': 'Bottom left',
  'bottom-center': 'Bottom center',
  'bottom-right': 'Bottom right',
  'rail-left': 'Left rail',
  'rail-right': 'Right rail',
};

export const sizeLabels: Record<WidgetSize, string> = {
  small: 'Small',
  medium: 'Medium',
  large: 'Large',
};

export const variantLabels: Record<string, string> = {
  glass: 'Glass',
  cinematic: 'Cinematic',
  ticker: 'Ticker',
  minimal: 'Minimal',
};

export const presenceLabels: Record<WidgetPresence, string> = {
  home: 'Home',
  music: 'In the player',
  idle: 'When idle',
};

export const builtinWidgetCatalog: WidgetMeta[] = [
  {
    type: 'spotify',
    name: 'Now playing',
    description: 'Current track with playback controls.',
    glyph: '♪',
    placements: widgetPlacements,
    sizes: ['medium', 'large'],
    variants: ['glass', 'cinematic', 'ticker', 'minimal'],
    contexts: ['home', 'idle'],
    conditions: [
      { value: 'always', label: 'Always' },
      { value: 'available', label: 'When Spotify is available' },
      { value: 'playing', label: 'Only during playback' },
    ],
    defaultPlacement: 'bar',
    defaultSize: 'medium',
    defaultVariant: 'glass',
    defaultPresence: ['home', 'idle'],
    defaultVisibility: 'available',
    defaultSettings: {},
    settings: [],
    maxInstances: 1,
  },
  {
    type: 'clock',
    name: 'Clock',
    description: 'Time and date, with a calm idle face.',
    glyph: '◷',
    placements: widgetPlacements.filter(placement => placement !== 'bar'),
    sizes: ['small', 'medium', 'large'],
    variants: ['glass', 'cinematic', 'ticker', 'minimal'],
    contexts: ['home', 'music', 'idle'],
    conditions: [{ value: 'always', label: 'Always' }],
    defaultPlacement: 'top-right',
    defaultSize: 'small',
    defaultVariant: 'minimal',
    defaultPresence: ['home', 'music', 'idle'],
    defaultVisibility: 'always',
    defaultSettings: { hour12: false, showDate: true },
    settings: [
      { key: 'hour12', label: '12-hour clock', description: 'Use 1:30 instead of 13:30.', type: 'boolean' },
      { key: 'showDate', label: 'Show the date', description: 'Add the weekday and date under the time.', type: 'boolean' },
    ],
    maxInstances: 8,
  },
];

export const widgetCatalog: WidgetMeta[] = [...builtinWidgetCatalog];

export function setPluginWidgets(widgets: WidgetMeta[]) {
  widgetCatalog.splice(0, widgetCatalog.length, ...builtinWidgetCatalog, ...widgets);
}

export function widgetMeta(type: string): WidgetMeta | undefined {
  return widgetCatalog.find(widget => widget.type === type);
}

export function defaultWidgetConfig(type: string, id = type): WidgetConfig {
  const meta = widgetMeta(type);
  return {
    type,
    enabled: true,
    placement: meta?.defaultPlacement ?? 'bottom-right',
    visibility: meta?.defaultVisibility ?? 'always',
    position: null,
    size: meta?.defaultSize ?? 'medium',
    variant: meta?.defaultVariant ?? 'glass',
    presence: [...(meta?.defaultPresence ?? ['home', 'idle'])],
    order: 0,
    settings: { ...(meta?.defaultSettings ?? {}) },
  };
}

export function defaultWidgets(): Settings['widgets'] {
  return { spotify: defaultWidgetConfig('spotify') };
}

export function nextWidgetId(type: string, widgets: Settings['widgets']) {
  if (!widgets[type]) return type;
  let index = 2;
  while (widgets[`${type}-${index}`]) index++;
  return `${type}-${index}`;
}

export function addWidget(widgets: Settings['widgets'], type: string) {
  const meta = widgetMeta(type);
  if (!meta) return null;
  const count = Object.values(widgets).filter(widget => widget.type === type).length;
  if (meta.maxInstances != null && count >= meta.maxInstances) return null;
  const id = nextWidgetId(type, widgets);
  const config = defaultWidgetConfig(type, id);
  config.order = Object.values(widgets).reduce((max, widget) => Math.max(max, widget.order), -1) + 1;
  widgets[id] = config;
  return id;
}

export function isPlacement(value: unknown): value is WidgetPlacement {
  return typeof value === 'string' && (widgetPlacements as string[]).includes(value);
}

export function isSize(value: unknown): value is WidgetSize {
  return typeof value === 'string' && (widgetSizes as string[]).includes(value);
}

export function isPresence(value: unknown): value is WidgetPresence {
  return typeof value === 'string' && (widgetPresences as string[]).includes(value);
}

export function isVisibility(value: unknown): value is WidgetVisibility {
  return value === 'always' || value === 'playing' || value === 'available';
}

const instancePattern = /^[A-Za-z0-9_-]{1,64}$/;
const typePattern = /^[A-Za-z0-9_.-]{1,64}$/;
const variantPattern = /^[A-Za-z0-9_-]{1,32}$/;

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function coordinate(value: unknown): value is number {
  return typeof value === 'number' && Number.isInteger(value) && value >= 0 && value <= 1000;
}

function sanitizeValue(value: unknown, depth = 0): unknown | undefined {
  if (value === null) return null;
  if (typeof value === 'string') return value.length <= 4096 ? value : undefined;
  if (typeof value === 'number') return Number.isFinite(value) ? value : undefined;
  if (typeof value === 'boolean') return value;
  if (depth >= 3) return undefined;
  if (Array.isArray(value)) {
    if (value.length > 32) return undefined;
    const items: unknown[] = [];
    for (const item of value) {
      const clean = sanitizeValue(item, depth + 1);
      if (clean === undefined) return undefined;
      items.push(clean);
    }
    return items;
  }
  if (isRecord(value)) {
    const keys = Object.keys(value);
    if (keys.length > 32) return undefined;
    const result: Record<string, unknown> = {};
    for (const key of keys) {
      if (!/^[A-Za-z0-9_-]{1,64}$/.test(key)) continue;
      const clean = sanitizeValue(value[key], depth + 1);
      if (clean !== undefined) result[key] = clean;
    }
    return result;
  }
  return undefined;
}

function sanitizeSettings(value: Record<string, unknown>) {
  const settings: Record<string, unknown> = {};
  let count = 0;
  for (const [key, entry] of Object.entries(value)) {
    if (count >= 32) break;
    if (!/^[A-Za-z0-9_-]{1,64}$/.test(key)) continue;
    const clean = sanitizeValue(entry);
    if (clean === undefined) continue;
    settings[key] = clean;
    count++;
  }
  return settings;
}

export function sanitizeWidgets(value: unknown): Settings['widgets'] | null {
  if (!isRecord(value)) return null;
  const widgets: Settings['widgets'] = {};
  let count = 0;
  for (const [id, raw] of Object.entries(value)) {
    if (count >= 32) break;
    if (!instancePattern.test(id) || !isRecord(raw)) continue;
    const type = typeof raw.type === 'string' && raw.type ? raw.type : 'spotify';
    if (!typePattern.test(type)) continue;
    const meta = widgetMeta(type);
    const config = defaultWidgetConfig(type, id);
    config.order = count;
    if (typeof raw.enabled === 'boolean') config.enabled = raw.enabled;
    if (isPlacement(raw.placement)) config.placement = raw.placement;
    if (isVisibility(raw.visibility)) config.visibility = raw.visibility;
    if (raw.position === null) config.position = null;
    else if (isRecord(raw.position) && coordinate(raw.position.x) && coordinate(raw.position.y)) {
      config.position = { x: raw.position.x, y: raw.position.y };
    }
    if (isSize(raw.size)) config.size = raw.size;
    if (typeof raw.variant === 'string' && variantPattern.test(raw.variant)) config.variant = raw.variant;
    if (Array.isArray(raw.presence)) {
      const presence = [...new Set(raw.presence.filter(isPresence))];
      if (presence.length) config.presence = presence;
    }
    if (typeof raw.order === 'number' && Number.isInteger(raw.order) && raw.order >= 0 && raw.order <= 1000) config.order = raw.order;
    if (isRecord(raw.settings)) config.settings = sanitizeSettings(raw.settings);
    if (meta) {
      if (!meta.placements.includes(config.placement)) config.placement = meta.defaultPlacement;
      if (!meta.sizes.includes(config.size)) config.size = meta.defaultSize;
      if (!meta.variants.includes(config.variant)) config.variant = meta.defaultVariant;
      config.presence = config.presence.filter(item => meta.contexts.includes(item));
      if (!config.presence.length) config.presence = [...meta.defaultPresence];
      if (!meta.conditions.some(condition => condition.value === config.visibility)) config.visibility = meta.defaultVisibility;
      const settings: Record<string, unknown> = {};
      for (const field of meta.settings) {
        if (config.settings[field.key] !== undefined) settings[field.key] = config.settings[field.key];
        else if (meta.defaultSettings[field.key] !== undefined) settings[field.key] = meta.defaultSettings[field.key];
      }
      config.settings = settings;
    }
    widgets[id] = config;
    count++;
  }
  return widgets;
}

export function widgetInstances(widgets: Settings['widgets']) {
  return Object.entries(widgets)
    .map(([id, config]) => ({ id, config, meta: widgetMeta(config.type) }))
    .sort((a, b) => a.config.order - b.config.order || a.id.localeCompare(b.id));
}

export function visibleInContext(config: WidgetConfig, context: WidgetContext) {
  if (context === 'idle') return config.presence.includes('home') || config.presence.includes('idle');
  return config.presence.includes(context);
}

export function widgetSummary(config: WidgetConfig) {
  if (!config.enabled) return 'Off';
  const presence = config.presence.map(item => presenceLabels[item].toLowerCase()).join(', ');
  return `${placementLabels[config.placement]} · ${presence || 'No context'} · ${sizeLabels[config.size]}`;
}

export type PlacementPreset = 'dock' | 'corners' | 'rails';

export const placementPresets: { id: PlacementPreset; label: string; description: string }[] = [
  { id: 'dock', label: 'Dock', description: 'One bar at the bottom, extras on the right rail.' },
  { id: 'corners', label: 'Corners', description: 'Spread widgets across the screen edges.' },
  { id: 'rails', label: 'Rails', description: 'Stack everything on the right edge.' },
];

const cornerCycle: WidgetPlacement[] = ['top-right', 'bottom-right', 'top-left', 'bottom-left', 'top-center', 'bottom-center'];

export function applyPlacementPreset(widgets: Settings['widgets'], preset: PlacementPreset) {
  const items = widgetInstances(widgets);
  const docked = items.find(item => item.meta?.defaultPlacement === 'bar');
  items.forEach((item, index) => {
    item.config.position = null;
    item.config.placement = preset === 'dock'
      ? item === docked ? 'bar' : 'rail-right'
      : preset === 'rails'
        ? 'rail-right'
        : cornerCycle[index % cornerCycle.length];
  });
}

export function applyStylePreset(widgets: Settings['widgets'], variant: string) {
  for (const item of widgetInstances(widgets)) {
    if (item.meta?.variants.includes(variant)) item.config.variant = variant;
  }
}

export function removeWidget(widgets: Settings['widgets'], id: string) {
  delete widgets[id];
}

export function resetWidget(widgets: Settings['widgets'], id: string) {
  const current = widgets[id];
  if (!current) return;
  widgets[id] = defaultWidgetConfig(current.type, id);
  widgets[id].order = current.order;
}
