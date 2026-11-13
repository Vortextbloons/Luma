import { invoke, isTauri } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { AppId, ExecutableAppId, MediaState, NavigationCommand, RemoteStatus } from '../types';

export const native = isTauri();
export const launchApp = (name: ExecutableAppId) => invoke<void>('launch_app', { name });
export const saveAppPaths = (paths: Record<ExecutableAppId, string>) => invoke<void>('save_app_paths', { paths });
export const mediaState = () => invoke<MediaState>('media_state');
export const mediaControl = (action: 'toggle' | 'previous' | 'next') => invoke<void>('media_control', { action });
export const remoteStatus = () => invoke<RemoteStatus>('remote_status');
export const configureRemote = (lan: boolean, origin: string) => invoke<RemoteStatus>('configure_remote', { lan, origin });
export const openYouTube = (fullscreen: boolean) => invoke<void>('open_youtube', { fullscreen });
export const closeYouTube = () => invoke<void>('close_youtube');
export const onNavigation = (callback: (command: NavigationCommand) => void) => listen<NavigationCommand>('remote-navigation', event => callback(event.payload));
export const onMedia = (callback: (state: MediaState) => void) => listen<MediaState>('spotify-state', event => callback(event.payload));
export const onRemote = (callback: (state: RemoteStatus) => void) => listen<RemoteStatus>('remote-status', event => callback(event.payload));
