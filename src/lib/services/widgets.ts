import type { Component } from 'svelte';
import type { WidgetId, WidgetPlacement } from '../types';
import MiniPlayer from '../components/spotify/MiniPlayer.svelte';

export interface WidgetDefinition {
  id: WidgetId;
  name: string;
  description: string;
  component: Component<{ placement: WidgetPlacement; onopen: () => void }>;
  placements: WidgetPlacement[];
}

export const widgetDefinitions: WidgetDefinition[] = [
  {
    id: 'spotify',
    name: 'Spotify',
    description: 'Now playing with playback controls.',
    component: MiniPlayer,
    placements: ['bar', 'top-right', 'bottom-right', 'bottom-left'],
  },
];

export function widgetDefinition(id: WidgetId): WidgetDefinition {
  return widgetDefinitions.find(widget => widget.id === id) ?? widgetDefinitions[0];
}
