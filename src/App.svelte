<script lang="ts">
  import { listen } from '@tauri-apps/api/event';
  import { onMount, tick, untrack } from 'svelte';
  import { dismissUpdate, initializeUpdates, updates } from './lib/state/updates.svelte';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { invoke } from '@tauri-apps/api/core';
  import Setup from './lib/components/Setup.svelte';
  import TopNav from './lib/components/TopNav.svelte';
  import HomeRenderer from './lib/components/HomeRenderer.svelte';
  import LayoutReview from './lib/components/LayoutReview.svelte';
  import UpdateDialog from './lib/components/UpdateDialog.svelte';
  import FramePairing from './lib/components/FramePairing.svelte';
  import SearchSportsRow from './lib/components/SearchSportsRow.svelte';
  import ActivityCard from './lib/components/activity/ActivityCard.svelte';
  import MediaRow from './lib/components/MediaRow.svelte';
  import AppDock from './lib/components/AppDock.svelte';
  import PageHeader from './lib/components/PageHeader.svelte';
  import EmptyState from './lib/components/EmptyState.svelte';
  import Screensaver from './lib/components/Screensaver.svelte';
  import { loadSaver } from './lib/services/screensaver';
  import SettingsPanel from './lib/components/SettingsPanel.svelte';
  import WidgetHost from './lib/components/WidgetHost.svelte';
  import WidgetHud from './lib/components/WidgetHud.svelte';
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
  import { createController } from './lib/services/controller';
  import { smoothHeight } from './lib/services/motion';
  import { playShutdownSound, playSound, playStartupSound, setSoundSettings, soundPlayedRecently, unlockSound, warmSound } from './lib/services/sound';
  import { onAppVisible, setAppVisible } from './lib/services/visibility';
  import { providerName } from './lib/services/activity';
  import { native, searchYouTube, launchApp, closeLaunchedApp, quitLuma, hideToTray, saveAppPaths, onNavigation, onOpenMusic, openYouTube } from './lib/services/tauri';
  import { searchMovies, searchShows } from './lib/services/catalog';
  import type { PluginHomeCard, PluginHomeCardFailure } from './lib/services/plugins';
  import type { ActivityItem, AppId, ControllerAction, ExecutableAppId, MediaItem } from './lib/types';
  import { loadSportsListing, type SportsEvent } from './lib/services/sports';
  import type { GameScore } from './lib/services/sportsScores';

  const preferences = createSettings();
  const settings = preferences.value;
  function shouldShowSetup() {
    try {
      const status = localStorage.getItem('luma-setup');
      if (status) return status === 'pending';
      if (localStorage.getItem('luma-v1')) return false;
      localStorage.setItem('luma-setup', 'pending');
    } catch {}
    return true;
  }
  let setupOpen = $state(shouldShowSetup());
  let setup: Setup | undefined = $state();
  function finishSetup() {
    try { localStorage.setItem('luma-setup', 'complete'); } catch {}
    setupOpen = false;
    void refreshStremio();
    void tick().then(() => navigation.focus(document.querySelector<HTMLElement>('#home-button') ?? undefined));
  }
  const activity = createActivity();
  const activityItems = $derived(activity.items);
  const sports = createSports();
  const stremioState = createStremio();
  const steamState = createSteam();
  const youtube = createYouTube();
  const layoutConfig = $derived(preferences.layout);
  const hasPlayerWidgets = $derived(Object.values(layoutConfig.widgets).some(widget => widget.enabled && widget.presence.includes('music')));
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
  let searchReturn = $state('Home');
  let panel = $state(false);
  let updateView = $state<'installed' | 'update' | null>(null);
  let updateOrigin: HTMLElement | undefined;
  let launcherReady = $state(false);
  let launcherVisible = $state(true);
  let settingsPanel = $state<{ consumeBack: () => boolean; cycleCategory: (direction: -1 | 1) => void }>();
  let remotePairing = $state(false);
  let exitRequested = $state(false);
  let settingsCategory = $state('Appearance');
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
  async function submitSearch() {
    if (layoutPreview) return;
    if (tab !== 'Search') searchReturn = tab;
    navigate('Search');
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
  let saver = $state(loadSaver());
  let saverOpen = $state(false);
  let topNav = $state<ReturnType<typeof TopNav>>();
  $effect(() => { localStorage.setItem('luma.screensaver', JSON.stringify(saver)); });
  let idle = $state(false);
  let lastInput = Date.now();
  function wake() {
    lastInput = Date.now();
    if (idle) idle = false;
    const dismissed = saverOpen;
    saverOpen = false;
    return dismissed;
  }
  let widgetHost = $state<WidgetHost>();
  let musicHome = $state<MusicHome>();
  let sportsHome = $state<SportsHome>();
  let player = $state<NowPlaying>();
  let launching = $state<AppId | null>(null);
  let stremioPairing = $state(false);
  const pluginHomeCards = $derived(preferences.plugins?.homeCards ?? []);
  let pluginCardPending = $state<string | null>(null);
  let pluginCardFailure = $state<PluginHomeCardFailure | null>(null);
  let pluginFocusReturn: string | null = null;
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
  let controllerCapture: ((buttonIndex: number) => boolean) | null = null;
  const showHome = $derived(tab === 'Home');
  const heroSlides = $derived(homeState.heroSlides);
  const heroSource = $derived(homeState.heroSource);
  const catalogVisible = $derived(homeState.catalogVisible);
  const stremioRecent = $derived(homeState.stremioRecent);
  const previewActivity = $derived(homeState.previewActivity);
  const previewMusic = $derived(homeState.previewMusic);
  async function retryCatalog(event: MouseEvent) {
    const control = event.currentTarget as HTMLElement;
    const focused = document.activeElement === control;
    await Promise.all([homeState.start(), ...(tab === 'Search' ? [homeState.loadShows(true)] : [])]);
    await tick();
    if (focused && (document.activeElement === control || document.activeElement === document.body)) {
      navigation.focus(control.isConnected ? control : document.querySelector<HTMLElement>(tab === 'Search' ? '#movie-search' : '#home-button') ?? undefined);
    }
  }
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
    if (tab === 'Search' && !submittedQuery) untrack(() => { void homeState.loadShows(); });
  });
  $effect(() => {
    if (native) invoke('update_app_state', { view: panel ? 'Settings' : details ? 'Details' : tab, fullscreen: settings.fullscreen, sound: settings.interfaceSounds }).catch(error => message = String(error));
  });
  $effect(() => {
    if (preferences.notice) { message = preferences.notice; preferences.notice = ''; }
  });
  $effect(() => {
    const cards = preferences.plugins?.homeCards;
    const failure = pluginCardFailure;
    if (cards && failure && !cards.some(card => card.id === failure.card.id)) pluginCardFailure = null;
  });
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
    const restoreFocus = () => { if (!disposed) restorePluginFocus(); };
    window.addEventListener('focus', restoreFocus);
    const cleanup: (() => void)[] = [];
    function track(stop: () => void) { if (disposed) stop(); else cleanup.push(stop); }
    track(onAppVisible(visible => { launcherVisible = visible; }));
    const idleCheck = setInterval(() => {
      const elapsed = Date.now() - lastInput;
      if (!playerOpen && elapsed > 120_000) idle = true;
      if (!saver.enabled || saverOpen) return;
      if (!document.hasFocus() || document.hidden || panel || details || remotePairing || stremioPairing || exitRequested || layoutPreview || document.querySelector('dialog[open]') || ((playerOpen || tab === 'Music') ? !saver.player : tab !== 'Home')) {
        lastInput = Date.now();
        return;
      }
      if (elapsed >= saver.minutes * 60_000) saverOpen = true;
    }, 1000);
    function wakeFromInput(event: Event) {
      if (event instanceof KeyboardEvent && (event.key.startsWith('Media') || event.key.startsWith('AudioVolume'))) return;
      if (!wake()) return;
      event.preventDefault();
      event.stopImmediatePropagation();
    }
    const inputOptions = { passive: true } as AddEventListenerOptions;
    window.addEventListener('pointermove', wake, inputOptions);
    window.addEventListener('blur', wake);
    for (const event of ['click', 'wheel', 'keydown']) window.addEventListener(event, wakeFromInput, { capture: true, passive: false });
    track(() => {
      clearInterval(idleCheck);
      window.removeEventListener('pointermove', wake);
      window.removeEventListener('blur', wake);
      for (const event of ['click', 'wheel', 'keydown']) window.removeEventListener(event, wakeFromInput, true);
    });
    const controller = createController({
      enabled: () => settings.controllerEnabled,
      binds: () => settings.controllerBinds,
      oncommand: action => runControllerAction(action),
      oncapture: buttonIndex => controllerCapture?.(buttonIndex) ?? false,
    });
    track(() => controller.destroy());
    async function start() {
      if (!native) { message = 'Browser preview. Native app launching, fullscreen, web remote, and Spotify controls require the desktop app.'; navigation.focus(document.querySelector<HTMLElement>('#home-button') ?? undefined); return; }
      try {
        const stopMusic = await onOpenMusic(() => { if (!disposed) void openMusic(); });
        if (disposed) { stopMusic(); return; }
        track(stopMusic);
        track(await listen<boolean>('window-visibility', event => {
          setAppVisible(event.payload);
          if (event.payload && !disposed) void steamState.refresh(true);
        }));
        track(await listen<string>('open-sports', async event => {
          if (disposed) return;
          navigate('Sports');
          await tick();
          await sportsHome?.showLeague(event.payload);
        }));
        track(await listen<string>('tv-setup-error', event => message = event.payload));
        track(await listen<string>('viewing-error', event => message = event.payload));
        const stopNavigation = await onNavigation(command => { if (!wake()) navigation.dispatch(command); });
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
        if (!disposed) launcherReady = true;
        if (!disposed) {
          if (native) {
            void getCurrentWindow().isVisible().then(visible => {
              if (visible && !disposed) void invoke('play_startup_sound', { preset: settings.interfaceSounds, level: settings.soundLevel }).catch(() => playStartupSound());
            }).catch(() => {});
          } else void playStartupSound();
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
      window.removeEventListener('focus', restoreFocus);
    };
  });
  $effect(() => {
    if (!native || !launcherReady || !launcherVisible || updateView || panel || !showHome
      || details || playerOpen || remotePairing || stremioPairing || exitRequested || layoutPreview || saverOpen || launching) return;
    if (updates.phase !== 'available' && updates.phase !== 'ready') return;
    if (!updates.nextVersion || updates.dismissedVersion === updates.nextVersion || document.querySelector('dialog[open]')) return;
    untrack(() => showUpdate('update'));
  });

  function showUpdate(kind: 'installed' | 'update') {
    updateOrigin = document.activeElement instanceof HTMLElement ? document.activeElement : undefined;
    updateView = kind;
  }

  async function closeUpdate() {
    if (updateView === 'update') dismissUpdate();
    const origin = updateOrigin;
    updateView = null;
    updateOrigin = undefined;
    await tick();
    navigation.focus(origin);
  }

  function rememberFocus() { previous = document.activeElement as HTMLElement; }
  function back() {
    if (setupOpen && setup?.back()) return;
    if (updateView) { void playSound('back'); void closeUpdate(); return; }
    if (topNav?.backMenu()) { void playSound('back'); return; }
    if (widgetHost?.cancelMove()) return;
    if (layoutPreview && !exitRequested && !remotePairing && !stremioPairing && !details && !playerOpen) { void cancelReview(); return; }
    if (exitRequested) { void playSound('back'); cancelExit(); return; }
    if (remotePairing) { void playSound('back'); closeRemotePairing(); return; }
    if (stremioPairing) { void playSound('back'); closeStremioPairing(); return; }
    if (panel && settingsPanel?.consumeBack()) { void playSound('back'); return; }
    if (panel) { void playSound('back'); panel = false; void refreshStremio(); }
    else if (details) { void playSound('back'); details = null; }
    else if (playerOpen) { void playSound('back'); if (!player?.closeOverlay()) closePlayer(); return; }
    else if (tab === 'Sports' && sportsHome?.closeDetail()) { void playSound('back'); return; }
    else if (tab === 'Music' && musicHome?.closeDetail()) { void playSound('back'); return; }
    else if (tab === 'Search' && searchReturn !== 'Search') { void playSound('back'); navigate(searchReturn); requestAnimationFrame(() => document.querySelector<HTMLElement>('#movie-search')?.focus()); return; }
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
  function home() { if (updateView === 'update') dismissUpdate(); updateView = null; updateOrigin = undefined; topNav?.closeMenu(); leaveApp(); remotePairing = false; homeState.clearPreview(); playerOpen = false; wake(); panel = false; details = null; tab = 'Home'; requestAnimationFrame(() => { window.scrollTo(0, 0); navigation.focus(document.querySelector<HTMLElement>('#home-button') ?? undefined); }); }
  function openPlayer() {
    playerPrevious = document.activeElement as HTMLElement;
    playerOpen = true;
    wake();
    requestAnimationFrame(() => navigation.focus(document.querySelector<HTMLElement>('.now-playing-close') ?? undefined));
  }
  function closePlayer() {
    playerOpen = false;
    wake();
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
    if (event.target instanceof HTMLInputElement && event.target.id === 'movie-search' && (event.key === 'ArrowUp' || event.key === 'ArrowDown')) {
      event.preventDefault();
      const up = event.key === 'ArrowUp';
      navigation.focus((up
        ? document.querySelector<HTMLElement>('.nav-pill [aria-current="page"]') ?? document.querySelector<HTMLElement>('#home-button')
        : document.querySelector<HTMLElement>('.view-content button, .view-content input, .view-content select, .view-content a[href]')
      ) ?? undefined);
      return;
    }
    if (event.key === 'F11' && !event.repeat) { event.preventDefault(); void fullscreen(); return; }
    if (event.key === ' ' && event.target instanceof HTMLElement && event.target.matches('[data-release-notes-scroll]')) return;
    if (event.key === ' ' && !event.repeat && !event.altKey && !event.ctrlKey && !event.metaKey && !(event.target instanceof HTMLInputElement) && !(event.target instanceof HTMLTextAreaElement) && !(event.target instanceof HTMLSelectElement) && !(event.target instanceof HTMLButtonElement) && !(event.target instanceof HTMLAnchorElement)) {
      event.preventDefault();
      if (spotify.playback.canToggle) void spotify.action('toggle');
      return;
    }
    navigation.keydown(event);
  }
  const tabOrder = ['Home', 'Sports', 'Music', 'Library', 'Apps'];
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
  function editingText() {
    const element = document.activeElement;
    return element instanceof HTMLInputElement || element instanceof HTMLTextAreaElement || element instanceof HTMLSelectElement
      || (element instanceof HTMLElement && element.isContentEditable);
  }
  function switchTab(direction: -1 | 1) {
    if (editingText()) return;
    if (panel) {
      if (!document.querySelector('dialog[open]')) settingsPanel?.cycleCategory(direction);
      return;
    }
    if (document.querySelector('dialog[open]') || layoutPreview || remotePairing || stremioPairing || exitRequested || details || playerOpen) return;
    if (tab === 'Search') {
      navigate(searchReturn === 'Search' ? 'Home' : searchReturn);
      requestAnimationFrame(() => document.querySelector<HTMLElement>('.nav-pill [aria-current="page"]')?.focus({ preventScroll: true }));
      return;
    }
    const index = tabOrder.indexOf(tab);
    if (index < 0) return;
    navigate(tabOrder[(index + direction + tabOrder.length) % tabOrder.length]);
    requestAnimationFrame(() => document.querySelector<HTMLElement>('.nav-pill [aria-current="page"]')?.focus({ preventScroll: true }));
  }
  function focusSearch() {
    if (editingText() || document.querySelector('dialog[open]')) return;
    const search = document.querySelector<HTMLInputElement>('#movie-search');
    if (!search) return;
    void playSound('press');
    search.focus({ preventScroll: true });
    search.select();
  }
  function toggleSettings() {
    if (editingText() || document.querySelector('dialog[open]')) return;
    if (panel) {
      void playSound('back');
      panel = false;
      void refreshStremio();
      requestAnimationFrame(() => navigation.focus(document.querySelector<HTMLElement>('.nav-pill [aria-current="page"]') ?? undefined));
      return;
    }
    void playSound('press');
    openSettings();
  }
  function mediaAction(action: 'toggle' | 'previous' | 'next') {
    if (spotify.busy) return;
    if (action === 'toggle' ? !spotify.playback.canToggle : action === 'previous' ? !spotify.playback.canSkipPrevious : !spotify.playback.canSkipNext) return;
    void playSound('volume');
    void spotify.action(action);
  }
  function runControllerAction(action: ControllerAction) {
    if (!['playpause', 'trackprev', 'tracknext'].includes(action) && wake()) return;
    switch (action) {
      case 'tabprev': switchTab(-1); return;
      case 'tabnext': switchTab(1); return;
      case 'search': focusSearch(); return;
      case 'settings': toggleSettings(); return;
      case 'playpause': mediaAction('toggle'); return;
      case 'trackprev': mediaAction('previous'); return;
      case 'tracknext': mediaAction('next'); return;
      case 'scores': if (tab === 'Sports') { void playSound('press'); sportsHome?.toggleScores(); } return;
      default: navigation.dispatch(action);
    }
  }
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
  function openSettings() { showSettings('Appearance'); }
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
  function errorText(error: unknown) {
    return error && typeof error === 'object' && 'message' in error ? String(error.message) : String(error);
  }
  async function openPluginCard(card: PluginHomeCard) {
    if (!native) {
      pluginCardFailure = { card, message: 'Plugin cards are available in the Luma desktop app.' };
      return;
    }
    if (pluginCardPending) return;
    pluginCardPending = card.id;
    pluginCardFailure = null;
    pluginFocusReturn = card.id;
    try {
      await invoke('customization_plugin_command', { pluginId: card.pluginId, commandId: card.commandId });
      void playSound('open');
    } catch (error) {
      pluginCardFailure = { card, message: errorText(error) };
      if (document.hasFocus()) pluginFocusReturn = null;
      void playSound('error');
    } finally {
      pluginCardPending = null;
    }
  }
  function retryPluginCard() {
    const card = pluginCardFailure?.card;
    if (!card || pluginCardPending) return;
    const target = document.querySelector<HTMLElement>(`[data-plugin-card="${CSS.escape(card.id)}"]`);
    if (target) navigation.focus(target);
    void openPluginCard(card);
  }
  function restorePluginFocus() {
    const id = pluginFocusReturn;
    if (!id) return;
    pluginFocusReturn = null;
    if (tab !== 'Home' || panel || details || playerOpen || remotePairing || stremioPairing || exitRequested || layoutPreview) return;
    requestAnimationFrame(() => {
      if (document.querySelector('dialog[open]')) return;
      const target = document.querySelector<HTMLElement>(`[data-plugin-card="${CSS.escape(id)}"]`);
      if (target?.isConnected) {
        navigation.focus(target);
        return;
      }
      if (document.activeElement && document.activeElement !== document.body) return;
      const fallback = document.querySelector<HTMLElement>('[data-home-instance="plugins"] [data-plugin-card]') ?? document.querySelector<HTMLElement>('#home-button');
      if (fallback) navigation.focus(fallback);
    });
  }
  async function launch(id: AppId, retryFeed = false) {
    if (id === 'Spotify') {
      await openMusic();
      return;
    }
    if (launching) return;
    if (!native) { message = 'App launching is available in the Luma desktop app.'; return; }
    launching = id;
    try {
      if (id === 'YouTube') { await openYouTube(settings.fullscreen, retryFeed); message = 'YouTube TV opened in its own window. Use Back to Luma when you are done.'; }
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
          ? `${found} app${found === 1 ? '' : 's'} found. Choose Save app settings to keep them.`
          : 'No apps found. Choose a program under App locations.';
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
    } catch (error) { message = String(error); throw error; }
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

{#if saverOpen}<Screensaver config={saver} playback={spotify.playback} movies={popular} reducedMotion={settings.reducedMotion} />{/if}

<svelte:window onkeydown={keydown} onclick={pointerSound} onchange={changeSound} />
<main class="app-shell" class:is-home={showHome} {@attach horizontalWheel}>
  {#if panel}<SettingsPanel bind:saver onsaverpreview={() => { saverOpen = true; }} bind:this={settingsPanel} {message} bind:settings={preferences.value} bind:homeDraft revision={preferences.revision} onlayoutpreview={previewLayout} onlayoutrestore={() => preferences.restoreLayout()} {native} {activity} steam={steamState} oncontrollercapture={handler => controllerCapture = handler} startCategory={settingsCategory} onupdatenotes={showUpdate} close={back} {fullscreen} {savePaths} {detect} popular={popular} featured={featured} plugins={preferences.plugins} pluginsLoading={preferences.pluginsLoading} pluginsError={preferences.pluginsError} onpluginsreload={() => preferences.refreshPlugins()} />
  {:else}
  <TopNav bind:this={topNav} {native} controllerEnabled={settings.controllerEnabled} binds={settings.controllerBinds}
    sleep={() => invoke('sleep_pc')} exit={() => { rememberFocus(); exitRequested = true; }} startScreensaver={() => { saverOpen = true; }} viewing={() => showSettings('viewing')} switchDisplay={() => { void invoke<string>('switch_display').then(display => message = `Moved to ${display}.`).catch(error => message = String(error)); }} active={tab} {navigate} settings={openSettings} connect={openRemotePairing} fullscreen={settings.fullscreen} toggle={fullscreen} bind:searchQuery={movieQuery} onsearch={submitSearch} />
  <div class="topbar-offset" aria-hidden="true"></div>
  <div class="view-content" {@attach animateView(tab)}>
  {#if (showHome || tab === 'Search') && homeState.error}
    <section class="content-row" aria-label="Catalog status">
      <EmptyState title="Some sources did not load" description={homeState.error} role="status">
        {#snippet actions()}<button class="text-link" disabled={homeState.loading} onclick={retryCatalog}>{homeState.loading ? 'Retrying catalog...' : 'Retry catalog'}</button>{/snippet}
      </EmptyState>
    </section>
  {/if}
  {#if showHome}
    <HomeRenderer rows={layoutConfig.home} columns={layoutConfig.homeColumns}
      data={{ heroSlides, heroSource, previewApp, stremioRecent, stremio, steamGames, activityItems, trackingEnabled: activity.trackingEnabled, order: layoutConfig.order, launching, smoothAppHover: settings.smoothAppHover, catalogVisible, youTubeFeed, youTubeFailed: youtube.failed, stremioError: stremioState.error, stremioLoading: stremioState.loading, previewMusic, steamRecommendations, steamRecommendationSource, previewActivity, musicLoading: spotify.homeLoading, popular, featured, previewLeague, leagueGames, leagueGamesLoading, leagueGamesError: sports.previewError, leagueHero: homeState.leagueHero, sportsBadges: sports.badges, homeCards: pluginHomeCards, pluginCardPending, pluginCardFailure }}
      actions={{ retryLeague: league => void sports.preview(league), retryStremio: () => void refreshStremio(true), openStremio, openStremioPairing, previewStremio: item => homeState.previewStremio(item), openSteamGame, previewSteamGame: item => homeState.previewSteamGame(item), openItem, removeItem, previewActivityItem, launch, preview, watchLeagueGame, previewLeagueGame: item => homeState.previewLeagueGame(item), openYouTubeItem, previewVideo: item => homeState.previewVideo(item), playMusic: async item => { if (await spotify.playUri(item.uri)) openPlayer(); }, previewMusicItem: item => homeState.previewMusicItem(item), openSteamStore, previewSteamRecommendation: item => homeState.previewSteamRecommendation(item), select, previewMovie, openPluginCard, retryPluginCard }} />
  {/if}
  {#if tab === 'Library'}<StremioLibrary status={stremio} loadError={stremioState.error} loading={stremioState.loading} retry={() => refreshStremio(true)} {native} open={openStremio} connect={openStremioPairing} sync={syncStremio} />{/if}
  {#if tab === 'Sports'}<SportsHome bind:this={sportsHome} {native} {sports} ondetailclose={returnFromSportsDetail} />{/if}
  {#if tab === 'Apps'}
    <PageHeader eyebrow="ON THIS PC" title="Apps" description="Open your streaming apps and keep the ones you use most in reach">
      {#snippet actions()}<button class="text-link" onclick={openApps}>Edit apps <span>→</span></button>{/snippet}
    </PageHeader>
    <AppDock grid order={settings.order} {launch} {launching} {preview} active={previewApp} smoothHover={settings.smoothAppHover} />
  {/if}
  {#if tab === 'Search'}
    <PageHeader eyebrow="FIND YOUR NEXT WATCH" title={submittedQuery ? 'Results' : 'Browse'} description={submittedQuery ? `Results for “${submittedQuery}”. Open a result to watch.` : 'Search titles, teams, or leagues from the bar above, or browse what is popular.'} />
    <section class="content-row" aria-label="Search status">
      <p class="movie-search-status" role="status">{searching ? 'Searching sources…' : searchStatus}</p>
    </section>
    {#if movieResults.length}
      <MediaRow id="search-results" title="Movies - Stremio" items={movieResults} onselect={item => openStremioDetail('movie', item.id)} />
    {/if}
    {#if showResults.length}<MediaRow kind="series" id="show-results" title="Shows - Stremio" items={showResults} onselect={item => openStremioDetail('series', item.id)} />{/if}
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
        {#if matchedSports.length}<SearchSportsRow items={matchedSports} badges={sports.badges} onselect={item => void openSportsDetail(item)} />{/if}
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
      {#if homeState.showsLoading && !popularShows.length}<p role="status">Loading popular shows...</p>{/if}
      <MediaRow kind="series" id="popular-shows" title="Popular shows" items={popularShows} onselect={item => openStremioDetail('series', item.id)} />
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
  </div>
  {#if !playerOpen}<WidgetHost bind:this={widgetHost} widgets={layoutConfig.widgets} context={idle ? 'idle' : 'home'} onposition={(id, position) => { if (settings.widgets[id]) settings.widgets[id].position = position; }} onopen={openPlayer} />{/if}
  {#if playerOpen}
    <NowPlaying bind:this={player} onclose={closePlayer} hasHud={hasPlayerWidgets}>
      {#snippet hud()}<WidgetHud widgets={layoutConfig.widgets} onopen={openPlayer} />{/snippet}
    </NowPlaying>
  {/if}
  {/if}
</main>
{#if setupOpen}<Setup bind:this={setup} {settings} {native} ondone={finishSetup} />{/if}
{#if updateView}<UpdateDialog kind={updateView} onclose={() => void closeUpdate()} />{/if}
{#if layoutPreview}<LayoutReview busy={preferences.reviewBusy} error={preferences.reviewError} editing={preferences.previewFromEditor} plugin={layoutPreview.plugin ?? null} detached={layoutPreview.detached ?? []} keep={() => void keepLayout()} cancel={() => void cancelReview()} />{/if}
{#if details}<DetailsDialog {message} item={details} close={back} launch={() => details && openStremioDetail('movie', details.id)} />{/if}

{#if stremioPairing}
  <dialog {@attach mountStremioDialog} class="details-dialog" aria-label="Connect Stremio" oncancel={(event) => { event.preventDefault(); closeStremioPairing(); }}>
    <button class="close-btn float-right mb-4" onclick={closeStremioPairing} aria-label="Close Stremio connection">&times;</button>
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
