<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { initializeUpdates } from './lib/state/updates.svelte';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { invoke } from '@tauri-apps/api/core';
  import TopNav from './lib/components/TopNav.svelte';
  import FramePairing from './lib/components/FramePairing.svelte';
  import MediaRow from './lib/components/MediaRow.svelte';
  import VideoRow from './lib/components/VideoRow.svelte';
  import AppDock from './lib/components/AppDock.svelte';
  import SettingsPanel from './lib/components/SettingsPanel.svelte';
  import MusicHome from './lib/components/spotify/MusicHome.svelte';
  import MiniPlayer from './lib/components/spotify/MiniPlayer.svelte';
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
  import { appProviders } from './lib/services/activity';
  import { native, launchApp, saveAppPaths, onNavigation, onOpenMusic, openYouTube, openYouTubeVideo, youtubeFeed, onYouTubeFeed, stremioStatus, stremioSync, stremioOpen, stremioOpenDetail } from './lib/services/tauri';
  import { loadCatalog, searchMovies } from './lib/services/catalog';
  import type { ActivityItem, AppId, ExecutableAppId, MediaItem, StremioStatus, YouTubeFeedItem } from './lib/types';

  const preferences = createSettings();
  const settings = preferences.value;
  const activity = createActivity();
  const activityItems = $derived(activity.items);
  let tab = $state('Home');
  let panel = $state(false);
  let remotePairing = $state(false);
  let settingsCategory = $state('Display');
  let details = $state<MediaItem | null>(null);
  let message = $state(preferences.error);
  let movieQuery = $state('');
  let movieResults = $state<MediaItem[]>([]);
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
  let popular = $state<MediaItem[]>([]);
  let featured = $state<MediaItem[]>([]);
  let playerOpen = $state(false);
  let musicHome = $state<MusicHome>();
  let player = $state<NowPlaying>();
  let launching = $state<AppId | null>(null);
  let stremioPairing = $state(false);
  let stremio = $state<StremioStatus | null>(null);
  let previewApp = $state<AppId | null>(null);
  let youTubeFeed = $state<YouTubeFeedItem[]>([]);
  let previous: HTMLElement | null = null;
  let playerPrevious: HTMLElement | null = null;
  const navigation = createNavigation(back, home, () => settings.homeShortcut);
  const showHome = $derived(tab === 'Home');
  const youtubePicks = $derived(showHome && previewApp === 'YouTube' && youTubeFeed.length > 0);

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
    document.documentElement.dataset.accent = settings.accent;
    document.documentElement.dataset.density = settings.density;
    document.documentElement.dataset.appTileShape = settings.appTileShape;
    try { localStorage.setItem('luma-v1', JSON.stringify({ homeShortcut: settings.homeShortcut, accent: settings.accent, density: settings.density, appTileShape: settings.appTileShape, fullscreen: settings.fullscreen, wallpaper: settings.wallpaper, sections: settings.sections, order: settings.order })); }
    catch { message = 'Preferences could not be saved. Try a smaller wallpaper or free local storage.'; }
  });
  $effect(() => {
    if (native) invoke('update_app_state', { view: panel ? 'Settings' : details ? 'Details' : tab, fullscreen: settings.fullscreen }).catch(error => message = String(error));
  });
  $effect(() => {
    if (activity.error) message = activity.error;
  });
  onMount(() => {
    void initializeUpdates();
    navigation.focus(document.querySelector<HTMLElement>('#home-button') ?? undefined);
    loadCatalog()
      .then(data => { popular = data.popular; featured = data.featured; })
      .catch(() => message = 'Movie catalog could not be loaded. Check your internet connection.');
    let disposed = false;
    const cleanup: (() => void)[] = [];
    async function start() {
      if (!native) { message = 'Browser preview. Native app launching, fullscreen, web remote, and Spotify controls require the desktop app.'; navigation.focus(document.querySelector<HTMLElement>('#home-button') ?? undefined); return; }
      try {
        const stopMusic = await onOpenMusic(() => { if (!disposed) void openMusic(); });
        if (disposed) { stopMusic(); return; }
        cleanup.push(stopMusic);
        const stopNavigation = await onNavigation(navigation.dispatch);
        if (disposed) { stopNavigation(); return; }
        cleanup.push(stopNavigation);
        await onYouTubeFeed(items => youTubeFeed = items).then(stop => cleanup.push(stop)).catch(() => {});
        youtubeFeed().then(items => youTubeFeed = items).catch(() => {});
        cleanup.push(await activity.start());
        await spotify.initialize();
        if (spotify.connection.installed) void spotify.start();
        cleanup.push(() => spotify.dispose());
        stremio = await stremioStatus();
        const paths = await invoke<Record<ExecutableAppId, string>>('app_paths');
        settings.paths = paths;
        await detect(false);
        await getCurrentWindow().setFullscreen(settings.fullscreen);
        cleanup.push(await getCurrentWindow().onResized(async () => { const value = await getCurrentWindow().isFullscreen(); if (value !== settings.fullscreen) settings.fullscreen = value; }));
      } catch (error) { message = String(error); }
      finally { if (disposed) cleanup.splice(0).forEach(stop => stop()); }
    }
    void start();
    return () => { disposed = true; cleanup.splice(0).forEach(stop => stop()); };
  });
  function rememberFocus() { previous = document.activeElement as HTMLElement; }
  function back() {
    if (remotePairing) { closeRemotePairing(); return; }
    if (stremioPairing) { closeStremioPairing(); return; }
    if (panel) { panel = false; void refreshStremio(); }
    else if (details) details = null;
    else if (playerOpen) { if (!player?.closeOverlay()) closePlayer(); return; }
    else if (tab === 'Music' && musicHome?.closeDetail()) return;
    else { home(); return; }
    requestAnimationFrame(() => navigation.focus(previous ?? undefined));
  }
  function home() { remotePairing = false; previewApp = null; playerOpen = false; panel = false; details = null; tab = 'Home'; requestAnimationFrame(() => { window.scrollTo(0, 0); navigation.focus(document.querySelector<HTMLElement>('#home-button') ?? undefined); }); }
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
  function animateView(_view: string) {
    return (element: HTMLElement) => {
      if (window.matchMedia('(prefers-reduced-motion: reduce)').matches) return;
      const animation = element.animate(
        [{ opacity: .72, transform: 'translateY(6px)' }, { opacity: 1, transform: 'none' }],
        { duration: 140, easing: 'cubic-bezier(.22,.8,.24,1)' },
      );
      return () => animation.cancel();
    };
  }
  function navigate(value: string) { if (value === 'Music') { void openMusic(); return; } previewApp = null; tab = value; requestAnimationFrame(() => window.scrollTo(0, 0)); }
  async function openMusic() {
    while (musicHome?.closeDetail()) {}
    previewApp = null;
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
      else { await launchApp(id); message = id === 'Stremio' ? 'Stremio opened inside Luma. Use Back to Luma when you are done.' : `${id} opened.`; }
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
  async function refreshStremio() {
    if (!native) return;
    try { stremio = await stremioStatus(); } catch (error) { message = String(error); }
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
  {#if panel}<SettingsPanel {message} {settings} {native} {activity} startCategory={settingsCategory} close={back} {fullscreen} {savePaths} {detect} report={value => message = value} />
  {:else}
  <TopNav active={tab} {navigate} settings={openSettings} connect={openRemotePairing} fullscreen={settings.fullscreen} toggle={fullscreen} />
  <div class="topbar-offset" aria-hidden="true"></div>
  <div class="view-content" {@attach animateView(tab)}>
  {#if showHome && settings.sections.continue}
    <ContinueWatching items={activityItems} trackingEnabled={activity.trackingEnabled} onopen={openItem} onremove={removeItem} focusProvider={previewApp ? appProviders[previewApp] : null} />
  {/if}
  {#if showHome && settings.sections.continue && stremio?.continueWatching.length}<StremioRow title="Continue watching on Stremio" items={stremio.continueWatching.slice(0, 10)} open={openStremio} />{/if}
  {#if tab === 'Library'}<StremioLibrary status={stremio} {native} open={openStremio} connect={openStremioPairing} sync={syncStremio} />{/if}
  {#if (showHome && settings.sections.apps) || tab === 'Apps'}<AppDock order={settings.order} {launch} customize={openApps} preview={id => previewApp = id} />{/if}
  {#if tab === 'Watch'}
    <section class="content-row" aria-label="Search movies">
      <div class="row-heading"><div><h2>Search movies</h2><p>Find a title from Stremio's catalog</p></div></div>
      <form onsubmit={searchWatch} class="movie-search-field" role="search">
        <svg class="movie-search-icon" viewBox="0 0 24 24" aria-hidden="true"><circle cx="11" cy="11" r="7" /><path d="m20 20-3.5-3.5" /></svg>
        <input id="movie-search" type="search" bind:value={movieQuery} placeholder="Search by movie title" aria-label="Search movies" autocomplete="off" spellcheck="false" />
        <button type="submit" class="movie-search-submit">Search</button>
      </form>
      <p class="movie-search-status" role="status">{searching ? 'Searching movies…' : searchStatus}</p>
    </section>
    {#if movieResults.length}<MediaRow id="search-results" title="Search results" items={movieResults} onselect={item => openStremioDetail('movie', item.id)} />{/if}
  {/if}
  {#if youtubePicks && (settings.sections.popular || settings.sections.featured)}
    <VideoRow items={youTubeFeed} open={openYouTubeItem} />
  {:else}
    {#if (showHome && settings.sections.popular) || tab === 'Watch'}<MediaRow id="popular" title="Popular movies" items={popular} onselect={select} />{/if}
    {#if (showHome && settings.sections.featured) || tab === 'Watch'}<MediaRow id="featured" title="Top-rated movies" items={featured} onselect={select} />{/if}
  {/if}
  {#if tab === 'Music'}<MusicHome bind:this={musicHome} onopen={openPlayer} onsettings={openIntegrations} />{/if}
  {#if tab === 'Watch'}
    <ContinueWatching items={activityItems} trackingEnabled={activity.trackingEnabled} onopen={openItem} onremove={removeItem} />
  {/if}
  {#if (showHome && settings.sections.recent) || tab === 'Watch'}
    <RecentlyPlayed items={activityItems} trackingEnabled={activity.trackingEnabled} onopen={openItem} onremove={removeItem} />
  {/if}
  {#if (showHome && settings.sections.recentApps) || tab === 'Apps'}
    <RecentApps items={activityItems} trackingEnabled={activity.trackingEnabled} onopen={openItem} onremove={removeItem} />
  {/if}
  </div>
{#if message && !panel && !details}<div role="status" class="notice"><span>{message}</span><button aria-label="Dismiss message" onclick={() => message = ''}>×</button></div>{/if}
  <footer class="footer"><span><span class="footer-mark">L</span> luma tv</span><span>Desktop · v0.1</span><span>Arrow keys to navigate · Enter to select · Escape to go back · Space toggles playback</span></footer>
  {#if !playerOpen}<MiniPlayer onopen={openPlayer} />{/if}
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

<style>
  .remote-dialog { width: min(820px, calc(100vw - 48px)); max-height: calc(100dvh - 48px); margin: auto; padding: clamp(24px, 3vw, 40px); overflow-y: auto; border: 1px solid #ffffff26; border-radius: 24px; background: #15171e; color: var(--text); box-shadow: 0 24px 90px #0008; }
  .remote-dialog::backdrop { background: #05060aaa; backdrop-filter: blur(8px); }
  .remote-dialog-heading { display: flex; align-items: center; justify-content: space-between; gap: 24px; }
  .remote-dialog-heading h2 { margin: 8px 0 0; font-size: clamp(26px, 2.4vw, 36px); }
</style>
