<script lang="ts">
  import { listen } from '@tauri-apps/api/event';
  import { onMount, tick } from 'svelte';
  import { initializeUpdates } from './lib/state/updates.svelte';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { invoke } from '@tauri-apps/api/core';
  import TopNav from './lib/components/TopNav.svelte';
  import UpdateNotice from './lib/components/UpdateNotice.svelte';
  import FramePairing from './lib/components/FramePairing.svelte';
  import MovieHero, { type HeroSlide } from './lib/components/MovieHero.svelte';
  import BackdropRow from './lib/components/BackdropRow.svelte';
  import MediaRow from './lib/components/MediaRow.svelte';
  import VideoRow from './lib/components/VideoRow.svelte';
  import PlaylistRow from './lib/components/spotify/PlaylistRow.svelte';
  import ActivityCard from './lib/components/activity/ActivityCard.svelte';
  import AppDock from './lib/components/AppDock.svelte';
  import SettingsPanel from './lib/components/SettingsPanel.svelte';
  import WidgetHost from './lib/components/WidgetHost.svelte';
  import MusicHome from './lib/components/spotify/MusicHome.svelte';
  import NowPlaying from './lib/components/spotify/NowPlaying.svelte';
  import { spotify } from './lib/state/spotify.svelte';
  import DetailsDialog from './lib/components/DetailsDialog.svelte';
  import StremioRow from './lib/components/StremioRow.svelte';
  import StremioLibrary from './lib/components/StremioLibrary.svelte';
  import StremioIntegration from './lib/components/StremioIntegration.svelte';
  import ContinueWatching from './lib/components/activity/ContinueWatching.svelte';
  import RecentlyPlayed from './lib/components/activity/RecentlyPlayed.svelte';
  import RecentApps from './lib/components/activity/RecentApps.svelte';
  import { createSettings } from './lib/state/settings.svelte';
  import { createActivity } from './lib/state/activity.svelte';
  import { createNavigation } from './lib/services/navigation';
  import { smoothHeight } from './lib/services/motion';
  import { applyTheme, appearanceFor, themeById } from './lib/services/theme';
  import { appProviders, actionLabel, isContinueWatching, progressLabel, providerName, sortRecent, typeLabel } from './lib/services/activity';
  import { native, launchApp, closeLaunchedApp, quitLuma, hideToTray, saveAppPaths, onNavigation, onOpenMusic, openYouTube, openYouTubeVideo, youtubeFeed, onYouTubeFeed, stremioStatus, stremioSync, stremioOpen, stremioOpenDetail } from './lib/services/tauri';
  import { loadCatalog, searchMovies } from './lib/services/catalog';
  import type { ActivityItem, AppId, ExecutableAppId, HomeRowId, MediaItem, StremioItem, StremioStatus, YouTubeFeedItem } from './lib/types';
  import type { MusicCardItem } from './lib/types/spotify';

  const preferences = createSettings();
  const settings = preferences.value;
  const activity = createActivity();
  const activityItems = $derived(activity.items);
  let tab = $state('Home');
  let panel = $state(false);
  let remotePairing = $state(false);
  let exitRequested = $state(false);
  let settingsCategory = $state('Display');
  let details = $state<MediaItem | null>(null);
  let message = $state(preferences.error);
  let movieQuery = $state('');
  let movieResults = $state.raw<MediaItem[]>([]);
  let searching = $state(false);
  let searchStatus = $state('');
  let searchRequest = 0;
  async function searchWatch(event: SubmitEvent) {
    event.preventDefault();
    const request = ++searchRequest;
    if (!movieQuery.trim()) { movieResults = []; searchStatus = ''; searching = false; return; }
    searching = true; movieResults = []; searchStatus = '';
    try {
      const results = await searchMovies(movieQuery);
      if (request === searchRequest) { movieResults = results; searchStatus = results.length ? '' : 'No movies found. Try another title.'; }
    } catch { if (request === searchRequest) searchStatus = 'Search could not load. Check your connection and try again.'; }
    finally { if (request === searchRequest) searching = false; }
  }
  let popular = $state.raw<MediaItem[]>([]);
  let featured = $state.raw<MediaItem[]>([]);
  let playerOpen = $state(false);
  let musicHome = $state<MusicHome>();
  let player = $state<NowPlaying>();
  let launching = $state<AppId | null>(null);
  let stremioPairing = $state(false);
  let stremio = $state.raw<StremioStatus | null>(null);
  let previewApp = $state<AppId | null>(null);
  let heroPick = $state<HeroSlide | null>(null);
  let youTubeFeed = $state.raw<YouTubeFeedItem[]>([]);
  let previous: HTMLElement | null = null;
  let playerPrevious: HTMLElement | null = null;
  let stremioPreviewSyncedAt = 0;
  const navigation = createNavigation(back, home, () => settings.homeShortcut);
  const showHome = $derived(tab === 'Home');
  const theme = $derived(themeById(settings.theme));
  const rowVisible = (id: HomeRowId) => settings.home.some(row => row.id === id && row.visible);
  const visibleHomeRows = $derived(settings.home.filter(row => row.visible));
  const catalogVisible = $derived(rowVisible('popular') || rowVisible('featured'));
  const previewProvider = $derived(previewApp ? appProviders[previewApp] : null);
  const stremioRecent = $derived(stremio?.items.filter(item => (item.lastWatched || item.timeOffset > 0) && item.kind !== 'other') ?? []);
  const previewActivity = $derived(previewProvider ? activityItems.filter(item => item.provider === previewProvider && item.type !== 'application').slice(0, 8) : []);
  const previewMusic = $derived(spotify.home.recentlyPlayed.map((item, index) => ({ id: `${item.id ?? item.title}-${index}`, title: item.title, uri: item.uri, artworkUrl: item.artworkUrl, subtitle: item.artist })));
  const continueItems = $derived(rowVisible('continue') ? sortRecent(activityItems.filter(item => item.type !== 'application' && isContinueWatching(item))) : []);
  const appHeroItems = $derived.by(() => {
    const app = previewApp;
    if (!app) return [];
    const own = sortRecent(activityItems.filter(item => item.provider === appProviders[app] && item.type !== 'application'));
    const resumable = own.filter(isContinueWatching);
    return resumable.length ? resumable : own;
  });
  const spotifyHero = $derived(previewApp === 'Spotify' && !appHeroItems.length ? previewMusic.slice(0, 5) : []);
  const stremioHero = $derived(previewApp === 'Stremio' ? (stremioRecent.length ? stremioRecent : stremio?.items ?? []).slice(0, 5) : []);
  function formatRuntime(value: string) {
    const minutes = Number.parseInt(value, 10);
    if (!Number.isFinite(minutes) || minutes <= 0) return value;
    return minutes >= 60 ? `${Math.floor(minutes / 60)}h ${minutes % 60}m` : `${minutes}m`;
  }
  function activitySlide(item: ActivityItem, eyebrow: string): HeroSlide {
    const progress = progressLabel(item);
    const label = actionLabel(item);
    return {
      key: `activity:${item.id}`,
      eyebrow,
      title: item.title,
      chips: [
        { label: providerName(item.provider) },
        { label: typeLabel(item) },
        ...(progress ? [{ label: progress }] : []),
      ],
      description: item.subtitle ?? '',
      art: item.artwork ?? '',
      primaryLabel: label,
      primaryIcon: label === 'Resume' ? 'play' : 'open',
      primary: () => { void openItem(item); },
    };
  }
  function activityEyebrow(item: ActivityItem) {
    const provider = providerName(item.provider).toUpperCase();
    if (previewApp && appProviders[previewApp] === item.provider) return `FROM ${provider}`;
    return isContinueWatching(item) ? 'CONTINUE WATCHING' : `FROM ${provider}`;
  }
  function movieSlide(item: MediaItem, eyebrow = 'FEATURED MOVIE'): HeroSlide {
    const genres = item.subtitle.split(' · ').filter(part => part !== item.year).slice(0, 2);
    return {
      key: `movie:${item.id}`,
      eyebrow,
      title: item.title,
      chips: [
        ...genres.map(label => ({ label })),
        ...(item.rating ? [{ label: `★ ${item.rating}`, tone: 'rating' as const }] : []),
        ...(item.runtime ? [{ label: formatRuntime(item.runtime) }] : []),
        ...(item.year ? [{ label: item.year }] : []),
      ],
      description: item.description,
      art: item.backdrop || item.poster,
      primaryLabel: 'Play',
      primary: () => { void openStremioDetail('movie', item.id); },
      details: () => select(item),
    };
  }
  function musicSlide(track: { id: string; title: string; uri?: string; artworkUrl?: string; subtitle?: string }): HeroSlide {
    return {
      key: `music:${track.id}`,
      eyebrow: 'FROM SPOTIFY',
      title: track.title,
      chips: [...(track.subtitle ? [{ label: track.subtitle }] : []), { label: 'Music' }],
      description: '',
      art: track.artworkUrl ?? '',
      artShape: 'square',
      primaryLabel: 'Play',
      primary: () => { if (track.uri) void spotify.playUri(track.uri).then(played => { if (played) openPlayer(); }); },
    };
  }
  function stremioSlide(item: StremioItem): HeroSlide {
    const label = item.kind === 'series' ? 'Series' : item.kind === 'movie' ? 'Movie' : 'Video';
    const remaining = item.duration > item.timeOffset ? `${Math.max(1, Math.round((item.duration - item.timeOffset) / 60_000))}m left` : '';
    return {
      key: `stremio:${item.id}`,
      eyebrow: 'FROM STREMIO',
      title: item.name,
      chips: [{ label: 'Stremio' }, { label }, ...(remaining ? [{ label: remaining }] : [])],
      description: '',
      art: item.poster,
      artShape: 'portrait',
      primaryLabel: 'Open Stremio',
      primaryIcon: 'open',
      primary: () => { void openStremio(item.id); },
    };
  }
  function videoSlide(item: YouTubeFeedItem): HeroSlide {
    return {
      key: `youtube:${item.id}`,
      eyebrow: 'FROM YOUTUBE',
      title: item.title,
      chips: [{ label: 'YouTube' }, { label: 'Video' }],
      description: '',
      art: `https://i.ytimg.com/vi/${item.id}/hqdefault.jpg`,
      primaryLabel: 'Open video',
      primaryIcon: 'open',
      primary: () => { void openYouTubeItem(item.id); },
    };
  }
  function previewActivityItem(item: ActivityItem | null) { heroPick = item ? activitySlide(item, activityEyebrow(item)) : null; }
  function previewMovie(item: MediaItem | null, eyebrow?: string) { heroPick = item ? movieSlide(item, eyebrow) : null; }
  function previewStremio(item: StremioItem | null) { heroPick = item ? stremioSlide(item) : null; }
  function previewVideo(item: YouTubeFeedItem | null) { heroPick = item ? videoSlide(item) : null; }
  function previewMusicItem(item: MusicCardItem | null) { heroPick = item ? musicSlide(item) : null; }
  const heroSource = $derived(heroPick ? `pick:${heroPick.key}` : previewApp && (appHeroItems.length || spotifyHero.length || stremioHero.length) ? `app:${previewApp}` : continueItems.length ? 'continue' : 'catalog');
  const heroSlides = $derived.by<HeroSlide[]>(() => {
    if (heroPick) return [heroPick];
    if (previewApp) {
      const eyebrow = `FROM ${providerName(appProviders[previewApp]).toUpperCase()}`;
      if (appHeroItems.length) return appHeroItems.slice(0, 5).map(item => activitySlide(item, eyebrow));
      if (spotifyHero.length) return spotifyHero.map(musicSlide);
      if (stremioHero.length) return stremioHero.map(stremioSlide);
    }
    if (continueItems.length) return continueItems.slice(0, 5).map(item => activitySlide(item, 'CONTINUE WATCHING'));
    if (rowVisible('featured')) return featured.slice(0, 5).map(item => movieSlide(item));
    if (rowVisible('popular')) return popular.slice(0, 5).map(item => movieSlide(item));
    return [];
  });
  function preview(id: AppId) {
    const changed = previewApp !== id;
    previewApp = id;
    if (changed) heroPick = null;
    if (id === 'Spotify') void spotify.loadHome();
    if (id === 'Stremio' && changed && Date.now() - stremioPreviewSyncedAt > 60_000) {
      stremioPreviewSyncedAt = Date.now();
      void refreshStremio(true);
    }
  }

  function horizontalWheel(node: HTMLElement) {
    function wheel(event: WheelEvent) {
      const track = event.target instanceof Element ? event.target.closest<HTMLElement>('.app-track, .card-track, .activity-track, .video-track, .music-card-track') : null;
      if (!track || track.scrollWidth <= track.clientWidth || event.ctrlKey) return;
      const delta = Math.abs(event.deltaX) > Math.abs(event.deltaY) ? event.deltaX : event.deltaY;
      const max = track.scrollWidth - track.clientWidth;
      if ((delta < 0 && track.scrollLeft <= 0) || (delta > 0 && track.scrollLeft >= max - 1)) return;
      event.preventDefault();
      track.scrollLeft += delta;
    }
    node.addEventListener('wheel', wheel, { passive: false });
    return () => node.removeEventListener('wheel', wheel);
  }

  $effect(() => {
    applyTheme(theme.tokens);
    document.documentElement.dataset.theme = settings.theme;
    document.documentElement.dataset.density = settings.density;
    document.documentElement.dataset.appTileShape = settings.appTileShape;
  });
  $effect(() => {
    try { localStorage.setItem('luma-v1', JSON.stringify({ homeShortcut: settings.homeShortcut, shortcutVersion: 1, theme: settings.theme, density: settings.density, appTileShape: settings.appTileShape, fullscreen: settings.fullscreen, home: settings.home, widgets: settings.widgets, order: settings.order })); }
    catch { message = 'Preferences could not be saved. Free some local storage and try again.'; }
  });
  $effect(() => {
    if (!native) return;
    invoke('update_appearance', { appearance: appearanceFor(theme) }).catch(() => {});
  });
  $effect(() => {
    if (native) invoke('update_app_state', { view: panel ? 'Settings' : details ? 'Details' : tab, fullscreen: settings.fullscreen }).catch(error => message = String(error));
  });
  $effect(() => {
    if (activity.error) message = activity.error;
  });
  onMount(() => {
    void initializeUpdates();
    loadCatalog()
      .then(data => { popular = data.popular; featured = data.featured; })
      .catch(() => message = 'Movie catalog could not be loaded. Check your internet connection.');
    let disposed = false;
    const cleanup: (() => void)[] = [];
    function track(stop: () => void) { if (disposed) stop(); else cleanup.push(stop); }
    async function start() {
      if (!native) { message = 'Browser preview. Native app launching, fullscreen, web remote, and Spotify controls require the desktop app.'; navigation.focus(document.querySelector<HTMLElement>('#home-button') ?? undefined); return; }
      try {
        const stopMusic = await onOpenMusic(() => { if (!disposed) void openMusic(); });
        if (disposed) { stopMusic(); return; }
        track(stopMusic);
        track(await listen<string>('tv-setup-error', event => message = event.payload));
        track(await listen('stremio-closed', () => { if (!disposed) void refreshStremio(true); }));
        const stopNavigation = await onNavigation(navigation.dispatch);
        if (disposed) { stopNavigation(); return; }
        track(stopNavigation);
        track(await listen('exit-requested', () => {
          if (disposed) return;
          previous = document.activeElement as HTMLElement;
          exitRequested = true;
        }));
        const [stopActivity] = await Promise.all([
          activity.start(),
          stremioStatus().then(value => { if (!disposed) stremio = value; }).catch(error => { if (!disposed) message = String(error); }),
          invoke<Record<ExecutableAppId, string>>('app_paths').then(paths => { if (!disposed) settings.paths = paths; }).catch(error => { if (!disposed) message = String(error); }),
        ]);
        track(stopActivity);
        await getCurrentWindow().setFullscreen(settings.fullscreen);
        track(await getCurrentWindow().onResized(async () => { const value = await getCurrentWindow().isFullscreen(); if (value !== settings.fullscreen) settings.fullscreen = value; }));
        if (!disposed) void hydrate();
      } catch (error) { message = String(error); }
      finally { if (disposed) cleanup.splice(0).forEach(stop => stop()); }
    }
    async function hydrate() {
      try {
        track(await onYouTubeFeed(items => { if (!disposed) youTubeFeed = items; }));
        youtubeFeed().then(items => { if (!disposed) youTubeFeed = items; }).catch(() => {});
        await spotify.initialize();
        track(() => spotify.dispose());
        if (spotify.connection.installed && await getCurrentWindow().isVisible()) void spotify.start();
        const tvWarning = await invoke<string | null>('tv_setup_warning');
        if (tvWarning && !disposed) message = tvWarning;
        await detect(false);
      } catch (error) { if (!disposed) message = String(error); }
    }
    async function reveal() {
      try { await start(); }
      finally {
        await tick();
        document.getElementById('startup')?.remove();
        document.getElementById('app')?.removeAttribute('inert');
        if (!disposed) navigation.focus(document.querySelector<HTMLElement>('#home-button') ?? undefined);
      }
    }
    void reveal();
    return () => { disposed = true; cleanup.splice(0).forEach(stop => stop()); };
  });
  function rememberFocus() { previous = document.activeElement as HTMLElement; }
  function back() {
    if (exitRequested) { cancelExit(); return; }
    if (remotePairing) { closeRemotePairing(); return; }
    if (stremioPairing) { closeStremioPairing(); return; }
    if (panel) { panel = false; void refreshStremio(); }
    else if (details) details = null;
    else if (playerOpen) { if (!player?.closeOverlay()) closePlayer(); return; }
    else if (tab === 'Music' && musicHome?.closeDetail()) return;
    else { home(); return; }
    requestAnimationFrame(() => navigation.focus(previous ?? undefined));
  }
  function cancelExit() {
    exitRequested = false;
    requestAnimationFrame(() => navigation.focus(previous ?? undefined));
  }
  function keepRunning() {
    exitRequested = false;
    void hideToTray().catch(() => {});
  }
  function confirmQuit() { void quitLuma().catch(() => {}); }
  function mountExitDialog(dialog: HTMLDialogElement) {
    dialog.showModal();
    requestAnimationFrame(() => dialog.querySelector<HTMLButtonElement>('button')?.focus());
  }
  function leaveApp() { if (native) void closeLaunchedApp().catch(() => {}); }
  function home() { leaveApp(); remotePairing = false; previewApp = null; heroPick = null; playerOpen = false; panel = false; details = null; tab = 'Home'; requestAnimationFrame(() => { window.scrollTo(0, 0); navigation.focus(document.querySelector<HTMLElement>('#home-button') ?? undefined); }); }
  function openPlayer() {
    playerPrevious = document.activeElement as HTMLElement;
    playerOpen = true;
    requestAnimationFrame(() => navigation.focus(document.querySelector<HTMLElement>('.now-playing-close') ?? undefined));
  }
  function closePlayer() {
    playerOpen = false;
    requestAnimationFrame(() => navigation.focus(playerPrevious ?? undefined));
  }
  function keydown(event: KeyboardEvent) {
    if (event.target instanceof HTMLInputElement && event.target.type === 'range') {
      if (event.key === 'ArrowUp' || event.key === 'ArrowDown') {
        event.preventDefault();
        navigation.dispatch(event.key === 'ArrowUp' ? 'up' : 'down');
        return;
      }
      if (event.key === 'Backspace') {
        event.preventDefault();
        navigation.dispatch('back');
        return;
      }
    }
    if (event.key === 'F11' && !event.repeat) { event.preventDefault(); void fullscreen(); return; }
    if (event.key === ' ' && !event.repeat && !event.altKey && !event.ctrlKey && !event.metaKey && !(event.target instanceof HTMLInputElement) && !(event.target instanceof HTMLTextAreaElement) && !(event.target instanceof HTMLSelectElement) && !(event.target instanceof HTMLButtonElement) && !(event.target instanceof HTMLAnchorElement)) {
      event.preventDefault();
      if (spotify.playback.canToggle) void spotify.action('toggle');
      return;
    }
    navigation.keydown(event);
  }
  const tabOrder = ['Home', 'Watch', 'Library', 'Music', 'Apps'];
  let previousTab = 0;
  function animateView(view: string) {
    const index = Math.max(0, tabOrder.indexOf(view));
    const direction = index < previousTab ? -1 : 1;
    previousTab = index;
    return (element: HTMLElement) => {
      if (window.matchMedia('(prefers-reduced-motion: reduce)').matches) return;
      const animation = element.animate(
        [{ opacity: .35, transform: `translate3d(${direction * 28}px, 6px, 0)` }, { opacity: 1, transform: 'none' }],
        { duration: 220, easing: 'cubic-bezier(.22,.8,.24,1)' },
      );
      return () => animation.cancel();
    };
  }
  function navigate(value: string) { if (value === 'Music') { void openMusic(); return; } if (value === 'Home') leaveApp(); previewApp = null; heroPick = null; tab = value; requestAnimationFrame(() => window.scrollTo(0, 0)); }
  async function openMusic() {
    while (musicHome?.closeDetail()) {}
    previewApp = null;
    heroPick = null;
    panel = false;
    details = null;
    playerOpen = false;
    message = '';
    tab = 'Music';
    await tick();
    if (tab === 'Music' && !panel && !details && !playerOpen) {
      window.scrollTo(0, 0);
      navigation.focusMusic();
    }
  }
  function openRemotePairing() { rememberFocus(); remotePairing = true; }
  function closeRemotePairing() {
    remotePairing = false;
    requestAnimationFrame(() => navigation.focus(previous ?? undefined));
  }
  function mountRemoteDialog(dialog: HTMLDialogElement) { dialog.showModal(); }
  function openSettings() { showSettings('Display'); }
  function openApps() { showSettings('Apps'); }
  function openStremioPairing() { rememberFocus(); stremioPairing = true; }
  function closeStremioPairing() {
    stremioPairing = false;
    void refreshStremio();
    requestAnimationFrame(() => navigation.focus(previous ?? undefined));
  }
  function mountStremioDialog(dialog: HTMLDialogElement) { dialog.showModal(); }
  function openIntegrations() { showSettings('Integrations'); }
  function showSettings(category: string) { previewApp = null; rememberFocus(); message = ''; settingsCategory = category; panel = true; }
  function select(item: MediaItem) { rememberFocus(); message = ''; details = item; }
  async function launch(id: AppId) {
    if (id === 'Spotify') {
      await openMusic();
      return;
    }
    if (launching) return;
    if (!native) { message = 'App launching is available in the Luma desktop app.'; return; }
    launching = id;
    try {
      if (id === 'YouTube') { await openYouTube(settings.fullscreen); message = 'YouTube TV opened in its own window. Use Back to Luma when you are done.'; }
      else { await launchApp(id); message = id === 'Stremio' ? 'Stremio opened inside Luma. Use Back to Luma when you are done.' : id === 'Steam' ? 'Steam Big Picture opened. Press Home to return to Luma.' : `${id} opened. Press Home to return to Luma.`; }
    } catch (error) { message = String(error); }
    finally { launching = null; }
  }
  async function openItem(item: ActivityItem) {
    if (!native) { message = 'Opening activity is available in the Luma desktop app.'; return; }
    try { message = (await activity.open(item.id, settings.fullscreen)).message; }
    catch (error) { message = String(error); }
  }
  async function removeItem(item: ActivityItem) {
    if (!native) return;
    try {
      await activity.remove(item.id);
      message = `${item.title} removed from watch history.`;
    } catch (error) { message = String(error); }
  }
  async function fullscreen() {
    try { const value = !await getCurrentWindow().isFullscreen(); await getCurrentWindow().setFullscreen(value); settings.fullscreen = value; }
    catch (error) { message = String(error); }
  }
  async function detect(announce = true) {
    try {
      const apps = await invoke<{ name: ExecutableAppId; path: string | null }[]>('detect_apps');
      for (const app of apps) if (app.path && !settings.paths[app.name]) settings.paths[app.name] = app.path;
      if (announce) {
        const found = apps.filter(app => app.path).length;
        message = found
          ? `${found} app${found === 1 ? '' : 's'} found. Choose Save app settings to keep them. If an app you use is missing, choose it under App locations.`
          : 'No installed apps were found automatically. You can choose an app under App locations.';
      }
    } catch (error) { message = String(error); }
  }
  async function savePaths() {
    try { await saveAppPaths(settings.paths); message = 'App settings saved.'; }
    catch (error) { message = String(error); }
  }
  let refreshingStremio = false;
  async function refreshStremio(sync = false) {
    if (!native || refreshingStremio) return;
    refreshingStremio = true;
    try {
      stremio = await stremioStatus();
      if (sync && stremio.connected) stremio = await stremioSync();
    } catch (error) { message = String(error); }
    finally { refreshingStremio = false; }
  }
  async function syncStremio() {
    stremio = await stremioSync();
    message = 'Your Stremio library is up to date.';
  }
  async function openYouTubeItem(id: string) {
    if (!native) { message = 'Opening YouTube is available in the desktop app.'; return; }
    try { await openYouTubeVideo(id, settings.fullscreen); message = 'Opening in YouTube. Press Ctrl+W there to return.'; }
    catch (error) { message = String(error); }
  }
  async function openStremio(id: string) {
    if (!native) { message = 'Opening Stremio is available in the desktop app.'; return; }
    try { await stremioOpen(id); message = 'Opening in Stremio inside Luma.'; }
    catch (error) { message = String(error); }
  }
  async function openStremioDetail(kind: string, id: string) {
    if (!native) { message = 'Opening Stremio is available in the desktop app.'; return; }
    try { await stremioOpenDetail(kind, id); message = 'Opening in Stremio inside Luma.'; }
    catch (error) { message = String(error); }
  }
