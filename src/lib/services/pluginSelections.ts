import { invoke } from '@tauri-apps/api/core';
import type { PluginsState } from './plugins';

export interface WidgetSelectionTarget {
  pluginId: string;
  widgetType: string;
  setting: string;
  source: string;
  label: string;
}

export function widgetSelectionTargets(plugins: PluginsState | null, source: string): WidgetSelectionTarget[] {
  return (plugins?.widgets ?? []).flatMap(widget => widget.selection?.source === source
    ? [{
      pluginId: widget.pluginId,
      widgetType: widget.type,
      setting: widget.selection.setting,
      source: widget.selection.source,
      label: widget.selection.label,
    }]
    : []);
}

export async function saveWidgetSelection(target: WidgetSelectionTarget, value: string) {
  const document = await invoke<{ revision: number }>('customization_read');
  await invoke('customization_plugin_settings_save', {
    pluginId: target.pluginId,
    expectedRevision: document.revision,
    settings: { [target.setting]: value },
  });
}
