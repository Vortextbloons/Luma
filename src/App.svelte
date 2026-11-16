<script lang="ts">
  import { listen } from '@tauri-apps/api/event';
  import { onMount, tick } from 'svelte';
  import { initializeUpdates } from './lib/state/updates.svelte';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { invoke } from '@tauri-apps/api/core';
  import TopNav from './lib/components/TopNav.svelte';
  import HomeRenderer from './lib/components/HomeRenderer.svelte';
  import LayoutReview from './lib/components/LayoutReview.svelte';
  import UpdateNotice from './lib/components/UpdateNotice.svelte';
  import FramePairing from './lib/components/FramePairing.svelte';
  import SearchSportsRow from './lib/components/SearchSportsRow.svelte';
  import ActivityCard from './lib/components/activity/ActivityCard.svelte';
  import MediaRow from './lib/components/MediaRow.svelte';
  import AppDock from './lib/components/AppDock.svelte';
  import SettingsPanel from './lib/components/SettingsPanel.svelte';
  import WidgetHost from './lib/components/WidgetHost.svelte';
  import MusicHome from './lib/components/spotify/MusicHome.svelte';
  import NowPlaying from './lib/components/spotify/NowPlaying.svelte';
  import SportsHome from './lib/components/SportsHome.svelte';
  import { spotify } from './lib/state/spotify.svelte';
  import DetailsDialog from './lib/components/DetailsDialog.svelte';
  import StremioLibrary from './lib/components/StremioLibrary.svelte';
  import StremioIntegration from './lib/components/StremioIntegration.svelte';
  import ContinueWatching from './lib/components/activity/ContinueWatching.svelte';
  import RecentlyPlayed from './lib/components/activity/RecentlyPlayed.svelte';
  import RecentApps from './lib/components/activity/RecentApps.svelte';
  import { createSettings } from './lib/state/settings.svelte';
  import type { HomeDraft } from './lib/services/customization';
  import { createActivity } from './lib/state/activity.svelte';
  import { createSports } from './lib/state/sports.svelte';
  import { createStremio } from './lib/state/stremio.svelte';
  import { createSteam } from './lib/state/steam.svelte';
  import { createYouTube } from './lib/state/youtube.svelte';
  import { createHome } from './lib/state/home.svelte';
  import { createNavigation } from './lib/services/navigation';
  import { smoothHeight } from './lib/services/motion';
  import { playShutdownSound, playSound, playStartupSound, setSoundSettings, soundPlayedRecently, unlockSound, warmSound } from './lib/services/sound';
  import { setAppVisible } from './lib/services/visibility';
  import { providerName } from './lib/services/activity';
  import { native, searchYouTube, launchApp, closeLaunchedApp, quitLuma, hideToTray, saveAppPaths, onNavigation, onOpenMusic, openYouTube } from './lib/services/tauri';
  import { searchMovies, searchShows } from './lib/services/catalog';
  import type { ActivityItem, AppId, ExecutableAppId, MediaItem } from './lib/types';
  import { loadSportsListing, type SportsEvent } from './lib/services/sports';
  import type { GameScore } from './lib/services/sportsScores';

  const preferences = createSettings();
  const settings = preferences.value;
  const activity = createActivity();
  const activityItems = $derived(activity.items);
  const sports = createSports();
  const stremioState = createStremio();
  const steamState = createSteam();
  const youtube = createYouTube();
  const layoutConfig = $derived(preferences.layout);
  const layoutPreview = $derived(preferences.preview);
  const homeState = createHome({
    layout: () => layoutConfig,
    activity, spotify, sports,
    stremio: stremioState,
    steam: steamState,
    youtube,
    actions: {
      launch: id => void launch(id),
      openItem: item => void openItem(item),
      openStremio: id => void openStremio(id),
      openStremioDetail: (kind, id) => void openStremioDetail(kind, id),
      openSteamGame: appid => void openSteamGame(appid),
      openSteamStore: appid => void openSteamStore(appid),
      openYouTubeItem: id => void openYouTubeItem(id),
      select: item => select(item),
      openPlayer: () => openPlayer(),
      watchLeague: game => void watchLeagueGame(game),
    },
  });
  let tab = $state('Home');
  let panel = $state(false);
  let remotePairing = $state(false);
  let exitRequested = $state(false);
  let settingsCategory = $state('Display');
  let details = $state<MediaItem | null>(null);
  let message = $state(preferences.error);
  let homeDraft = $state<HomeDraft | null>(null);
  let movieQuery = $state('');
  let movieResults = $state.raw<MediaItem[]>([]);
  let showResults = $state.raw<MediaItem[]>([]);
  let matchedSports = $state.raw<SportsEvent[]>([]);
  let submittedQuery = $state('');
  const matchesSearch = (title: string) => title.toLocaleLowerCase().includes(submittedQuery.toLocaleLowerCase());
  const searchHistory = $derived(submittedQuery ? activityItems.filter(item => matchesSearch(item.title)) : []);
  let searching = $state(false);
  let searchStatus = $state('');
  let searchRequest = 0;
  async function searchWatch(event: SubmitEvent) {
    event.preventDefault();
    await runSearch();
  }
  async function runSearch() {
    const request = ++searchRequest;
    submittedQuery = movieQuery.trim();
    movieResults = []; showResults = []; matchedSports = []; searchVideos = []; searchStatus = '';
    if (!submittedQuery) { searching = false; return; }
    searching = true;
    const query = submittedQuery;
    const results = await Promise.allSettled([
      searchYouTube(query).then(items => { if (request === searchRequest) searchVideos = items; }),
      searchMovies(query).then(items => { if (request === searchRequest) movieResults = items; }),
      searchShows(query).then(items => { if (request === searchRequest) showResults = items; }),
      loadSportsListing().then(listing => {
        if (request === searchRequest) matchedSports = listing.events.filter(item => matchesSearch(item.title + ' ' + item.league));
      }),
    ]);
    if (request !== searchRequest) return;
    searching = false;
    searchStatus = results.some(result => result.status === 'rejected') ? 'Some sources could not load. Search again to retry.' : movieResults.length + showResults.length + matchedSports.length + searchVideos.length + searchHistory.length ? '' : 'No results found. Try another title, team, or league.';
  }

  const popular = $derived(homeState.popular);
  const featured = $derived(homeState.featured);
  const popularShows = $derived(homeState.popularShows);
  let playerOpen = $state(false);
  let musicHome = $state<MusicHome>();
  let sportsHome = $state<SportsHome>();
  let player = $state<NowPlaying>();
  let launching = $state<AppId | null>(null);
  let stremioPairing = $state(false);
  const stremio = $derived(stremioState.status);
  const steamGames = $derived(steamState.games);
  const steamRecommendations = $derived(steamState.recommendations);
  const steamRecommendationSource = $derived(steamState.recommendationSource);
  const previewApp = $derived(homeState.previewApp);
  const previewLeague = $derived(homeState.previewLeague);
  const leagueGames = $derived(homeState.leagueGames);
  const leagueGamesLoading = $derived(homeState.leagueGamesLoading);
  let sportsReturn: { tab: string; focus: HTMLElement | null; eventId: string } | null = null;
  async function openSportsDetail(event: SportsEvent) {
    sportsReturn = { tab, eventId: event.id, focus: document.activeElement instanceof HTMLElement ? document.activeElement : null };
    navigate('Sports');
    await tick();
    sportsHome?.showEvent(event);
  }
  function returnFromSportsDetail() {
    const origin = sportsReturn;
    sportsReturn = null;
    if (!origin) return;
    navigate(origin.tab);
    void tick().then(() => {
      const target = origin.focus?.isConnected ? origin.focus : origin.tab === 'Search'
        ? [...document.querySelectorAll<HTMLElement>('[data-sports-event]')].find(element => element.dataset.sportsEvent === origin.eventId)
        : document.querySelector<HTMLElement>('#home-button');
      target?.focus({ preventScroll: true });
    });
  }
  async function watchLeagueGame(game: GameScore) {
    await openSportsDetail(sports.eventFor(game));
  }

  const youTubeFeed = $derived(youtube.feed);
  let searchVideos = $state.raw<import('./lib/types').YouTubeFeedItem[]>([]);

  let previous: HTMLElement | null = null;
  let playerPrevious: HTMLElement | null = null;
  const navigation = createNavigation(back, home, () => settings.homeShortcut, playSound);
  const showHome = $derived(tab === 'Home');
  const heroSlides = $derived(homeState.heroSlides);
  const heroSource = $derived(homeState.heroSource);
  const catalogVisible = $derived(homeState.catalogVisible);
  const stremioRecent = $derived(homeState.stremioRecent);
  const previewActivity = $derived(homeState.previewActivity);
  const previewMusic = $derived(homeState.previewMusic);
  function previewActivityItem(item: ActivityItem | null) { homeState.previewActivityItem(item); }
  function previewMovie(item: MediaItem | null, eyebrow?: string) { homeState.previewMovie(item, eyebrow); }
  function preview(id: AppId) { homeState.preview(id); }

  function horizontalWheel(node: HTMLElement) {
    function wheel(event: WheelEvent) {
      const track = event.target instanceof Element ? event.target.closest<HTMLElement>('.app-track, .card-track, .activity-track, .video-track, .music-card-track, .sports-grid') : null;
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

  let activePreviewId: string | null | undefined;
  $effect(() => {
    const candidate = preferences.preview;
    const id = candidate?.id ?? null;
    if (id === activePreviewId) return;
    activePreviewId = id;
    if (!candidate) return;
    panel = false;
    tab = 'Home';
    details = null;
    playerOpen = false;
    homeState.clearPreview();
  });
  $effect(() => {
    if (native) invoke('update_app_state', { view: panel ? 'Settings' : details ? 'Details' : tab, fullscreen: settings.fullscreen, sound: settings.interfaceSounds }).catch(error => message = String(error));
  });
  $effect(() => {
    if (preferences.notice) { message = preferences.notice; preferences.notice = ''; }
  });
  $effect(() => { if (homeState.error) message = homeState.error; });
  $effect(() => { if (stremioState.error) message = stremioState.error; });
  $effect(() => {
    if (activity.error) message = activity.error;
  });
  onMount(() => {
    setSoundSettings(settings.interfaceSounds, settings.soundLevel);
    warmSound();
    void preferences.start();
    void homeState.start();
    const unlock = () => unlockSound();
    window.addEventListener('pointerdown', unlock, { once: true, capture: true });
    window.addEventListener('keydown', unlock, { once: true, capture: true });
    void initializeUpdates();
    let disposed = false;
    const cleanup: (() => void)[] = [];
    function track(stop: () => void) { if (disposed) stop(); else cleanup.push(stop); }
    async function start() {
      if (!native) { message = 'Browser preview. Native app launching, fullscreen, web remote, and Spotify controls require the desktop app.'; navigation.focus(document.querySelector<HTMLElement>('#home-button') ?? undefined); return; }
      try {
        const stopMusic = await onOpenMusic(() => { if (!disposed) void openMusic(); });
        if (disposed) { stopMusic(); return; }
        track(stopMusic);
        track(await listen<boolean>('window-visibility', event => setAppVisible(event.payload)));
        track(await listen<string>('open-sports', async event => {
          if (disposed) return;
          navigate('Sports');
          await tick();
          await sportsHome?.showLeague(event.payload);
        }));
        track(await listen<string>('tv-setup-error', event => message = event.payload));
        const stopNavigation = await onNavigation(navigation.dispatch);
        if (disposed) { stopNavigation(); return; }
        track(stopNavigation);
        track(await listen('remote-volume', () => { if (!disposed) void playSound('volume'); }));
        track(await listen('exit-requested', () => {
          if (disposed) return;
          previous = document.activeElement as HTMLElement;
          exitRequested = true;
        }));
        const [stopActivity, stopSports, stopStremio, stopYouTube] = await Promise.all([
          activity.start(),
          sports.start(),
          stremioState.start(),
          youtube.start(),
          invoke<Record<ExecutableAppId, string>>('app_paths').then(paths => { if (!disposed) settings.paths = paths; }).catch(error => { if (!disposed) message = String(error); }),
        ]);
        track(stopActivity);
        track(stopSports);
        track(stopStremio);
        track(stopYouTube);
        if (disposed) return;
        if (stremioState.error) message = stremioState.error;
        await getCurrentWindow().setFullscreen(settings.fullscreen);
        track(await getCurrentWindow().onResized(async () => { const value = await getCurrentWindow().isFullscreen(); if (value !== settings.fullscreen) settings.fullscreen = value; }));
        if (!disposed) void hydrate();
      } catch (error) { message = String(error); }
      finally { if (disposed) cleanup.splice(0).forEach(stop => stop()); }
    }
    async function hydrate() {
      try {
        await spotify.initialize();
        track(() => spotify.dispose());
        if (spotify.connection.installed && await getCurrentWindow().isVisible()) void spotify.start();
        const tvWarning = await invoke<string | null>('tv_setup_warning');
        if (tvWarning && !disposed) message = tvWarning;
        await detect(false);
        void steamState.refresh();
      } catch (error) { if (!disposed) message = String(error); }
    }
    async function reveal() {
      try { await start(); }
      finally {
        await tick();
        document.getElementById('startup')?.remove();
        document.getElementById('app')?.removeAttribute('inert');
        if (!disposed) {
          void getCurrentWindow().isVisible().then(visible => {
            if (!visible || disposed) return;
            if (native) void invoke('play_startup_sound', { preset: settings.interfaceSounds, level: settings.soundLevel }).catch(() => playStartupSound());
            else void playStartupSound();
          }).catch(() => {});
          navigation.focus(document.querySelector<HTMLElement>('#home-button') ?? undefined);
          if (native) await invoke('launcher_ready');
        }
      }
    }
    void reveal();
    return () => {
      disposed = true;
      preferences.dispose();
      cleanup.splice(0).forEach(stop => stop());
      window.removeEventListener('pointerdown', unlock, true);
      window.removeEventListener('keydown', unlock, true);
    };
  });
  function rememberFocus() { previous = document.activeElement as HTMLElement; }
  function back() {
    if (layoutPreview && !exitRequested && !remotePairing && !stremioPairing && !details && !playerOpen) { void cancelReview(); return; }
    if (exitRequested) { void playSound('back'); cancelExit(); return; }
    if (remotePairing) { void playSound('back'); closeRemotePairing(); return; }
    if (stremioPairing) { void playSound('back'); closeStremioPairing(); return; }
    if (panel) { void playSound('back'); panel = false; void refreshStremio(); }
    else if (details) { void playSound('back'); details = null; }
    else if (playerOpen) { void playSound('back'); if (!player?.closeOverlay()) closePlayer(); return; }
    else if (tab === 'Sports' && sportsHome?.closeDetail()) { void playSound('back'); return; }
    else if (tab === 'Music' && musicHome?.closeDetail()) { void playSound('back'); return; }
    else { void playSound('back'); home(); return; }
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
  async function confirmQuit() { await playShutdownSound(); void quitLuma().catch(() => {}); }
  function mountExitDialog(dialog: HTMLDialogElement) {
    dialog.showModal();
    requestAnimationFrame(() => dialog.querySelector<HTMLButtonElement>('button')?.focus());
  }
  function leaveApp() { if (native) void closeLaunchedApp().catch(() => {}); }
  function home() { leaveApp(); remotePairing = false; homeState.clearPreview(); playerOpen = false; panel = false; details = null; tab = 'Home'; requestAnimationFrame(() => { window.scrollTo(0, 0); navigation.focus(document.querySelector<HTMLElement>('#home-button') ?? undefined); }); }
  function openPlayer() {
    playerPrevious = document.activeElement as HTMLElement;
    playerOpen = true;
    requestAnimationFrame(() => navigation.focus(document.querySelector<HTMLElement>('.now-playing-close') ?? undefined));
  }
  function closePlayer() {
    playerOpen = false;
    requestAnimationFrame(() => navigation.focus(playerPrevious ?? undefined));
  }
  function pointerSound(event: MouseEvent) {
    if (event.detail === 0 || soundPlayedRecently()) return;
    if (!(event.target instanceof Element) || event.target.closest('input, select, textarea, label, option')) return;
    const target = event.target.closest('button, a[href], summary, [role="button"]');
    if (target && !(target instanceof HTMLButtonElement && target.disabled)) void playSound('press');
  }
  function changeSound(event: Event) {
    if (event.target instanceof HTMLInputElement && (event.target.type === 'checkbox' || event.target.type === 'radio')) void playSound('toggle');
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
  const tabOrder = ['Home', 'Search', 'Sports', 'Library', 'Music', 'Apps'];
  let previousTab = 0;
  function animateView(view: string) {
    const index = Math.max(0, tabOrder.indexOf(view));
    const direction = index < previousTab ? -1 : 1;
    previousTab = index;
    return (element: HTMLElement) => {
      if (settings.reducedMotion || window.matchMedia('(prefers-reduced-motion: reduce)').matches) return;
      const animation = element.animate(
        [{ opacity: .35, transform: `translate3d(${direction * 28}px, 6px, 0)` }, { opacity: 1, transform: 'none' }],
        { duration: 220, easing: 'cubic-bezier(.22,.8,.24,1)' },
      );
      return () => animation.cancel();
    };
  }
  function navigate(value: string) { if (layoutPreview) { void cancelReview(false).then(cancelled => { if (cancelled) navigate(value); }); return; } if (value === 'Music') { void openMusic(); return; } if (value !== tab) void playSound('page'); if (value === 'Home') leaveApp(); homeState.clearPreview(); tab = value; requestAnimationFrame(() => window.scrollTo(0, 0)); }
  async function openMusic() {
    if (layoutPreview && !await cancelReview(false)) return;
    if (tab !== 'Music') void playSound('page');
    while (musicHome?.closeDetail()) {}
    homeState.clearPreview();
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
  function showSettings(category: string) { if (layoutPreview) { void cancelReview(false).then(cancelled => { if (cancelled) showSettings(category); }); return; } homeState.clearPreview(); rememberFocus(); message = ''; settingsCategory = category; panel = true; }
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
      void playSound('open');
    } catch (error) { message = String(error); void playSound('error'); }
    finally { launching = null; }
  }
  async function openItem(item: ActivityItem) {
    if (!native) { message = 'Opening activity is available in the Luma desktop app.'; return; }
    try { message = (await activity.open(item.id, settings.fullscreen)).message; void playSound('open'); }
    catch (error) { message = String(error); void playSound('error'); }
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
  async function refreshStremio(sync = false) {
    await stremioState.refresh(sync);
    if (stremioState.error) message = stremioState.error;
  }
  async function syncStremio() {
    try {
      await stremioState.sync();
      message = 'Your Stremio library is up to date.';
    } catch (error) { message = String(error); }
  }
  async function openYouTubeItem(id: string) {
    if (!native) { message = 'Opening YouTube is available in the desktop app.'; return; }
    try { await youtube.openVideo(id, settings.fullscreen); message = 'Opening in YouTube. Press Ctrl+W there to return.'; void playSound('open'); }
    catch (error) { message = String(error); void playSound('error'); }
  }
  async function openStremio(id: string) {
    if (!native) { message = 'Opening Stremio is available in the desktop app.'; return; }
    try { await stremioState.open(id); message = 'Opening in Stremio inside Luma.'; void playSound('open'); }
    catch (error) { message = String(error); void playSound('error'); }
  }
  async function openStremioDetail(kind: string, id: string) {
    if (!native) { message = 'Opening Stremio is available in the desktop app.'; return; }
    try { await stremioState.openDetail(kind, id); message = 'Opening in Stremio inside Luma.'; void playSound('open'); }
    catch (error) { message = String(error); void playSound('error'); }
  }
  async function openSteamGame(appid: number) {
    if (!native) { message = 'Opening Steam games is available in the desktop app.'; return; }
    try { await steamState.openGame(appid); message = 'Opening in Steam Big Picture. Press Home to return to Luma.'; void playSound('open'); }
    catch (error) { message = String(error); void playSound('error'); }
  }
  async function previewLayout(draft: HomeDraft) {
    await preferences.previewLayout(draft);
  }
  async function keepLayout() {
    const pluginPreview = layoutPreview?.plugin;
    if (!await preferences.keepLayout()) return;
    homeDraft = null;
    if (pluginPreview) {
      message = pluginPreview.action === 'settings'
        ? `${pluginPreview.name} settings saved.`
        : pluginPreview.action === 'reattach'
          ? `${pluginPreview.name} controls restored.`
          : `${pluginPreview.name} enabled.`;
      settingsCategory = 'Plugins';
      panel = true;
    } else {
      message = 'Layout saved.';
      requestAnimationFrame(() => navigation.focus(document.querySelector<HTMLElement>('#home-button') ?? undefined));
    }
  }
  async function cancelReview(editing = preferences.previewFromEditor) {
    const pluginPreview = layoutPreview?.plugin;
    if (!await preferences.cancelReview()) return false;
    if (pluginPreview) {
      settingsCategory = 'Plugins';
      panel = true;
      message = 'Plugin change cancelled.';
    } else if (editing) {
      settingsCategory = 'Home layout';
      panel = true;
    } else {
      requestAnimationFrame(() => navigation.focus(document.querySelector<HTMLElement>('#home-button') ?? undefined));
    }
    return true;
  }
  async function openSteamStore(appid: number) {
    if (!native) { message = 'Opening the Steam store is available in the desktop app.'; return; }
    try { await steamState.openStore(appid); message = 'Opening the Steam store in Big Picture. Press Home to return to Luma.'; void playSound('open'); }
    catch (error) { message = String(error); void playSound('error'); }
  }
</script>

<svelte:window onkeydown={keydown} onclick={pointerSound} onchange={changeSound} />
<main class="app-shell" class:is-home={showHome} {@attach horizontalWheel}>
  {#if panel}<SettingsPanel {message} bind:settings={preferences.value} bind:homeDraft revision={preferences.revision} onlayoutpreview={previewLayout} onlayoutrestore={() => preferences.restoreLayout()} {native} {activity} startCategory={settingsCategory} close={back} {fullscreen} {savePaths} {detect} popular={popular} featured={featured} plugins={preferences.plugins} pluginsLoading={preferences.pluginsLoading} pluginsError={preferences.pluginsError} onpluginsreload={() => preferences.refreshPlugins()} />
  {:else}
  <TopNav active={tab} {navigate} settings={openSettings} connect={openRemotePairing} fullscreen={settings.fullscreen} toggle={fullscreen} />
  <div class="topbar-offset" aria-hidden="true"></div>
  <div class="view-content" {@attach animateView(tab)}>
  {#if showHome}
    <HomeRenderer rows={layoutConfig.home} columns={layoutConfig.homeColumns}
      data={{ heroSlides, heroSource, previewApp, stremioRecent, stremio, steamGames, activityItems, trackingEnabled: activity.trackingEnabled, order: layoutConfig.order, launching, catalogVisible, youTubeFeed, previewMusic, steamRecommendations, steamRecommendationSource, previewActivity, musicLoading: spotify.homeLoading, popular, featured, previewLeague, leagueGames, leagueGamesLoading }}
      actions={{ openStremio, openStremioPairing, previewStremio: item => homeState.previewStremio(item), openSteamGame, previewSteamGame: item => homeState.previewSteamGame(item), openItem, removeItem, previewActivityItem, launch, preview, watchLeagueGame, openYouTubeItem, previewVideo: item => homeState.previewVideo(item), playMusic: async item => { if (await spotify.playUri(item.uri)) openPlayer(); }, previewMusicItem: item => homeState.previewMusicItem(item), openSteamStore, previewSteamRecommendation: item => homeState.previewSteamRecommendation(item), select, previewMovie }} />
  {/if}
  {#if tab === 'Library'}<StremioLibrary status={stremio} {native} open={openStremio} connect={openStremioPairing} sync={syncStremio} />{/if}
  {#if tab === 'Sports'}<SportsHome bind:this={sportsHome} {native} {sports} ondetailclose={returnFromSportsDetail} />{/if}
  {#if tab === 'Apps'}
    <header class="page-head">
      <div><span class="eyebrow">ON THIS PC</span><h1>Apps</h1><p>Open your streaming apps and keep the ones you use most in reach.</p></div>
      <button class="text-link" onclick={openApps}>Edit apps <span>→</span></button>
    </header>
    <AppDock order={settings.order} {launch} {launching} {preview} active={previewApp} />
  {/if}
  {#if tab === 'Search'}
    <header class="page-head">
      <div><span class="eyebrow">FIND YOUR NEXT WATCH</span><h1>{tab}</h1><p>Find movies, shows, videos, and sports. Open a result to watch.</p></div>
      <form onsubmit={searchWatch} class="movie-search-field" role="search">
        <svg class="movie-search-icon" viewBox="0 0 24 24" aria-hidden="true"><circle cx="11" cy="11" r="7" /><path d="m20 20-3.5-3.5" /></svg>
        <input id="movie-search" type="search" bind:value={movieQuery} placeholder="Search titles, teams, or leagues" aria-label="Search all sources" autocomplete="off" spellcheck="false" />
        <button type="submit" class="movie-search-submit">Search</button>
      </form>
    </header>
    <section class="content-row" aria-label="Search status">
      <p class="movie-search-status" role="status">{searching ? 'Searching sources…' : searchStatus}</p>
    </section>
    {#if movieResults.length}
      <MediaRow id="search-results" title="Movies - Stremio" items={movieResults} onselect={item => openStremioDetail('movie', item.id)} />
    {/if}
    {#if showResults.length}<MediaRow id="show-results" title="Shows - Stremio" items={showResults} onselect={item => openStremioDetail('series', item.id)} />{/if}
    {#if submittedQuery}
      <div id="search-extra">
        {#if searchVideos.length}
          <section class="content-row" aria-label="YouTube search results"><div class="row-heading"><div><h2>YouTube</h2><p>YouTube search results</p></div></div>
            <div class="video-track">
              {#each searchVideos as item (item.id)}
                <button class="video-card" onclick={() => openYouTubeItem(item.id)}>
                  <span class="video-art"><img src={`https://i.ytimg.com/vi/${item.id}/hqdefault.jpg`} alt="" loading="lazy" decoding="async" /></span>
                  <span class="video-name">{item.title}</span>
                </button>
              {/each}
            </div>
          </section>
        {/if}
        {#if matchedSports.length}<SearchSportsRow items={matchedSports} onselect={item => void openSportsDetail(item)} />{/if}
        {#if searchHistory.length}
          <section class="content-row activity-row" aria-label="Results from your sources">
            <div class="row-heading"><div><h2>From your sources</h2><p>Titles from your connected history</p></div></div>
            <div class="activity-track is-cinematic">{#each searchHistory as item (item.id)}<ActivityCard {item} cinematic onopen={openItem} onpreview={previewActivityItem} />{/each}</div>
          </section>
        {/if}
      </div>
    {/if}
    {#if !submittedQuery}
    <div {@attach smoothHeight}><div class="resizing-content">
      <MediaRow id="popular" title="Popular movies" items={popular} onselect={select} />
      <MediaRow id="featured" title="Top-rated movies" items={featured} onselect={select} />
      <MediaRow id="popular-shows" title="Popular shows" items={popularShows} onselect={item => openStremioDetail('series', item.id)} />
    </div></div>
    <ContinueWatching items={activityItems} trackingEnabled={activity.trackingEnabled} onopen={openItem} />
    <RecentlyPlayed items={activityItems} trackingEnabled={activity.trackingEnabled} onopen={openItem} onremove={removeItem} onpreview={previewActivityItem} />
  {/if}
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
  {#if !playerOpen}<WidgetHost widgets={layoutConfig.widgets} onopen={openPlayer} />{/if}
  {#if playerOpen}<NowPlaying bind:this={player} onclose={closePlayer} />{/if}
  {/if}
</main>
{#if layoutPreview}<LayoutReview busy={preferences.reviewBusy} error={preferences.reviewError} editing={preferences.previewFromEditor} plugin={layoutPreview.plugin ?? null} detached={layoutPreview.detached ?? []} keep={() => void keepLayout()} cancel={() => void cancelReview()} />{/if}
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
  .remote-dialog { width: min(820px, calc(100vw - 48px)); max-height: calc(100dvh - 48px); margin: auto; padding: clamp(24px, 3vw, 40px); overflow-y: auto; border: 1px solid var(--line-strong); border-radius: 24px; background: var(--surface-2); color: var(--text); box-shadow: 0 24px 90px #0008; }
  .remote-dialog::backdrop { background: #05060aaa; backdrop-filter: blur(6px); }
  .remote-dialog-heading { display: flex; align-items: center; justify-content: space-between; gap: 24px; }
  .remote-dialog-heading h2 { margin: 8px 0 0; font-size: clamp(26px, 2.4vw, 36px); }
  .exit-dialog { width: min(560px, calc(100vw - 48px)); margin: auto; padding: clamp(26px, 3vw, 38px); border: 1px solid var(--line-strong); border-radius: 22px; background: var(--surface-2); color: var(--text); box-shadow: 0 24px 90px #0008; }
  .exit-dialog::backdrop { background: #05060aaa; backdrop-filter: blur(6px); }
  .exit-dialog h2 { margin: 0 0 12px; font-size: clamp(24px, 2.2vw, 32px); letter-spacing: -.03em; }
  .exit-dialog p { margin: 0 0 24px; color: var(--muted); font-size: clamp(13px, .95vw, 16px); line-height: 1.6; }
  .exit-actions { display: flex; gap: 12px; justify-content: flex-end; flex-wrap: wrap; }
  .exit-actions button { width: auto; min-height: 48px; margin: 0; padding: 0 22px; }
  .exit-actions .exit-keep { border-color: transparent; background: var(--accent); color: var(--accent-ink); }
</style>
