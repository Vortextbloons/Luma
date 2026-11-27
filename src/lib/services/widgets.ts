import type { Component } from 'svelte';
import type { WidgetConfig } from '../types';
import MiniPlayer from '../components/spotify/MiniPlayer.svelte';
import ClockWidget from '../components/widgets/ClockWidget.svelte';
import { spotify } from '../state/spotify.svelte';
import { widgetCatalog, type WidgetConditionOption, type WidgetMeta, type WidgetProps } from './widgetModel';

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

export const widgetDefinitions: WidgetDefinition[] = widgetCatalog.flatMap(meta => {
  const component = components[meta.type];
  if (!component) return [];
  return [{ ...meta, component, conditions: conditions[meta.type] ?? meta.conditions.map(condition => ({ ...condition, available: () => true })) }];
});

export function widgetDefinition(type: string): WidgetDefinition | undefined {
  return widgetDefinitions.find(widget => widget.type === type);
}

export function widgetAvailable(config: WidgetConfig) {
  const definition = widgetDefinition(config.type);
  if (!definition) return false;
  return definition.conditions.find(condition => condition.value === config.visibility)?.available() ?? true;
}
