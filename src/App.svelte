<script lang="ts">
  import { onMount } from 'svelte';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { invoke } from '@tauri-apps/api/core';
  import TopNav from './lib/components/TopNav.svelte';
  import Hero from './lib/components/Hero.svelte';
  import MediaRow from './lib/components/MediaRow.svelte';
  import AppDock from './lib/components/AppDock.svelte';
  import MediaControls from './lib/components/MediaControls.svelte';
  import SettingsPanel from './lib/components/SettingsPanel.svelte';
  import DetailsDialog from './lib/components/DetailsDialog.svelte';
  import { createSettings } from './lib/state/settings.svelte';
  import { createNavigation } from './lib/services/navigation';
  import { native, launchApp, saveAppPaths, mediaState, mediaControl, remoteStatus, configureRemote, onNavigation, onMedia, onRemote } from './lib/services/tauri';
  import { continueWatching, featured, heroArtwork } from './lib/demo';
  import type { AppId, MediaItem, MediaState, RemoteStatus } from './lib/types';

  const preferences = createSettings();
  const settings = preferences.value;
  let tab = $state('Home');
  let panel = $state(false);
  let details = $state<MediaItem | null>(null);
  let message = $state(preferences.error);
  let media = $state<MediaState>({ available: false, title: '', artist: '', playing: false, canToggle: false, canNext: false, canPrevious: false, error: null });
  let remote = $state<RemoteStatus>({ running: false, lan: false, address: '', token: '', origin: '', error: null });
  let mediaBusy = $state(false);
  let launching = $state<AppId | null>(null);
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
  onMount(() => {
    let disposed = false;
    const cleanup: (() => void)[] = [];
    async function start() {
      if (!native) { message = 'Browser preview. Native app launching, fullscreen, remote server, and Spotify controls require the desktop app.'; navigation.focus(document.querySelector<HTMLElement>('#home-button') ?? undefined); return; }
      try {
        cleanup.push(await onNavigation(navigation.dispatch));
        cleanup.push(await onMedia(state => media = state));
        cleanup.push(await onRemote(state => remote = state));
        media = await mediaState();
        remote = await remoteStatus();
        const paths = await invoke<Record<AppId, string>>('app_paths');
        settings.paths = paths;
        await detect(false);
        await getCurrentWindow().setFullscreen(settings.fullscreen);
      } catch (error) { message = String(error); }
      finally { if (disposed) cleanup.splice(0).forEach(stop => stop()); }
      navigation.focus(document.querySelector<HTMLElement>('#home-button') ?? undefined);
    }
    void start();
    return () => { disposed = true; cleanup.splice(0).forEach(stop => stop()); };
  });
  function rememberFocus() { previous = document.activeElement as HTMLElement; }
  function back() {
    if (panel) panel = false;
    else if (details) details = null;
    else { home(); return; }
    requestAnimationFrame(() => navigation.focus(previous ?? undefined));
  }
  function home() { panel = false; details = null; tab = 'Home'; requestAnimationFrame(() => { window.scrollTo(0, 0); navigation.focus(document.querySelector<HTMLElement>('#home-button') ?? undefined); }); }
  function navigate(value: string) { tab = value; requestAnimationFrame(() => window.scrollTo(0, 0)); }
  function openSettings() { rememberFocus(); message = ''; panel = true; }
  function select(item: MediaItem) { rememberFocus(); message = ''; details = item; }
  async function launch(id: AppId) {
    if (launching) return;
    if (!native) { message = 'App launching is available in the Luma desktop app.'; return; }
    launching = id;
    try { await launchApp(id); message = `${id} launch requested.`; } catch (error) { message = String(error); }
    finally { launching = null; }
  }
  async function fullscreen() {
    try { const value = !await getCurrentWindow().isFullscreen(); await getCurrentWindow().setFullscreen(value); settings.fullscreen = value; }
    catch (error) { message = String(error); }
  }
  async function detect(announce = true) {
    try {
      const apps = await invoke<{ name: AppId; path: string | null }[]>('detect_apps');
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
  async function refreshMedia() {
    if (!native) return;
    try { media = await mediaState(); } catch (error) { message = String(error); }
  }
  async function control(action: 'toggle' | 'previous' | 'next') {
    mediaBusy = true;
    try { await mediaControl(action); await refreshMedia(); } catch (error) { message = String(error); }
    finally { mediaBusy = false; }
  }
</script>

<svelte:window onkeydown={navigation.keydown} />
<main class="app-shell">
  <TopNav active={tab} {navigate} settings={openSettings} />
  {#if showHome}
    <Hero wallpaper={settings.wallpaper} launch={() => launch('Stremio')} details={() => select({ title: 'Killers of the Flower Moon', subtitle: 'Crime drama · 2023 · 3h 26m', artwork: heroArtwork })} />
  {:else}<div class="h-32" aria-hidden="true"></div>{/if}
  {#if (showHome && settings.sections.continue) || tab === 'Watch'}<MediaRow id="continue" title="Continue watching" items={continueWatching} onselect={select} />{/if}
  {#if (showHome && settings.sections.apps) || tab === 'Apps'}<AppDock order={settings.order} {launch} customize={openSettings} />{/if}
  {#if tab === 'Music'}<MediaControls state={media} busy={mediaBusy} {control} launch={() => launch('Spotify')} refresh={refreshMedia} />{/if}
  {#if (showHome && settings.sections.featured) || tab === 'Watch'}<MediaRow id="featured" title="Featured for you" items={featured} onselect={select} />{/if}
{#if message && !panel && !details}<div role="status" class="notice"><span>{message}</span><button aria-label="Dismiss message" onclick={() => message = ''}>×</button></div>{/if}
  <footer class="footer"><span><span class="footer-mark">L</span> luma tv</span><span>Desktop · v0.1</span><span>Arrow keys to navigate · Enter to select · Escape to go back</span></footer>
</main>
{#if panel}<SettingsPanel {message} {settings} {remote} {native} close={back} {fullscreen} {savePaths} {network} {detect} report={value => message = value} />{/if}
{#if details}<DetailsDialog {message} item={details} close={back} launch={() => launch('Stremio')} />{/if}
