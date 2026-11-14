export type AppId = 'YouTube' | 'Spotify' | 'Stremio' | 'Custom';
export type ExecutableAppId = Exclude<AppId, 'YouTube'>;
export type NavigationCommand = 'up' | 'down' | 'left' | 'right' | 'activate' | 'back' | 'home';
export interface Settings {
  accent: 'violet' | 'blue' | 'mint' | 'peach';
  density: 'comfortable' | 'compact';
  fullscreen: boolean;
  wallpaper: string;
  sections: { popular: boolean; apps: boolean; featured: boolean; continue: boolean };
  paths: Record<ExecutableAppId, string>;
  order: AppId[];
}
export interface MediaItem {
  id: string; title: string; subtitle: string; poster: string; backdrop: string; description: string;
  year: string; runtime: string; rating: string;
}
export type ActivityType = 'movie' | 'episode' | 'video' | 'track' | 'application';
export interface ActivityItem {
  id: string; provider: string; type: ActivityType; title: string; subtitle: string | null;
  artwork: string | null; lastActivityAt: string; progressSeconds: number | null;
  durationSeconds: number | null; completed: boolean | null;
  resumeKind: 'direct' | 'content' | 'service' | 'none';
}
export interface ActivitySnapshot { items: ActivityItem[]; trackingEnabled: boolean; generatedAt: string }
export interface StremioItem {
  id: string; name: string; kind: string; poster: string; progress: number;
  timeOffset: number; duration: number; lastWatched: string | null;
  videoId: string | null; mtime: string;
}
export interface StremioLink { code: string; link: string; qr: string }
export interface StremioStatus {
  connected: boolean; email: string | null; lastSync: number | null;
  linking: StremioLink | null; items: StremioItem[]; continueWatching: StremioItem[];
}
export interface MediaState {
  available: boolean; title: string; artist: string; playing: boolean;
  canToggle: boolean; canPrevious: boolean; canNext: boolean; error: string | null;
}
export interface RemoteStatus { running: boolean; lan: boolean; address: string; token: string; origin: string; error: string | null }
