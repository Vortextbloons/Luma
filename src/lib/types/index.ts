export type AppId = 'YouTube' | 'Steam' | 'Spotify' | 'Stremio' | 'Netflix' | 'Disney+' | 'Prime Video' | 'Apple TV+' | 'Hulu' | 'Max' | 'Peacock' | 'NBA' | 'NFL' | 'MLB' | 'NHL' | 'Custom';
export type ExecutableAppId = Exclude<AppId, 'YouTube' | 'NBA' | 'NFL' | 'MLB' | 'NHL'>;
export type NavigationCommand = 'up' | 'down' | 'left' | 'right' | 'activate' | 'back' | 'home';
export type ControllerAction = NavigationCommand
  | 'tabprev'
  | 'tabnext'
  | 'search'
  | 'settings'
  | 'playpause'
  | 'trackprev'
  | 'tracknext'
  | 'scores';
export type ControllerBinds = Record<ControllerAction, number | null>;
export type ThemeId =
  | 'violet-night'
  | 'sapphire'
  | 'mint'
  | 'ember'
  | 'graphite'
  | 'onyx'
  | 'aurora'
  | 'synthwave'
  | 'biolume';
export type SoundPreset = 'real' | 'google' | 'roku' | 'off';
export type SoundLevel = 'soft' | 'standard' | 'loud';
export type TextSize = 'standard' | 'large' | 'larger';
export type HomeRowId = 'hero' | 'continue' | 'apps' | 'popular' | 'featured' | 'recent' | 'recentApps';
export type WidgetPlacement = 'bar' | 'top-left' | 'top-center' | 'top-right' | 'bottom-left' | 'bottom-center' | 'bottom-right' | 'rail-left' | 'rail-right';
export type WidgetVisibility = 'always' | 'playing' | 'available';
export type WidgetSize = 'small' | 'medium' | 'large';
export type WidgetPresence = 'home' | 'music' | 'idle';
export type WidgetContext = WidgetPresence;
export interface HomeRow { id: string; component: HomeRowId; visible: boolean; span: 1 | 2; }
export interface WidgetConfig {
  type: string;
  enabled: boolean;
  placement: WidgetPlacement;
  visibility: WidgetVisibility;
  position: { x: number; y: number } | null;
  size: WidgetSize;
  variant: string;
  presence: WidgetPresence[];
  order: number;
  settings: Record<string, unknown>;
}
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
  smoothAppHover: boolean;
  controllerEnabled: boolean;
  controllerBinds: ControllerBinds;
  home: HomeRow[];
  homeColumns: 1 | 2;
  widgets: Record<string, WidgetConfig>;
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
export interface SteamShortcutInfo { appid: number; name: string; exe: string; startDir: string }
export interface SportsBadgeSide { team: string; value: string; pct?: number }
export interface SportsBadge { pluginId: string; league: string; teams: string[]; label: string; sides: SportsBadgeSide[]; updatedAt: number; showOnCards: boolean; showInPlayer: boolean }
export interface StremioLink { code: string; link: string; qr: string }
export interface StremioStatus {
  connected: boolean; email: string | null; lastSync: number | null;
  linking: StremioLink | null; items: StremioItem[]; continueWatching: StremioItem[];
}
