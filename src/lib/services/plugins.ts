import type { CustomizationConfig } from './customization';

export type PluginSettingValue = string | number | boolean;

export interface PluginSettingOption {
  value: string;
  label: string;
}

export interface PluginSetting {
  key: string;
  qualifiedKey: string;
  label: string;
  description: string;
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
  runtime: string;
  executable: boolean;
  executionReason: string | null;
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
}

export interface PluginsState {
  plugins: PluginRecord[];
  effective: CustomizationConfig;
}

export const pluginFieldLabels: Record<string, string> = {
  theme: 'theme',
  density: 'density',
  appTileShape: 'app tile shape',
  textSize: 'text size',
  reducedMotion: 'reduced motion',
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
