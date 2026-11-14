import type { ActivityItem } from '../types';

export function progressPercent(item: ActivityItem): number | null {
  if (item.progressSource === 'unknown') return null;
  const position = item.progressSeconds;
  const duration = item.durationSeconds;
  if (typeof position !== 'number' || typeof duration !== 'number') return null;
  if (!Number.isFinite(position) || !Number.isFinite(duration) || duration <= 0 || position <= 0 || position >= duration) return null;
  return Math.min(100, Math.max(0, (position / duration) * 100));
}

export function isContinueWatching(item: ActivityItem): boolean {
  return item.completed !== true && progressPercent(item) !== null;
}

export function isRecentlyPlayed(item: ActivityItem): boolean {
  return item.type !== 'application' && !isContinueWatching(item);
}

export function isApplication(item: ActivityItem): boolean {
  return item.type === 'application';
}

export function sortRecent(items: ActivityItem[]): ActivityItem[] {
  return [...items].sort((a, b) => Date.parse(b.lastActivityAt) - Date.parse(a.lastActivityAt));
}

export function actionLabel(item: ActivityItem): string {
  if (item.resumeKind === 'direct') return 'Resume';
  if (item.type === 'application') return 'Open app';
  if (item.type === 'track') return 'Open Spotify';
  if (item.type === 'video') return 'Open video';
  return 'Open';
}

export function progressLabel(item: ActivityItem): string {
  const percent = progressPercent(item);
  if (percent === null) return '';
  const left = Math.max(0, (item.durationSeconds ?? 0) - (item.progressSeconds ?? 0));
  return `${Math.round(percent)}% · ${formatDuration(left)} left`;
}

export function formatDuration(seconds: number): string {
  if (!Number.isFinite(seconds) || seconds <= 0) return '';
  const total = Math.round(seconds);
  const hours = Math.floor(total / 3600);
  const minutes = Math.round((total % 3600) / 60);
  if (hours > 0) return `${hours}h ${String(minutes).padStart(2, '0')}m`;
  return `${Math.max(1, minutes)}m`;
}

export function formatRelative(iso: string): string {
  const time = Date.parse(iso);
  if (!Number.isFinite(time)) return 'Unknown';
  const seconds = Math.max(0, Math.round((Date.now() - time) / 1000));
  if (seconds < 45) return 'Just now';
  if (seconds < 3600) return `${Math.round(seconds / 60)}m ago`;
  if (seconds < 86400) return `${Math.round(seconds / 3600)}h ago`;
  if (seconds < 604800) return `${Math.round(seconds / 86400)}d ago`;
  return new Date(time).toLocaleDateString([], { month: 'short', day: 'numeric' });
}

export function typeLabel(item: ActivityItem): string {
  if (item.type === 'track') return 'Music';
  if (item.type === 'application') return 'App';
  if (item.type === 'episode') return 'Episode';
  if (item.type === 'movie') return 'Movie';
  return 'Video';
}

export const providerNames: Record<string, string> = {
  local: 'This PC', youtube: 'YouTube', spotify: 'Spotify', plex: 'Plex', jellyfin: 'Jellyfin',
  trakt: 'Trakt', stremio: 'Stremio', netflix: 'Netflix', disneyplus: 'Disney+', primevideo: 'Prime Video',
  appletv: 'Apple TV+', hulu: 'Hulu', max: 'Max', peacock: 'Peacock',
};

export function providerName(provider: string): string {
  return providerNames[provider] ?? provider;
}

export const appProviders: Record<string, string> = {
  YouTube: 'youtube', Spotify: 'spotify', Stremio: 'stremio', Netflix: 'netflix',
  'Disney+': 'disneyplus', 'Prime Video': 'primevideo', 'Apple TV+': 'appletv',
  Hulu: 'hulu', Max: 'max', Peacock: 'peacock', Custom: 'local',
};
