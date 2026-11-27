import type { Component } from 'svelte';
import type { WidgetConfig } from '../types';
import MiniPlayer from '../components/spotify/MiniPlayer.svelte';
import ClockWidget from '../components/widgets/ClockWidget.svelte';
import PluginWidget from '../components/widgets/PluginWidget.svelte';
import { spotify } from '../state/spotify.svelte';
import { pluginWidgets } from '../state/pluginWidgets.svelte';
import { widgetCatalog, widgetMeta, type WidgetConditionOption, type WidgetMeta, type WidgetProps } from './widgetModel';

export type { WidgetProps } from './widgetModel';
export * from './widgetModel';

export interface WidgetCondition extends WidgetConditionOption {
  available: () => boolean;
}

export interface WidgetDefinition extends Omit<WidgetMeta, 'conditions'> {
  component: Component<WidgetProps>;
  conditions: WidgetCondition[];
}

const components: Record<string, Component<WidgetProps>> = {
  spotify: MiniPlayer,
  clock: ClockWidget,
};

const conditions: Record<string, WidgetCondition[]> = {
  spotify: [
    { value: 'always', label: 'Always', available: () => true },
    { value: 'available', label: 'When Spotify is available', available: () => spotify.playback.playerAvailable || Boolean(spotify.playback.title) },
    { value: 'playing', label: 'Only during playback', available: () => spotify.playback.playing },
  ],
  clock: [{ value: 'always', label: 'Always', available: () => true }],
};

function toDefinition(meta: WidgetMeta): WidgetDefinition {
  const component = components[meta.type] ?? PluginWidget;
  const widgetConditions = conditions[meta.type] ?? [
    { value: 'always', label: 'Always', available: () => true },
    { value: 'available', label: 'When active', available: () => pluginWidgets.available(meta.type) },
  ];
  return { ...meta, component, conditions: widgetConditions };
}

export function widgetDefinitionsList(): WidgetDefinition[] {
  return widgetCatalog.map(toDefinition);
}

export const widgetDefinitions: WidgetDefinition[] = widgetDefinitionsList();

export function widgetDefinition(type: string): WidgetDefinition | undefined {
  const meta = widgetMeta(type);
  return meta ? toDefinition(meta) : undefined;
}

export function widgetAvailable(config: WidgetConfig) {
  const definition = widgetDefinition(config.type);
  if (!definition) return false;
  return definition.conditions.find(condition => condition.value === config.visibility)?.available() ?? true;
}
