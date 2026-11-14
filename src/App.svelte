<script lang="ts">
  import { onMount } from 'svelte';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { invoke } from '@tauri-apps/api/core';
  import TopNav from './lib/components/TopNav.svelte';
  import MediaRow from './lib/components/MediaRow.svelte';
  import AppDock from './lib/components/AppDock.svelte';
  import SettingsPanel from './lib/components/SettingsPanel.svelte';
  import MusicHome from './lib/components/spotify/MusicHome.svelte';
  import MiniPlayer from './lib/components/spotify/MiniPlayer.svelte';
  import NowPlaying from './lib/components/spotify/NowPlaying.svelte';
  import { spotify } from './lib/state/spotify.svelte';
  import DetailsDialog from './lib/components/DetailsDialog.svelte';
  import StremioRow from './lib/components/StremioRow.svelte';
  import StremioLibrary from './lib/components/StremioLibrary.svelte';
  import ContinueWatching from './lib/components/activity/ContinueWatching.svelte';
  import RecentlyPlayed from './lib/components/activity/RecentlyPlayed.svelte';
  import RecentApps from './lib/components/activity/RecentApps.svelte';
  import { createSettings } from './lib/state/settings.svelte';
  import { createActivity } from './lib/state/activity.svelte';
  import { createNavigation } from './lib/services/navigation';
  import { native, launchApp, saveAppPaths, remoteStatus, configureRemote, onNavigation, onRemote, openYouTube, stremioStatus, stremioSync, stremioOpen, stremioOpenDetail } from './lib/services/tauri';
  import { loadCatalog } from './lib/services/catalog';
  import type { ActivityItem, AppId, ExecutableAppId, MediaItem, RemoteStatus, StremioStatus } from './lib/types';

  const preferences = createSettings();
  const settings = preferences.value;
  const activity = createActivity();
  const activityItems = $derived(activity.items);
  let tab = $state('Home');
  let panel = $state(false);
  let settingsCategory = $state('Display');
  let details = $state<MediaItem | null>(null);
  let message = $state(preferences.error);
  let popular = $state<MediaItem[]>([]);
  let featured = $state<MediaItem[]>([]);
  let remote = $state<RemoteStatus>({ running: false, lan: false, address: '', token: '', origin: '', error: null });
  let playerOpen = $state(false);
  let launching = $state<AppId | null>(null);
  let stremio = $state<StremioStatus | null>(null);
  let previous: HTMLElement | null = null;
  const navigation = createNavigation(back, home);
  const showHome = $derived(tab === 'Home');

  $effect(() => {
    document.documentElement.dataset.accent = settings.accent;
    document.documentElement.dataset.density = settings.density;
    try { localStorage.setItem('luma-v1', JSON.stringify({ accent: settings.accent, density: settings.density, fullscreen: settings.fullscreen, wallpaper: settings.wallpaper, sections: settings.sections, order: settings.order })); }
    catch { message = 'Preferences could not be saved. Try a smaller wallpaper or free local storage.'; }
  });
  $effect(() => {
    if (native) invoke('update_app_state', { view: panel ? 'Settings' : details ? 'Details' : tab, fullscreen: settings.fullscreen }).catch(error => message = String(error));
  });
  $effect(() => {
    if (activity.error) message = activity.error;
  });
  onMount(() => {
    navigation.focus(document.querySelector<HTMLElement>('#home-button') ?? undefined);
    loadCatalog()
      .then(data => { popular = data.popular; featured = data.featured; })
      .catch(() => message = 'Movie catalog could not be loaded. Check your internet connection.');
    let disposed = false;
    const cleanup: (() => void)[] = [];
    async function start() {
      if (!native) { message = 'Browser preview. Native app launching, fullscreen, remote server, and Spotify controls require the desktop app.'; navigation.focus(document.querySelector<HTMLElement>('#home-button') ?? undefined); return; }
      try {
        cleanup.push(await onNavigation(navigation.dispatch));
        cleanup.push(await onRemote(state => remote = state));
        cleanup.push(await activity.start());
        await spotify.initialize();
        cleanup.push(() => spotify.dispose());
        remote = await remoteStatus();
        stremio = await stremioStatus();
        const paths = await invoke<Record<ExecutableAppId, string>>('app_paths');
        settings.paths = paths;
        await detect(false);
        await getCurrentWindow().setFullscreen(settings.fullscreen);
      } catch (error) { message = String(error); }
      finally { if (disposed) cleanup.splice(0).forEach(stop => stop()); }
    }
    void start();
    return () => { disposed = true; cleanup.splice(0).forEach(stop => stop()); };
  });
  function rememberFocus() { previous = document.activeElement as HTMLElement; }
  function back() {
    if (panel) { panel = false; void refreshStremio(); }
    else if (details) details = null;
    else if (playerOpen) { closePlayer(); return; }
    else { home(); return; }
    requestAnimationFrame(() => navigation.focus(previous ?? undefined));
  }
  function home() { playerOpen = false; panel = false; details = null; tab = 'Home'; requestAnimationFrame(() => { window.scrollTo(0, 0); navigation.focus(document.querySelector<HTMLElement>('#home-button') ?? undefined); }); }
  function openPlayer() {
    rememberFocus();
    playerOpen = true;
    requestAnimationFrame(() => navigation.focus(document.querySelector<HTMLElement>('.now-playing-close') ?? undefined));
  }
  function closePlayer() {
    playerOpen = false;
    requestAnimationFrame(() => navigation.focus(previous ?? undefined));
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
    if (event.key === ' ' && !event.repeat && !event.altKey && !event.ctrlKey && !event.metaKey && !(event.target instanceof HTMLInputElement) && !(event.target instanceof HTMLTextAreaElement) && !(event.target instanceof HTMLSelectElement) && !(event.target instanceof HTMLButtonElement) && !(event.target instanceof HTMLAnchorElement)) {
      event.preventDefault();
      if (spotify.playback.canToggle) void spotify.action('toggle');
      return;
    }
    navigation.keydown(event);
  }
  function navigate(value: string) { tab = value; requestAnimationFrame(() => window.scrollTo(0, 0)); }
  function openSettings() { showSettings('Display'); }
  function openIntegrations() { showSettings('Integrations'); }
  function showSettings(category: string) { rememberFocus(); message = ''; settingsCategory = category; panel = true; }
  function select(item: MediaItem) { rememberFocus(); message = ''; details = item; }
  async function launch(id: AppId) {
    if (launching) return;
    if (!native) { message = 'App launching is available in the Luma desktop app.'; return; }
    launching = id;
    try {
      if (id === 'YouTube') { await openYouTube(settings.fullscreen); message = 'YouTube TV opened in its own window. Press Ctrl+W there to return to Luma.'; }
      else { await launchApp(id); message = id === 'Stremio' ? 'Stremio opened inside Luma. Press Ctrl+W there to return.' : `${id} launch requested.`; }
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
      message = `${item.title} removed from local activity history.`;
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
      if (announce) message = apps.map(app => app.path ? `${app.name} found.` : `${app.name} was not found; configure its path manually.`).join(' ');
    } catch (error) { message = String(error); }
  }
  async function savePaths() {
    try { await saveAppPaths(settings.paths); message = 'Approved application paths saved.'; }
    catch (error) { message = String(error); }
  }
  async function network(lan: boolean, origin: string) {
    try { remote = await configureRemote(lan, origin); }
    catch (error) { message = String(error); }
  }
  async function refreshStremio() {
    if (!native) return;
    try { stremio = await stremioStatus(); } catch (error) { message = String(error); }
  }
  async function syncStremio() {
    stremio = await stremioSync();
    message = 'Stremio library synchronized.';
  }
  async function openStremio(id: string) {
    if (!native) { message = 'Opening Stremio is available in the desktop app.'; return; }
    try { await stremioOpen(id); message = 'Opening in Stremio inside Luma. Press Ctrl+W there to return.'; }
    catch (error) { message = String(error); }
  }
  async function openStremioDetail(kind: string, id: string) {
    if (!native) { message = 'Opening Stremio is available in the desktop app.'; return; }
    try { await stremioOpenDetail(kind, id); message = 'Opening in Stremio inside Luma. Press Ctrl+W there to return.'; }
    catch (error) { message = String(error); }
  }
