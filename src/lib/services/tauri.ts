import { invoke, isTauri } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { ActivitySnapshot, ExecutableAppId, MediaState, NavigationCommand, OpenOutcome, StremioStatus, YouTubeFeedItem } from '../types';

export const native = isTauri();
export const openSetupLink = (service: string) => invoke<void>('open_setup_link', { service });
export const launchApp = (name: ExecutableAppId) => invoke<void>('launch_app', { name });
export const saveAppPaths = (paths: Record<ExecutableAppId, string>) => invoke<void>('save_app_paths', { paths });
export const mediaState = () => invoke<MediaState>('media_state');
export const mediaControl = (action: 'toggle' | 'previous' | 'next') => invoke<void>('media_control', { action });
export const openYouTube = (fullscreen: boolean) => invoke<void>('open_youtube', { fullscreen })
export const youtubeFeed = () => invoke<YouTubeFeedItem[]>('youtube_feed');
export const openYouTubeVideo = (videoId: string, fullscreen: boolean) => invoke<void>('youtube_open_video', { videoId, fullscreen });
export const onYouTubeFeed = (callback: (items: YouTubeFeedItem[]) => void) => listen<YouTubeFeedItem[]>('youtube-feed', event => callback(event.payload));
export const activitySnapshot = () => invoke<ActivitySnapshot>('activity_snapshot');
export const openActivity = (id: string, fullscreen: boolean) => invoke<OpenOutcome>('open_activity', { id, fullscreen });
export const stremioStatus = () => invoke<StremioStatus>('stremio_status');
export const stremioLinkStart = () => invoke<StremioStatus>('stremio_link_start');
export const stremioLinkPoll = () => invoke<StremioStatus>('stremio_link_poll');
export const stremioLinkCancel = () => invoke<StremioStatus>('stremio_link_cancel');
export const stremioOpenLink = () => invoke<void>('stremio_open_link');
export const stremioSync = () => invoke<StremioStatus>('stremio_sync');
export const stremioDisconnect = () => invoke<StremioStatus>('stremio_disconnect');
export const stremioOpen = (id: string) => invoke<void>('stremio_open', { id });
export const stremioOpenDetail = (kind: string, id: string) => invoke<void>('stremio_open', { kind, id });
export const onActivity = (callback: (snapshot: ActivitySnapshot) => void) => listen<ActivitySnapshot>('activity-updated', event => callback(event.payload));
export const onNavigation = (callback: (command: NavigationCommand) => void) => listen<NavigationCommand>('remote-navigation', event => callback(event.payload));
export const onOpenMusic = (callback: () => void) => listen('open-music', callback);
export const onMedia = (callback: (state: MediaState) => void) => listen<MediaState>('spotify-state', event => callback(event.payload));
