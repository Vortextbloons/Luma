import { native } from '../services/tauri';
import {
  onSpotifyConnection,
  onSpotifyPlayback,
  spotifyClientId,
  spotifyConnect,
  spotifyControl,
  spotifyDisconnect,
  spotifyHome,
  spotifyPlay,
  spotifyPlayback,
  spotifyQueue,
  spotifySearch,
  spotifySetClientId,
  spotifyStart,
  spotifyStatus,
  spotifyTransfer,
} from '../services/spotify';
import type {
  SpotifyConnection,
  SpotifyHomeData,
  SpotifyPlaybackState,
  SpotifyQueue,
  SpotifySearchResults,
} from '../types/spotify';

function emptyPlayback(): SpotifyPlaybackState {
  return {
    connected: false, playerAvailable: false, playing: false,
    canSeek: false, canSkipNext: false, canSkipPrevious: false, canToggle: false, updatedAt: 0,
  };
}

function emptyConnection(): SpotifyConnection {
  return {
    authenticated: false, processRunning: false, installed: false, playerAvailable: false,
    starting: false, clientIdConfigured: false,
  };
}

function emptyHome(): SpotifyHomeData {
  return {
    authenticated: false, recentlyPlayed: [], playlists: [], albums: [], tracks: [], devices: [], warnings: [],
  };
}

export function createSpotify() {
  let playback = $state<SpotifyPlaybackState>(emptyPlayback());
  let connection = $state<SpotifyConnection>(emptyConnection());
  let home = $state<SpotifyHomeData>(emptyHome());
  let queue = $state<SpotifyQueue | null>(null);
  let clientId = $state('');
  let loading = $state(false);
  let homeLoading = $state(false);
  let busy = $state(false);
  let connecting = $state(false);
  let error = $state('');
  let tick = $state(Date.now());
  let initialized = false;
  let homeLoadedAt = 0;
  const stops: (() => void)[] = [];
  let timer: ReturnType<typeof setInterval> | undefined;

  async function initialize() {
    if (initialized) return;
    initialized = true;
    if (!native) return;
    stops.push(await onSpotifyPlayback(state => { playback = state; }));
    stops.push(await onSpotifyConnection(state => { connection = state; }));
    timer = setInterval(() => {
      if (playback.playing) tick = Date.now();
    }, 500);
    try { clientId = await spotifyClientId(); } catch { clientId = ''; }
    await refresh();
  }

  function dispose() {
    for (const stop of stops.splice(0)) stop();
    if (timer) clearInterval(timer);
    timer = undefined;
  }

  async function refresh() {
    if (!native) return;
    loading = true;
    try {
      [playback, connection] = await Promise.all([spotifyPlayback(), spotifyStatus()]);
    } catch (cause) {
      error = String(cause);
    } finally {
      loading = false;
    }
  }

  async function loadHome(force = false) {
    if (!native || !connection.authenticated) return;
    if (!force && homeLoadedAt && Date.now() - homeLoadedAt < 60_000) return;
    homeLoading = true;
    try {
      home = await spotifyHome();
      homeLoadedAt = Date.now();
    } catch (cause) {
      error = String(cause);
    } finally {
      homeLoading = false;
    }
  }

  async function action(name: string, value?: string) {
    busy = true;
    try {
      await spotifyControl(name, value);
      await refresh();
      error = '';
      return true;
    } catch (cause) {
      error = String(cause);
      return false;
    } finally {
      busy = false;
    }
  }

  async function playUri(uri?: string) {
    if (!uri) return false;
    try {
      await spotifyPlay(uri);
      await refresh();
      error = '';
      return true;
    } catch (cause) {
      error = String(cause);
      return false;
    }
  }

  async function transfer(deviceId?: string) {
    if (!deviceId) return false;
    try {
      await spotifyTransfer(deviceId);
      await refresh();
      error = '';
      return true;
    } catch (cause) {
      error = String(cause);
      return false;
    }
  }

  async function start() {
    if (!native) return false;
    try {
      connection.starting = true;
      await spotifyStart();
      error = '';
      return true;
    } catch (cause) {
      connection.starting = false;
      error = String(cause);
      return false;
    }
  }

  async function connect() {
    if (!native) return false;
    connecting = true;
    error = '';
    try {
      connection = await spotifyConnect();
      await refresh();
      await loadHome(true);
      return true;
    } catch (cause) {
      error = String(cause);
      return false;
    } finally {
      connecting = false;
    }
  }

  async function disconnect() {
    if (!native) return false;
    try {
      connection = await spotifyDisconnect();
      home = emptyHome();
      queue = null;
      homeLoadedAt = 0;
      await refresh();
      error = '';
      return true;
    } catch (cause) {
      error = String(cause);
      return false;
    }
  }

  async function saveClientId(value: string) {
    try {
      await spotifySetClientId(value.trim());
      clientId = value.trim();
      connection = await spotifyStatus();
      error = '';
      return true;
    } catch (cause) {
      error = String(cause);
      return false;
    }
  }

  async function search(query: string): Promise<SpotifySearchResults> {
    if (!native || !query.trim()) return { tracks: [], albums: [], playlists: [] };
    return spotifySearch(query);
  }

  async function loadQueue() {
    if (!native || !connection.authenticated) return;
    try {
      queue = await spotifyQueue();
    } catch (cause) {
      error = String(cause);
    }
  }

  return {
    get playback() { return playback; },
    get connection() { return connection; },
    get home() { return home; },
    get queue() { return queue; },
    get clientId() { return clientId; },
    get loading() { return loading; },
    get homeLoading() { return homeLoading; },
    get busy() { return busy; },
    get connecting() { return connecting; },
    get error() { return error; },
    get positionMs() {
      const duration = playback.durationMs ?? 0;
      const elapsed = playback.playing ? Math.max(0, tick - (playback.updatedAt || tick)) : 0;
      const position = (playback.positionMs ?? 0) + elapsed;
      return duration > 0 ? Math.min(position, duration) : position;
    },
    get progress() {
      const duration = playback.durationMs ?? 0;
      const elapsed = playback.playing ? Math.max(0, tick - (playback.updatedAt || tick)) : 0;
      const position = (playback.positionMs ?? 0) + elapsed;
      return duration > 0 ? Math.min(1, position / duration) : 0;
    },
    initialize,
    dispose,
    refresh,
    loadHome,
    action,
    playUri,
    transfer,
    start,
    connect,
    disconnect,
    saveClientId,
    search,
    loadQueue,
    clearError() { error = ''; },
  };
}

export const spotify = createSpotify();
