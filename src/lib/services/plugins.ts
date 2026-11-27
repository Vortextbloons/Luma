import type { CustomizationConfig } from './customization';

export type PluginSettingValue = string | number | boolean;
export type PluginRuntimeKind = 'declarative' | 'isolated-ui' | 'background' | 'main-page';
export type PluginTrigger = 'on-demand' | 'launcher-open' | 'startup' | 'always';

export interface PluginRuntimeStatus {
  state: string;
  reason: string | null;
  restarts: number;
}

export interface PluginSettingOption {
  value: string;
  label: string;
}

export interface PluginSetting {
  key: string;
  qualifiedKey: string;
  label: string;
  description: string;
  tab?: string | null;
  kind: 'boolean' | 'enum' | 'number' | 'string';
  value: PluginSettingValue;
  default: PluginSettingValue;
  valid: boolean;
  options: PluginSettingOption[];
  min: number | null;
  max: number | null;
  integer: boolean;
  maxLength: number;
  minLength: number;
}

export interface PluginRecord {
  id: string;
  name: string;
  version: string;
  apiVersion: number;
  description: string;
  author: string;
  enabled: boolean;
  builtin: boolean;
  runtime: string;
  executable: boolean;
  executionReason: string | null;
  permissions: string[];
  network: string[];
  entry: string | null;
  surfaceOpenable: boolean;
  controls: string[];
  declaredControls: string[];
  detachedFields: string[];
  conflicts: string[];
  canEnable: boolean;
  enableError: string | null;
  disabledReason: string | null;
  settings: PluginSetting[];
  source: string;
  installedAt: string;
  sizeBytes: number;
  contentHash: string;
  sourceFiles: string[];
  assets: string[];
  iconUrl?: string | null;
  runtimeStatus?: PluginRuntimeStatus;
  grantedPermissions?: string[];
  grantedNetwork?: string[];
  trigger?: string | null;
}

export interface PluginHomeCard {
  id: string;
  pluginId: string;
  title: string;
  commandId: string;
}

export interface PluginHomeCardFailure {
  card: PluginHomeCard;
  message: string;
}

export interface PluginWidgetRow {
  label: string;
  value: string;
  sub?: string;
  accent?: string;
}

export interface PluginWidgetAction {
  label: string;
  command?: string;
  url?: string;
}

export interface PluginWidgetData {
  pluginId: string;
  id: string;
  type: string;
  available: boolean;
  title: string;
  subtitle?: string;
  primary?: string;
  badge?: string;
  tone?: string;
  progress?: number;
  progressLabel?: string;
  meta?: string[];
  rows?: PluginWidgetRow[];
  action?: PluginWidgetAction;
  updatedAt: number;
}

export interface PluginWidgetSelection {
  setting: string;
  source: string;
  label: string;
}

export interface PluginWidgetMeta {
  type: string;
  pluginId: string;
  widgetId: string;
  name: string;
  description: string;
  glyph: string;
  placements: string[];
  sizes: string[];
  variants: string[];
  contexts: string[];
  defaultPlacement: string;
  defaultSize: string;
  defaultVariant: string;
  defaultPresence: string[];
  defaultSettings: Record<string, unknown>;
  settings: {
    key: string;
    label: string;
    description?: string;
    type: 'boolean' | 'choice';
    options?: { value: string; label: string }[];
  }[];
  maxInstances?: number;
  selection?: PluginWidgetSelection;
}

export interface PluginsState {
  plugins: PluginRecord[];
  effective: CustomizationConfig;
  homeCards: PluginHomeCard[];
  widgets?: PluginWidgetMeta[];
}

export const pluginFieldLabels: Record<string, string> = {
  theme: 'theme',
  density: 'density',
  appTileShape: 'app tile shape',
  textSize: 'text size',
  reducedMotion: 'reduced motion',
  appearance: 'appearance',
  homeColumns: 'Home columns',
  home: 'Home layout',
  widgets: 'widgets',
  order: 'app order',
};

export function pluginFieldLabel(field: string) {
  return pluginFieldLabels[field] ?? field;
}

