export interface SpotifyPlaybackState {
  connected: boolean;
  playerAvailable: boolean;
  playing: boolean;
  trackId?: string;
  title?: string;
  artist?: string;
  album?: string;
  artworkUrl?: string;
  positionMs?: number;
  durationMs?: number;
  deviceId?: string;
  deviceName?: string;
  canSeek: boolean;
  canSkipNext: boolean;
  canSkipPrevious: boolean;
  canToggle: boolean;
  shuffle?: boolean;
  repeat?: string;
  volume?: number;
  updatedAt: number;
  error?: string;
}

export interface SpotifyConnection {
  authenticated: boolean;
  processRunning: boolean;
  installed: boolean;
  playerAvailable: boolean;
  starting: boolean;
  clientIdConfigured: boolean;
  displayName?: string;
  error?: string;
}

export interface SpotifyTrack {
  id?: string;
  uri?: string;
  title: string;
  artist: string;
  album: string;
  artworkUrl?: string;
  durationMs?: number;
}

export interface SpotifyPlaylist {
  id: string;
  uri: string;
  name: string;
  description?: string;
  owner?: string;
  artworkUrl?: string;
  trackCount: number;
}

export interface SpotifyAlbum {
  id: string;
  uri: string;
  name: string;
  artist: string;
  artworkUrl?: string;
  year?: string;
}

export interface SpotifyDevice {
  id?: string;
  name: string;
  kind: string;
  active: boolean;
  restricted: boolean;
  volume?: number;
}

export interface SpotifyHomeData {
  authenticated: boolean;
  displayName?: string;
  recentlyPlayed: SpotifyTrack[];
  playlists: SpotifyPlaylist[];
  albums: SpotifyAlbum[];
  tracks: SpotifyTrack[];
  devices: SpotifyDevice[];
  warnings: string[];
}

export interface SpotifySearchResults {
  tracks: SpotifyTrack[];
  albums: SpotifyAlbum[];
  playlists: SpotifyPlaylist[];
}

export interface SpotifyQueue {
  currentlyPlaying?: SpotifyTrack;
  items: SpotifyTrack[];
}

export interface MusicCardItem {
  id: string;
  title: string;
  subtitle?: string;
  artworkUrl?: string;
  uri?: string;
}
