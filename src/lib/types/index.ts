export type AppId = 'YouTube' | 'Spotify' | 'Stremio' | 'Custom';
export type ExecutableAppId = Exclude<AppId, 'YouTube'>;
export type NavigationCommand = 'up' | 'down' | 'left' | 'right' | 'activate' | 'back' | 'home';
export interface Settings {
  accent: 'violet' | 'blue' | 'mint' | 'peach';
  density: 'comfortable' | 'compact';
  fullscreen: boolean;
  wallpaper: string;
  sections: { popular: boolean; apps: boolean; featured: boolean };
  paths: Record<ExecutableAppId, string>;
  order: AppId[];
}
export interface MediaItem {
  id: string; title: string; subtitle: string; artwork: string; description: string;
  year: string; runtime: string; rating: string;
}
export interface MediaState {
  available: boolean; title: string; artist: string; playing: boolean;
  canToggle: boolean; canPrevious: boolean; canNext: boolean; error: string | null;
}
export interface RemoteStatus { running: boolean; lan: boolean; address: string; token: string; origin: string; error: string | null }