export function pluginControlSummary(record: PluginRecord) {
  if (!record.controls.length) return 'No appearance controls attached';
  const labels = record.controls.map(pluginFieldLabel);
  if (labels.length === 1) return `Controls ${labels[0]}`;
  if (labels.length === 2) return `Controls ${labels[0]} and ${labels[1]}`;
  return `Controls ${labels.slice(0, -1).join(', ')}, and ${labels.at(-1)}`;
}

export function pluginSettingValues(record: PluginRecord) {
  return Object.fromEntries(record.settings.map(setting => [setting.key, setting.value]));
}

export function pluginRuntimeKind(record: Pick<PluginRecord, 'runtime' | 'executable'>): PluginRuntimeKind {
  const runtime = record.runtime.trim().toLowerCase();
  if (runtime === 'background' || runtime === 'isolated-ui' || runtime === 'main-page' || runtime === 'declarative') return runtime;
  return record.executable ? 'isolated-ui' : 'declarative';
}

export function pluginRuntimeLabel(record: Pick<PluginRecord, 'runtime' | 'executable'>) {
  const kind = pluginRuntimeKind(record);
  if (kind === 'background') return 'Background service';
  if (kind === 'isolated-ui') return 'Plugin UI';
  if (kind === 'main-page') return 'Unsupported runtime';
  return 'Declarative';
}

export function pluginTriggers(record: Pick<PluginRecord, 'runtime' | 'executable' | 'apiVersion'>): PluginTrigger[] {
  const kind = pluginRuntimeKind(record);
  if (kind === 'background') return ['launcher-open', 'startup', 'always'];
  if (kind === 'isolated-ui') return record.apiVersion >= 3 ? ['on-demand'] : ['launcher-open'];
  return [];
}

export function pluginDefaultTrigger(record: Pick<PluginRecord, 'runtime' | 'executable' | 'trigger' | 'apiVersion'>): PluginTrigger {
  const triggers = pluginTriggers(record);
  const previous = record.trigger as PluginTrigger | null | undefined;
  return previous && triggers.includes(previous) ? previous : triggers[0] ?? 'launcher-open';
}

export function pluginTriggerLabel(trigger: PluginTrigger) {
  if (trigger === 'on-demand') return 'When you open it';
  if (trigger === 'startup') return 'When Luma starts';
  if (trigger === 'always') return 'Always while enabled';
  return 'While Luma is open';
}

export function pluginTriggerDescription(trigger: PluginTrigger) {
  if (trigger === 'on-demand') return 'Starts only when you choose its Home card or Open in Settings. It stops when you press Back.';
  if (trigger === 'startup') return 'Starts with Luma, including when Luma opens in the tray, and keeps running after the window closes.';
  if (trigger === 'always') return 'Runs the whole time it is enabled, including while Luma is in the tray, until you disable it.';
  return 'Runs only while Luma is open. Hiding Luma to the tray stops it.';
}

export function pluginTriggerSummary(trigger: PluginTrigger) {
  if (trigger === 'on-demand') return 'Starts only when you open it';
  if (trigger === 'startup') return 'Starts with Luma, including in the tray';
  if (trigger === 'always') return 'Always on while enabled, including in the tray';
  return 'Runs while Luma is open';
}

const permissionDescriptions: Record<string, string> = {
  'settings.read': 'Reads this plugin’s own settings.',
  'network.get': 'Requests data through Luma from the origins listed below.',
  'playback.read': 'Reads what is playing.',
  'playback.control': 'Plays, pauses, skips, and changes playback.',
};

export function pluginPermissionDescription(permission: string) {
  return permissionDescriptions[permission] ?? 'Access declared by this package.';
}

export function pluginRuntimeStateLabel(state: string) {
  const normalized = state.trim().toLowerCase();
  const labels: Record<string, string> = {
    starting: 'Starting',
    running: 'Running',
    failed: 'Failed',
  };
  if (labels[normalized]) return labels[normalized];
  if (!normalized) return 'Unknown';
  return normalized.charAt(0).toUpperCase() + normalized.slice(1);
}

export function pluginRuntimeTone(state: string) {
  const normalized = state.trim().toLowerCase();
  if (normalized === 'running') return 'is-good';
  if (normalized === 'failed') return 'is-warn';
  return 'is-muted';
}
