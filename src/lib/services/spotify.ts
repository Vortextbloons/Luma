import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type {
  SpotifyBrowseKind,
  SpotifyBrowsePage,
  SpotifyConnection,
  SpotifyDevice,
  SpotifyHomeData,
  SpotifyPlaybackState,
  SpotifyQueue,
  SpotifySearchResults,
} from '../types/spotify';

export const spotifyStatus = () => invoke<SpotifyConnection>('spotify_status');
export const spotifyPlayback = () => invoke<SpotifyPlaybackState>('spotify_playback');
export const spotifyControl = (action: string, value?: string) => invoke<void>('spotify_control', { action, value: value ?? null });
export const spotifyPlay = (uri: string, deviceId?: string, contextUri?: string) => invoke<void>('spotify_play', { uri, deviceId: deviceId ?? null, contextUri: contextUri ?? null });
export const spotifyTransfer = (deviceId: string) => invoke<void>('spotify_transfer', { deviceId });
export const spotifyHome = () => invoke<SpotifyHomeData>('spotify_home');
export const spotifySearch = (query: string) => invoke<SpotifySearchResults>('spotify_search', { query });
export const spotifyQueue = () => invoke<SpotifyQueue>('spotify_queue');
export const spotifyDevices = () => invoke<SpotifyDevice[]>('spotify_devices');
export const spotifyClientId = () => invoke<string>('spotify_client_id');
export const spotifySetClientId = (clientId: string) => invoke<void>('spotify_set_client_id', { clientId });
export const spotifyConnect = () => invoke<SpotifyConnection>('spotify_connect');
export const spotifyDisconnect = () => invoke<SpotifyConnection>('spotify_disconnect');
export const spotifyStart = () => invoke<void>('spotify_start');
export const onSpotifyPlayback = (callback: (state: SpotifyPlaybackState) => void) => listen<SpotifyPlaybackState>('spotify-playback', event => callback(event.payload));
export const onSpotifyArtwork = (callback: (artworkUrl: string) => void) => listen<string>('spotify-artwork', event => callback(event.payload));
export const onSpotifyConnection = (callback: (state: SpotifyConnection) => void) => listen<SpotifyConnection>('spotify-connection', event => callback(event.payload));

export const spotifyBrowse = (kind: SpotifyBrowseKind, id: string, offset = 0) => invoke<SpotifyBrowsePage>('spotify_browse', { kind, id, offset });
