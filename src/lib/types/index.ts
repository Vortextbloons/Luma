export type AppId = 'YouTube' | 'Spotify' | 'Stremio' | 'Netflix' | 'Disney+' | 'Prime Video' | 'Apple TV+' | 'Hulu' | 'Max' | 'Peacock' | 'Custom';
export type ExecutableAppId = Exclude<AppId, 'YouTube'>;
export type NavigationCommand = 'up' | 'down' | 'left' | 'right' | 'activate' | 'back' | 'home';
export interface Settings {
  accent: 'violet' | 'blue' | 'mint' | 'peach';
  density: 'comfortable' | 'compact';
  appTileShape: 'wide' | 'square';
  homeShortcut: string;
  fullscreen: boolean;
  wallpaper: string;
  sections: { popular: boolean; apps: boolean; featured: boolean; continue: boolean; recent: boolean; recentApps: boolean };
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
export interface MediaState {
  available: boolean; title: string; artist: string; playing: boolean;
  canToggle: boolean; canPrevious: boolean; canNext: boolean; error: string | null;
}
