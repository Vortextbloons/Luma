export type AppId = 'Spotify' | 'Stremio' | 'Custom';
export type NavigationCommand = 'up' | 'down' | 'left' | 'right' | 'activate' | 'back' | 'home';
export interface Settings {
  accent: 'violet' | 'blue' | 'mint' | 'peach';
  density: 'comfortable' | 'compact';
  fullscreen: boolean;
  wallpaper: string;
  sections: { continue: boolean; apps: boolean; featured: boolean };
  paths: Record<AppId, string>;
  order: AppId[];
}
export interface MediaItem { title: string; subtitle: string; artwork: string; progress?: number }
export interface MediaState {
  available: boolean; title: string; artist: string; playing: boolean;
  canToggle: boolean; canPrevious: boolean; canNext: boolean; error: string | null;
}
export interface RemoteStatus { running: boolean; lan: boolean; address: string; token: string; origin: string; error: string | null }
