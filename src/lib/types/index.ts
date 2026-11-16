export type AppId = 'YouTube' | 'Steam' | 'Spotify' | 'Stremio' | 'Netflix' | 'Disney+' | 'Prime Video' | 'Apple TV+' | 'Hulu' | 'Max' | 'Peacock' | 'NBA' | 'NFL' | 'MLB' | 'NHL' | 'Custom';
export type ExecutableAppId = Exclude<AppId, 'YouTube' | 'NBA' | 'NFL' | 'MLB' | 'NHL'>;
export type NavigationCommand = 'up' | 'down' | 'left' | 'right' | 'activate' | 'back' | 'home';
export type ThemeId = 'violet-night' | 'sapphire' | 'mint' | 'ember' | 'graphite' | 'onyx';
export type SoundPreset = 'real' | 'google' | 'roku' | 'off';
export type SoundLevel = 'soft' | 'standard' | 'loud';
export type TextSize = 'standard' | 'large' | 'larger';
export type HomeRowId = 'hero' | 'continue' | 'apps' | 'popular' | 'featured' | 'recent' | 'recentApps';
export type WidgetId = 'spotify';
export type WidgetPlacement = 'bar' | 'top-left' | 'top-right' | 'bottom-right' | 'bottom-left';
export interface HomeRow { id: string; component: HomeRowId; visible: boolean; span: 1 | 2; }
export interface WidgetConfig { enabled: boolean; placement: WidgetPlacement; visibility: 'always' | 'playing' | 'available'; position: { x: number; y: number } | null; }
export interface Settings {
  theme: ThemeId;
  density: 'comfortable' | 'compact';
  appTileShape: 'wide' | 'square';
  homeShortcut: string;
  fullscreen: boolean;
  fastAppSwitch: boolean;
  interfaceSounds: SoundPreset;
  soundLevel: SoundLevel;
  textSize: TextSize;
  reducedMotion: boolean;
  home: HomeRow[];
  homeColumns: 1 | 2;
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
export interface SteamGame { appid: number; name: string; icon: string; hero: string; header: string; playtimeMinutes: number }
export interface StremioLink { code: string; link: string; qr: string }
export interface StremioStatus {
  connected: boolean; email: string | null; lastSync: number | null;
  linking: StremioLink | null; items: StremioItem[]; continueWatching: StremioItem[];
}
