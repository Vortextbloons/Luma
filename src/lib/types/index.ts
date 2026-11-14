export type AppId = 'YouTube' | 'Steam' | 'Spotify' | 'Stremio' | 'Netflix' | 'Disney+' | 'Prime Video' | 'Apple TV+' | 'Hulu' | 'Max' | 'Peacock' | 'Custom';
export type ExecutableAppId = Exclude<AppId, 'YouTube'>;
export type NavigationCommand = 'up' | 'down' | 'left' | 'right' | 'activate' | 'back' | 'home';
export type ThemeId = 'violet-night' | 'sapphire' | 'mint' | 'ember' | 'graphite';
export type HomeRowId = 'continue' | 'apps' | 'popular' | 'featured' | 'recent' | 'recentApps';
export type WidgetId = 'spotify';
export type WidgetPlacement = 'bar' | 'top-right' | 'bottom-right' | 'bottom-left';
export interface HomeRow { id: HomeRowId; visible: boolean; }
export interface WidgetConfig { enabled: boolean; placement: WidgetPlacement; }
export interface Settings {
  theme: ThemeId;
  density: 'comfortable' | 'compact';
  appTileShape: 'wide' | 'square';
  homeShortcut: string;
  fullscreen: boolean;
  home: HomeRow[];
  widgets: Record<WidgetId, WidgetConfig>;
  paths: Record<ExecutableAppId, string>;
  order: AppId[];
}
export interface MediaItem {
  id: string; title: string; subtitle: string; poster: string; backdrop: string; description: string;
  year: string; runtime: string; rating: string;
}
export type {
  ActivityItem, ActivitySnapshot, ActivityType, OpenOutcome, ProgressSource, ProviderConfigInput,
  ProviderState, ProviderStatus, RefreshResult, ResumeKind,
} from './activity';
export interface StremioItem {
  id: string; name: string; kind: string; poster: string; progress: number;
  timeOffset: number; duration: number; lastWatched: string | null;
  videoId: string | null; mtime: string;
}
export interface YouTubeFeedItem { id: string; title: string }
export interface StremioLink { code: string; link: string; qr: string }
export interface StremioStatus {
  connected: boolean; email: string | null; lastSync: number | null;
  linking: StremioLink | null; items: StremioItem[]; continueWatching: StremioItem[];
}