</script>

<svelte:window onkeydown={keydown} />
<main class="app-shell">
  {#if panel}<SettingsPanel {message} {settings} {remote} {native} {activity} startCategory={settingsCategory} close={back} {fullscreen} {savePaths} {network} {detect} report={value => message = value} />
  {:else}
  <TopNav active={tab} {navigate} settings={openSettings} fullscreen={settings.fullscreen} toggle={fullscreen} />
  <div class="h-32" aria-hidden="true"></div>
  {#if showHome && settings.sections.continue}
    <ContinueWatching items={activityItems} trackingEnabled={activity.trackingEnabled} onopen={openItem} onremove={removeItem} />
  {/if}
  {#if showHome && settings.sections.continue && stremio?.continueWatching.length}<StremioRow title="Continue watching on Stremio" items={stremio.continueWatching.slice(0, 10)} open={openStremio} />{/if}
  {#if tab === 'Library'}<StremioLibrary status={stremio} {native} open={openStremio} connect={openIntegrations} sync={syncStremio} />{/if}
  {#if (showHome && settings.sections.apps) || tab === 'Apps'}<AppDock order={settings.order} {launch} customize={openSettings} />{/if}
  {#if (showHome && settings.sections.popular) || tab === 'Watch'}<MediaRow id="popular" title="Popular movies" items={popular} onselect={select} />{/if}
  {#if tab === 'Music'}<MusicHome onopen={openPlayer} onsettings={openIntegrations} />{/if}
  {#if (showHome && settings.sections.featured) || tab === 'Watch'}<MediaRow id="featured" title="Featured for you" items={featured} onselect={select} />{/if}
  {#if tab === 'Watch'}
    <ContinueWatching items={activityItems} trackingEnabled={activity.trackingEnabled} onopen={openItem} onremove={removeItem} />
  {/if}
  {#if (showHome && settings.sections.recent) || tab === 'Watch'}
    <RecentlyPlayed items={activityItems} trackingEnabled={activity.trackingEnabled} onopen={openItem} onremove={removeItem} />
  {/if}
  {#if (showHome && settings.sections.recentApps) || tab === 'Apps'}
    <RecentApps items={activityItems} trackingEnabled={activity.trackingEnabled} onopen={openItem} onremove={removeItem} />
  {/if}
{#if message && !panel && !details}<div role="status" class="notice"><span>{message}</span><button aria-label="Dismiss message" onclick={() => message = ''}>×</button></div>{/if}
  <footer class="footer"><span><span class="footer-mark">L</span> luma tv</span><span>Desktop · v0.1</span><span>Arrow keys to navigate · Enter to select · Escape to go back · Space toggles playback</span></footer>
  {#if !playerOpen}<MiniPlayer onopen={openPlayer} />{/if}
  {#if playerOpen}<NowPlaying onclose={closePlayer} />{/if}
  {/if}
</main>
{#if details}<DetailsDialog {message} item={details} close={back} launch={() => details && openStremioDetail('movie', details.id)} />{/if}
