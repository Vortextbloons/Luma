import type { Component } from 'svelte';
import type { WidgetConfig, WidgetId, WidgetPlacement } from '../types';
import MiniPlayer from '../components/spotify/MiniPlayer.svelte';
import { spotify } from '../state/spotify.svelte';

export interface WidgetDefinition {
  id: WidgetId;
  name: string;
  description: string;
  component: Component<{ placement: WidgetPlacement; onopen: () => void }>;
  placements: WidgetPlacement[];
  isVisible: (visibility: WidgetConfig['visibility']) => boolean;
}

export const widgetDefinitions: WidgetDefinition[] = [
  {
    id: 'spotify',
    name: 'Spotify',
    description: 'Now playing with playback controls.',
    component: MiniPlayer,
    placements: ['bar', 'top-left', 'top-right', 'bottom-right', 'bottom-left'],
    isVisible: visibility => visibility === 'always' || (visibility === 'playing' ? spotify.playback.playing : spotify.playback.playerAvailable || Boolean(spotify.playback.title)),
  },
];

export function widgetDefinition(id: WidgetId): WidgetDefinition {
  return widgetDefinitions.find(widget => widget.id === id) ?? widgetDefinitions[0];
}