</script>

<svelte:window onkeydown={keydown} />
<main class="app-shell" class:is-home={showHome} {@attach horizontalWheel}>
  {#if panel}<SettingsPanel {message} {settings} {native} {activity} startCategory={settingsCategory} close={back} {fullscreen} {savePaths} {detect} popular={popular} featured={featured} />
  {:else}
  <TopNav active={tab} {navigate} settings={openSettings} connect={openRemotePairing} fullscreen={settings.fullscreen} toggle={fullscreen} />
  <div class="topbar-offset" aria-hidden="true"></div>
  <div class="view-content" {@attach animateView(tab)}>
  {#if showHome && heroSlides.length}
    <MovieHero slides={heroSlides} sourceKey={heroSource} />
  {/if}
  {#if showHome}
    <div {@attach smoothHeight}><div class="resizing-content">
      {#each visibleHomeRows as row (row.id)}
        {#if row.id === 'continue'}
          {#if previewApp === 'Stremio'}
            <div class="view-swap"><StremioRow title="Recently watched on Stremio" items={stremioRecent.slice(0, 4)} open={openStremio} connected={stremio?.connected ?? false} connect={openStremioPairing} onpreview={previewStremio} cinematic /></div>
          {:else}
            <ContinueWatching items={activityItems} trackingEnabled={activity.trackingEnabled} onopen={openItem} focusProvider={previewApp ? appProviders[previewApp] : null} onpreview={previewActivityItem} />
          {/if}
        {:else if row.id === 'apps'}
          <AppDock order={settings.order} {launch} {launching} {preview} active={previewApp} />
          {#if previewApp && catalogVisible}
            {#key previewApp}
              <div class="view-swap">
                {#if previewApp === 'YouTube' && youTubeFeed.length}
                  <VideoRow items={youTubeFeed} open={openYouTubeItem} onpreview={previewVideo} />
                {:else if previewApp === 'Spotify' && previewMusic.length}
                  <PlaylistRow title="Recently played on Spotify" items={previewMusic} empty="" kind="Track" onselect={async item => { if (await spotify.playUri(item.uri)) openPlayer(); }} onpreview={previewMusicItem} />
                {:else if previewApp === 'Stremio' && stremio?.items.length}
                  <StremioRow title="Your Stremio library" items={stremio.items.slice(0, 10)} open={openStremio} onpreview={previewStremio} />
                {:else if previewActivity.length}
                  <section class="content-row"><div class="row-heading"><h2>From {previewApp}</h2></div><div class="activity-track">{#each previewActivity as item (item.id)}<ActivityCard {item} onopen={openItem} onremove={removeItem} onpreview={previewActivityItem} />{/each}</div></section>
                {:else}
                  <section class="content-row" aria-label={`${previewApp} recommendations`}><div class="row-heading"><h2>From {previewApp === 'Custom' ? 'your custom app' : previewApp}</h2></div><div class="activity-empty"><strong>{previewApp === 'Spotify' && spotify.homeLoading ? 'Loading your music...' : 'No recommendations available yet'}</strong><span>{previewApp === 'YouTube' ? 'Open YouTube to load its recommendations.' : 'Content from this app will appear here when available.'}</span><button class="text-link" onclick={() => previewApp && launch(previewApp)}>Open {previewApp === 'Custom' ? 'custom app' : previewApp}</button></div></section>
                {/if}
              </div>
            {/key}
          {/if}
        {:else if row.id === 'popular'}
          {#if !previewApp}<BackdropRow id="popular" title="Popular movies" items={popular} onselect={select} onpreview={item => previewMovie(item, 'POPULAR MOVIE')} />{/if}
        {:else if row.id === 'featured'}
          {#if !previewApp}<BackdropRow id="featured" title="Top-rated movies" items={featured} onselect={select} onpreview={item => previewMovie(item, 'TOP-RATED MOVIE')} />{/if}
        {:else if row.id === 'recent'}
          <RecentlyPlayed items={activityItems} trackingEnabled={activity.trackingEnabled} onopen={openItem} onremove={removeItem} onpreview={previewActivityItem} />
        {:else if row.id === 'recentApps'}
          <RecentApps items={activityItems} trackingEnabled={activity.trackingEnabled} onopen={openItem} onremove={removeItem} onpreview={previewActivityItem} />
        {/if}
      {/each}
    </div></div>
  {/if}
  {#if tab === 'Library'}<StremioLibrary status={stremio} {native} open={openStremio} connect={openStremioPairing} sync={syncStremio} />{/if}
  {#if tab === 'Apps'}
    <header class="page-head">
      <div><span class="eyebrow">ON THIS PC</span><h1>Apps</h1><p>Open your streaming apps and keep the ones you use most in reach.</p></div>
      <button class="text-link" onclick={openApps}>Edit apps <span>→</span></button>
    </header>
    <AppDock order={settings.order} {launch} {launching} {preview} active={previewApp} />
  {/if}
  {#if tab === 'Watch'}
    <header class="page-head">
      <div><span class="eyebrow">STREMIO CATALOG</span><h1>Watch</h1><p>Search for a title or browse what is popular right now.</p></div>
    </header>
    <section class="content-row" aria-label="Search movies">
      <form onsubmit={searchWatch} class="movie-search-field" role="search">
        <svg class="movie-search-icon" viewBox="0 0 24 24" aria-hidden="true"><circle cx="11" cy="11" r="7" /><path d="m20 20-3.5-3.5" /></svg>
        <input id="movie-search" type="search" bind:value={movieQuery} placeholder="Search by movie title" aria-label="Search movies" autocomplete="off" spellcheck="false" />
        <button type="submit" class="movie-search-submit">Search</button>
      </form>
      <p class="movie-search-status" role="status">{searching ? 'Searching movies…' : searchStatus}</p>
    </section>
    {#if movieResults.length}
      {#key movieResults}
        <div class="view-swap"><MediaRow id="search-results" title="Search results" items={movieResults} onselect={item => openStremioDetail('movie', item.id)} /></div>
      {/key}
    {/if}
    <div {@attach smoothHeight}><div class="resizing-content">
      <MediaRow id="popular" title="Popular movies" items={popular} onselect={select} />
      <MediaRow id="featured" title="Top-rated movies" items={featured} onselect={select} />
    </div></div>
    <ContinueWatching items={activityItems} trackingEnabled={activity.trackingEnabled} onopen={openItem} />
    <RecentlyPlayed items={activityItems} trackingEnabled={activity.trackingEnabled} onopen={openItem} onremove={removeItem} onpreview={previewActivityItem} />
  {/if}
  {#if tab === 'Music'}<MusicHome bind:this={musicHome} onopen={openPlayer} onsettings={openIntegrations} />{/if}
  {#if tab === 'Apps'}
    <RecentApps items={activityItems} trackingEnabled={activity.trackingEnabled} onopen={openItem} onremove={removeItem} onpreview={previewActivityItem} />
  {/if}
  </div>
  <div class="notice-stack">
    {#if message && !details}<div role="status" class="notice"><span>{message}</span><button aria-label="Dismiss message" onclick={() => message = ''}>×</button></div>{/if}
    <UpdateNotice {native} />
  </div>
  {#if !playerOpen}<WidgetHost widgets={settings.widgets} onopen={openPlayer} />{/if}
  {#if playerOpen}<NowPlaying bind:this={player} onclose={closePlayer} />{/if}
  {/if}
</main>
{#if details}<DetailsDialog {message} item={details} close={back} launch={() => details && openStremioDetail('movie', details.id)} />{/if}

{#if stremioPairing}
  <dialog {@attach mountStremioDialog} class="details-dialog" aria-label="Connect Stremio" oncancel={(event) => { event.preventDefault(); closeStremioPairing(); }}>
    <button class="close-btn float-right mb-4" onclick={closeStremioPairing} aria-label="Close Stremio connection">?</button>
    <StremioIntegration {native} autoConnect onconnected={closeStremioPairing} />
  </dialog>
{/if}

{#if remotePairing}
  <dialog {@attach mountRemoteDialog} class="remote-dialog" aria-label="Connect a remote" oncancel={(event) => { event.preventDefault(); closeRemotePairing(); }}>
    <div class="remote-dialog-heading"><div><span class="eyebrow">REMOTE CONTROL</span><h2>Connect to Luma</h2></div><button class="close-btn" aria-label="Close connection" onclick={closeRemotePairing}>&times;</button></div>
    <FramePairing {native} autoConnect pairingOnly />
    <button class="panel-secondary" onclick={closeRemotePairing}>Done</button>
  </dialog>
{/if}

{#if exitRequested}
  <dialog {@attach mountExitDialog} class="exit-dialog" aria-labelledby="exit-title" oncancel={(event) => { event.preventDefault(); cancelExit(); }}>
    <h2 id="exit-title">Exit Luma?</h2>
    <p>Keep Luma running in the notification area so the Display Box and web remote stay connected, or quit completely and stop the server.</p>
    <div class="exit-actions">
      <button class="panel-secondary exit-keep" onclick={keepRunning}>Keep running in tray</button>
      <button class="panel-secondary" onclick={confirmQuit}>Quit Luma</button>
    </div>
  </dialog>
{/if}

<style>
  .resizing-content { display: flow-root; }
  .remote-dialog { width: min(820px, calc(100vw - 48px)); max-height: calc(100dvh - 48px); margin: auto; padding: clamp(24px, 3vw, 40px); overflow-y: auto; border: 1px solid #ffffff26; border-radius: 24px; background: #15171e; color: var(--text); box-shadow: 0 24px 90px #0008; }
  .remote-dialog::backdrop { background: #05060aaa; backdrop-filter: blur(6px); }
  .remote-dialog-heading { display: flex; align-items: center; justify-content: space-between; gap: 24px; }
  .remote-dialog-heading h2 { margin: 8px 0 0; font-size: clamp(26px, 2.4vw, 36px); }
  .exit-dialog { width: min(560px, calc(100vw - 48px)); margin: auto; padding: clamp(26px, 3vw, 38px); border: 1px solid #ffffff26; border-radius: 22px; background: #15171e; color: var(--text); box-shadow: 0 24px 90px #0008; }
  .exit-dialog::backdrop { background: #05060aaa; backdrop-filter: blur(6px); }
  .exit-dialog h2 { margin: 0 0 12px; font-size: clamp(24px, 2.2vw, 32px); letter-spacing: -.03em; }
  .exit-dialog p { margin: 0 0 24px; color: #a9aab5; font-size: clamp(13px, .95vw, 16px); line-height: 1.6; }
  .exit-actions { display: flex; gap: 12px; justify-content: flex-end; flex-wrap: wrap; }
  .exit-actions button { width: auto; min-height: 48px; margin: 0; padding: 0 22px; }
  .exit-actions .exit-keep { border-color: transparent; background: var(--accent); color: #10131b; }
</style>
