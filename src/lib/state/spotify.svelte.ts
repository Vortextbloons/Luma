import { native } from '../services/tauri';
import { appVisible, onAppVisible } from '../services/visibility';
import {
  onSpotifyArtwork,
  onSpotifyConnection,
  onSpotifyPlayback,
  spotifyBrowse,
  spotifyDevices,
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
  SpotifyDevice,
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
  let artwork = $state('');
  let connection = $state<SpotifyConnection>(emptyConnection());
  let home = $state.raw<SpotifyHomeData>(emptyHome());
  let queueError = $state('');
  let devices = $state.raw<SpotifyDevice[]>([]);
  let devicesLoading = $state(false);
  let devicesError = $state('');
  let queue = $state.raw<SpotifyQueue | null>(null);
  let clientId = $state('');
  let loading = $state(false);
  let homeLoading = $state(false);
  let busy = $state(false);
  let connecting = $state(false);
  let error = $state('');
  let tick = $state(Date.now());
  let initialized = false;
  let homeLoadedAt = 0;
  let accountRevision = 0;
  const stops: (() => void)[] = [];
  let timer: ReturnType<typeof setInterval> | undefined;
  let stopVisibility: (() => void) | undefined;

  function resetAccountData() {
    accountRevision++;
    home = emptyHome();
    homeLoadedAt = 0;
    homeLoading = false;
    queue = null;
    queueError = '';
    devices = [];
    devicesError = '';
    devicesLoading = false;
  }

  function applyConnection(state: SpotifyConnection) {
    if (state.authenticated !== connection.authenticated || state.displayName !== connection.displayName) resetAccountData();
    connection = state;
  }

  function updateTimer() {
    const active = playback.playing && appVisible();
    if (active && !timer) {
      tick = Date.now();
      timer = setInterval(() => { tick = Date.now(); }, 1000);
    } else if (!active && timer) {
      clearInterval(timer);
      timer = undefined;
    }
  }

  async function initialize() {
    if (initialized) return;
    initialized = true;
    if (!native) return;
    stops.push(await onSpotifyPlayback(state => {
      if (state.title !== playback.title || state.artist !== playback.artist) artwork = '';
      playback = state;
      updateTimer();
    }));
    stops.push(await onSpotifyArtwork(value => { artwork = value; }));
    stops.push(await onSpotifyConnection(applyConnection));
    stopVisibility = onAppVisible(updateTimer);
    try { clientId = await spotifyClientId(); } catch { clientId = ''; }
    await refresh();
  }

  function dispose() {
    resetAccountData();
    for (const stop of stops.splice(0)) stop();
    stopVisibility?.();
    stopVisibility = undefined;
    if (timer) clearInterval(timer);
    timer = undefined;
    initialized = false;
  }

  async function refresh() {
    if (!native) return;
    const revision = accountRevision;
    loading = true;
    try {
      const previous = playback;
      const [nextPlayback, nextConnection] = await Promise.all([spotifyPlayback(), spotifyStatus()]);
      if (revision !== accountRevision) return;
      playback = nextPlayback;
      applyConnection(nextConnection);
      updateTimer();
      if (playback.artworkUrl) artwork = playback.artworkUrl;
      else if (playback.title !== previous.title || playback.artist !== previous.artist) artwork = '';
      error = '';
    } catch (cause) {
      if (revision === accountRevision) error = String(cause);
    } finally {
      loading = false;
    }
  }

  async function loadHome(force = false) {
    if (!native || !connection.authenticated || connecting || homeLoading) return;
    if (!force && homeLoadedAt && Date.now() - homeLoadedAt < 60_000) return;
    const revision = accountRevision;
    homeLoading = true;
    try {
      const data = await spotifyHome();
      if (revision !== accountRevision) return;
      home = data;
      homeLoadedAt = Date.now();
      error = '';
    } catch (cause) {
      if (revision === accountRevision) error = String(cause);
    } finally {
      if (revision === accountRevision) homeLoading = false;
    }
  }

  async function action(name: string, value?: string) {
    if (busy) return false;
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

  async function playUri(uri?: string, contextUri?: string) {
    if (!uri || busy) return false;
    busy = true;
    try {
      await spotifyPlay(uri, undefined, contextUri);
      await refresh();
      error = '';
      return true;
    } catch (cause) {
      error = String(cause);
      return false;
    } finally { busy = false; }
  }

  async function transfer(deviceId?: string) {
    if (!deviceId || busy) return false;
    busy = true;
    try {
      await spotifyTransfer(deviceId);
      await refresh();
      error = '';
      return true;
    } catch (cause) {
      error = String(cause);
      return false;
    } finally { busy = false; }
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
    if (!native || connecting) return false;
    resetAccountData();
    connecting = true;
    error = '';
    try {
      applyConnection(await spotifyConnect());
      await refresh();
      connecting = false;
      void loadHome(true);
      return true;
    } catch (cause) {
      error = String(cause);
      return false;
    } finally {
      connecting = false;
    }
  }

  async function disconnect() {
    if (!native || connecting) return false;
    resetAccountData();
    connecting = true;
    try {
      applyConnection(await spotifyDisconnect());
      artwork = '';
      await refresh();
      error = '';
      return true;
    } catch (cause) {
      error = String(cause);
      return false;
    } finally {
      connecting = false;
    }
  }

  async function saveClientId(value: string) {
    try {
      await spotifySetClientId(value.trim());
      clientId = value.trim();
      connection.clientIdConfigured = true;
      applyConnection(await spotifyStatus());
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
    if (!native || !connection.authenticated || connecting) return;
    const revision = accountRevision;
    try {
      queueError = '';
      const data = await spotifyQueue();
      if (revision === accountRevision) queue = data;
    } catch (cause) {
      if (revision === accountRevision) queueError = String(cause);
    }
  }

  async function loadDevices() {
    if (!native || !connection.authenticated || connecting || devicesLoading) return;
    const revision = accountRevision;
    devicesLoading = true;
    devicesError = '';
    try {
      const data = await spotifyDevices();
      if (revision === accountRevision) devices = data;
    } catch (cause) {
      if (revision === accountRevision) devicesError = String(cause);
    } finally {
      if (revision === accountRevision) devicesLoading = false;
    }
  }

  return {
    browse: spotifyBrowse,
    loadDevices,
    get devices() { return devices; },
    get devicesLoading() { return devicesLoading; },
    get devicesError() { return devicesError; },
    get queueError() { return queueError; },
    get playback() { return { ...playback, artworkUrl: artwork || undefined }; },
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
