<script lang="ts">
  import ScreensaverSettings from './ScreensaverSettings.svelte';
  import type { SaverSettings } from '../services/screensaver';
  import { untrack } from 'svelte';
  import { homeShortcut } from '../services/navigation';
  import { previewSounds, setSoundSettings } from '../services/sound';
  import { resetSettingChoices } from '../services/settings';
  import logo from '../../assets/luma.png';
  import BackgroundSettings from './BackgroundSettings.svelte';
  import UpdateSettings from './UpdateSettings.svelte';
  import ViewingSettings from './ViewingSettings.svelte';
  import TvSettings from './TvSettings.svelte';
  import FramePairing from './FramePairing.svelte';
  import ControllerSettings from './ControllerSettings.svelte';
  import StremioIntegration from './StremioIntegration.svelte';
  import SpotifyIntegration from './spotify/SpotifyIntegration.svelte';
  import ActivitySettings from './activity/ActivitySettings.svelte';
  import ThemeSettings from './ThemeSettings.svelte';
  import HomeLayoutSettings from './HomeLayoutSettings.svelte';
  import WidgetSettings from './WidgetSettings.svelte';
  import AppOrderSettings from './AppOrderSettings.svelte';
  import SettingsChoices from './SettingsChoices.svelte';
  import CustomizationSettings from './CustomizationSettings.svelte';
  import PluginSettings from './PluginSettings.svelte';
  import PluginOwnershipNotice from './PluginOwnershipNotice.svelte';
  import { resolveSettingsTarget, settingsCategories, settingsCategory } from '../services/settingsNavigation';
  import { defaultHomeRows, moveHomeRow, moveHomeRowTo, toggleHomeRow } from '../services/home';
  import type { HomeDraft } from '../services/customization';
  import type { PluginsState } from '../services/plugins';
  import type { ActivityState } from '../state/activity.svelte';
  import type { SteamState } from '../state/steam.svelte';
  import type { ExecutableAppId, MediaItem, Settings } from '../types';

  let { saver = $bindable(), onsaverpreview, settings = $bindable(), homeDraft = $bindable(null), revision = 0, onlayoutpreview, onlayoutrestore, native, close, fullscreen, savePaths, detect, message, activity, steam, oncontrollercapture, onupdatenotes, startCategory = 'Appearance', popular = [], featured = [], plugins = null, pluginsLoading = false, pluginsError = '', onpluginsreload = async () => {} }: {
    homeDraft?: HomeDraft | null;
    revision?: number;
    onlayoutpreview?: (draft: HomeDraft) => Promise<void>;
    onlayoutrestore?: () => Promise<void>;
    saver: SaverSettings; onsaverpreview: () => void;
    message: string; detect: () => Promise<void>; settings: Settings; native: boolean; close: () => void;
    fullscreen: () => void; savePaths: () => Promise<void>;
    activity: ActivityState;
    steam?: SteamState;
    oncontrollercapture?: (handler: ((buttonIndex: number) => boolean) | null) => void;
    onupdatenotes: (kind: 'installed' | 'update') => void;
    startCategory?: string;
    popular?: MediaItem[];
    featured?: MediaItem[];
    plugins?: PluginsState | null;
    pluginsLoading?: boolean;
    pluginsError?: string;
    onpluginsreload?: () => Promise<void>;
  } = $props();

  const initial = untrack(() => resolveSettingsTarget(startCategory));
  let category = $state(initial.category);
  let section = $state(initial.section);
  const categoryIcons: Record<string, string[]> = {
    Appearance: ['M4.5 8.5h9v12h-9z', 'M10.5 4.5h9v12h-9z'],
    'Display & Sound': ['M3.5 5.5h17v10h-17z', 'M9 20h6', 'M12 15.5V20'],
    'Apps & Accounts': ['M4.5 4.5h6v6h-6z', 'M13.5 4.5h6v6h-6z', 'M4.5 13.5h6v6h-6z', 'M13.5 13.5h6v6h-6z'],
    Controls: ['M8.5 3.5h7a2 2 0 0 1 2 2v13a2 2 0 0 1-2 2h-7a2 2 0 0 1-2-2v-13a2 2 0 0 1 2-2z', 'M11 17.5h2'],
    'Watch History': ['M12 4.5a7.5 7.5 0 1 0 7.4 8.8', 'M12 8.5V12l2.6 1.8', 'M3.6 8.4V4.2h4.2'],
    'System': ['M12 15a3 3 0 1 0 0-6 3 3 0 0 0 0 6z', 'M12 3.4v2.2', 'M12 18.4v2.2', 'M4.6 12H2.4', 'M21.6 12h-2.2', 'M6.6 6.6 5.1 5.1', 'M18.9 18.9l-1.5-1.5', 'M17.4 6.6l1.5-1.5', 'M5.1 18.9l1.5-1.5'],
    'Add-ons': ['M9 3.5v3.6', 'M15 3.5v3.6', 'M6.5 7.1h11v7.4a5.5 5.5 0 0 1-11 0z'],
    Home: ['M3 11l9-7 9 7', 'M5 10v10h14V10', 'M10 20v-6h4v6'],
  };
  const activeCategory = $derived(settingsCategory(category));
  let pluginPanel = $state<{ consumeBack: () => boolean }>();
  let widgetPanel = $state<{ consumeBack: () => boolean }>();

  export function consumeBack() {
    return (pluginPanel?.consumeBack() || widgetPanel?.consumeBack()) ?? false;
  }
  export function cycleCategory(direction: -1 | 1) {
    const index = settingsCategories.findIndex(item => item.name === category);
    if (index < 0) return;
    selectCategory(settingsCategories[(index + direction + settingsCategories.length) % settingsCategories.length].name);
  }
  const activeSection = $derived(activeCategory.sections.find(item => item.id === section) ?? activeCategory.sections[0]);
  let resetNote = $state('');
  let recordingShortcut = $state(false);
  let shortcutFeedback = $state('');

  function recordShortcut(event: KeyboardEvent) {
    if (!recordingShortcut) return;
    event.preventDefault();
    event.stopPropagation();
    if (event.key === 'Escape') { recordingShortcut = false; shortcutFeedback = ''; return; }
    const shortcut = homeShortcut(event);
    if (!shortcut) { shortcutFeedback = 'Try Ctrl or Alt with a letter, number, Home, or function key (except F11).'; return; }
    settings.homeShortcut = shortcut;
    recordingShortcut = false;
    shortcutFeedback = `Home shortcut saved: ${shortcut}.`;
  }

  let busy = $state(false);
  let layoutBusy = $state(false);
  let layoutError = $state('');
  function reloadHome() {
    homeDraft = { home: JSON.parse(JSON.stringify(settings.home)), homeColumns: settings.homeColumns, baseRevision: revision };
    layoutError = '';
  }

  async function previewHome() {
    if (!homeDraft || !onlayoutpreview || layoutBusy) return;
    layoutBusy = true;
    layoutError = '';
    try { await onlayoutpreview(homeDraft); }
    catch (error) { layoutError = error && typeof error === 'object' && 'message' in error ? String(error.message) : String(error); }
    finally { layoutBusy = false; }
  }

  async function restoreHome() {
    if (!onlayoutrestore || layoutBusy) return;
    layoutBusy = true;
    layoutError = '';
    try { await onlayoutrestore(); reloadHome(); }
    catch (error) { layoutError = error && typeof error === 'object' && 'message' in error ? String(error.message) : String(error); }
    finally { layoutBusy = false; }
  }

  let page: HTMLElement;
  let content = $state<HTMLDivElement>();

  function openPlugins() { selectCategory('Add-ons'); }

  function selectCategory(name: string) {
    const next = settingsCategories.find(item => item.name === name);
    if (!next) return;
    if (category !== next.name) {
      category = next.name;
      section = next.sections[0].id;
      if (section === 'layout' && !homeDraft) reloadHome();
    }
    resetNote = '';
    layoutError = '';
    requestAnimationFrame(() => {
      if (content) content.scrollTop = 0;
      page?.querySelector<HTMLElement>('.settings-category.selected')?.focus();
    });
  }
  function selectSection(id: string) {
    section = id;
    if (section === 'layout' && !homeDraft) reloadHome();
    resetNote = '';
    layoutError = '';
    requestAnimationFrame(() => {
      if (content) content.scrollTop = 0;
      page?.querySelector<HTMLElement>('.settings-section-tab.selected')?.focus();
    });
  }
  function resetLayout() {
    resetSettingChoices(settings, ['density', 'appTileShape', 'textSize', 'reducedMotion', 'smoothAppHover']);
    resetNote = 'Text, spacing, and motion reset.';
  }
  function resetSound() {
    resetSettingChoices(settings, ['interfaceSounds', 'soundLevel']);
    resetNote = 'Menu sounds reset.';
  }
  function resetSystem() {
    settings.fastAppSwitch = true;
    resetNote = 'Fast app switching reset.';
  }

  async function findApps() {
    if (busy) return;
    busy = true;
    try { await detect(); } finally { busy = false; }
  }
  async function saveApps() {
    if (busy) return;
    busy = true;
    try { await savePaths(); } finally { busy = false; }
  }
  let shortcutName = $state('');
  let shortcutExe = $state('');
  let shortcutDir = $state('');
  let shortcutStatus = $state('');
  let shortcutBusy = $state(false);
  $effect(() => {
    if (native && activeSection.id === 'steam') {
      void steam?.loadShortcuts();
    }
  });
  async function addSteamGame(event: SubmitEvent) {
    event.preventDefault();
    if (!steam || shortcutBusy) return;
    shortcutBusy = true;
    shortcutStatus = '';
    try {
      const added = await steam.addShortcut(shortcutName, shortcutExe, shortcutDir);
      shortcutName = '';
      shortcutExe = '';
      shortcutDir = '';
      shortcutStatus = `Added ${added.name} to Steam.`;
    } catch (error) {
      shortcutStatus = error instanceof Error ? error.message : String(error);
    } finally {
      shortcutBusy = false;
    }
  }
  async function removeSteamGame(appid: number, name: string) {
    if (!steam || shortcutBusy) return;
    shortcutBusy = true;
    shortcutStatus = '';
    try {
      await steam.removeShortcut(appid);
      shortcutStatus = `Removed ${name} from Steam shortcuts.`;
    } catch (error) {
      shortcutStatus = error instanceof Error ? error.message : String(error);
    } finally {
      shortcutBusy = false;
    }
  }
  function mountPage(element: HTMLElement) {
    page = element;
    if (initial.section === 'layout') untrack(() => { if (!homeDraft) reloadHome(); });
    (page.querySelector<HTMLButtonElement>('.settings-section-tab.selected') ?? page.querySelector<HTMLButtonElement>('.settings-category.selected'))?.focus();
    window.scrollTo(0, 0);
  }
  function mountContent(element: HTMLDivElement) { content = element; }
</script>

<section {@attach mountPage} class="settings-page" aria-labelledby="settings-title">
  <header class="settings-page-header">
    <div class="settings-brand">
      <img class="settings-brand-logo" src={logo} alt="Luma TV" />
      <h1 id="settings-title">Settings</h1>
    </div>
    <button class="btn btn-glass" onclick={close}>Back</button>
  </header>
  <div class="settings-layout">
    <nav class="settings-categories" aria-label="Settings categories">
      {#each settingsCategories as item (item.name)}
        <button class={['settings-category', { selected: category === item.name }]} data-category={item.name} aria-current={category === item.name ? 'page' : undefined} onclick={() => selectCategory(item.name)}>
          <svg class="settings-category-icon" viewBox="0 0 24 24" aria-hidden="true">{#each categoryIcons[item.name] ?? [] as path (path)}<path d={path} />{/each}</svg>
          <span class="settings-category-copy"><strong>{item.name}</strong></span>
          <span class="settings-category-mark" aria-hidden="true">›</span>
        </button>
      {/each}
    </nav>
    <div class="settings-content" {@attach mountContent}>
      <header class="settings-content-header">
        <h2>{activeCategory.name}</h2>
      </header>
      {#if activeCategory.sections.length > 1}
        <nav class="settings-sections" aria-label={`${activeCategory.name} sections`}>
          {#each activeCategory.sections as item (item.id)}
            <button class={['settings-section-tab', { selected: section === item.id }]} data-section={item.id} aria-current={section === item.id ? 'page' : undefined} onclick={() => selectSection(item.id)}>{item.name}</button>
          {/each}
        </nav>
      {/if}
      {#if message}<p role="status" class="panel-feedback">{message}</p>{/if}
      {#if activeSection.id === 'theme'}
        <PluginOwnershipNotice {plugins} fields={['theme']} onmanage={openPlugins} />
        <ThemeSettings {settings} />
      {:else if activeSection.id === 'readability'}
        <PluginOwnershipNotice {plugins} fields={['density', 'appTileShape', 'textSize', 'reducedMotion', 'smoothAppHover']} onmanage={openPlugins} />
        <section class="settings-group">
          <h3>Text, spacing & motion</h3>
          <SettingsChoices {settings} section="layout" />
          <div class="settings-actions"><button class="panel-secondary" onclick={resetLayout}>Reset size and motion</button></div>
          {#if resetNote}<p class="panel-description" role="status">{resetNote}</p>{/if}
        </section>
      {:else if activeSection.id === 'screensaver'}
        <ScreensaverSettings bind:config={saver} onpreview={onsaverpreview} />
      {:else if activeSection.id === 'layout'}
        <PluginOwnershipNotice {plugins} fields={['homeColumns', 'home']} onmanage={openPlugins} />
        {#if layoutError}<p role="alert" class="panel-feedback">{layoutError}</p>{/if}
        {#if homeDraft}
          <HomeLayoutSettings
            rows={homeDraft.home}
            columns={homeDraft.homeColumns}
            busy={layoutBusy}
            {popular}
            {featured}
            activityItems={activity.items}
            apps={settings.order}
            widgets={settings.widgets}
            onmove={(id, direction) => homeDraft && (homeDraft.home = moveHomeRow(homeDraft.home, id, direction))}
            onmoveto={(id, index) => homeDraft && (homeDraft.home = moveHomeRowTo(homeDraft.home, id, index))}
            ontoggle={id => homeDraft && (homeDraft.home = toggleHomeRow(homeDraft.home, id))}
            oncolumns={columns => homeDraft && (homeDraft.homeColumns = columns)}
            onspan={(id, span) => homeDraft && (homeDraft.home = homeDraft.home.map(row => row.id === id ? { ...row, span } : row))}
            onreset={() => { if (homeDraft) { homeDraft.home = defaultHomeRows(); homeDraft.homeColumns = 1; } }}
            onpreview={native ? previewHome : undefined}
            onreload={reloadHome}
            onrestore={native ? restoreHome : undefined}
          />
        {/if}
      {:else if activeSection.id === 'order'}
        <AppOrderSettings {settings} onchange={order => settings.order = order} />
      {:else if activeSection.id === 'widgets'}
        <PluginOwnershipNotice {plugins} fields={['widgets']} onmanage={openPlugins} />
        <WidgetSettings bind:this={widgetPanel} {settings} />
      {:else if activeSection.id === 'providers'}
        <ActivitySettings {activity} {native} view="providers" />
      {:else if activeSection.id === 'display'}
        <section class="settings-group">
          <h3>Fullscreen</h3>
          <div class="settings-row">
            <div class="settings-row-copy"><strong>Luma window</strong><span>{native ? (settings.fullscreen ? 'Fullscreen' : 'Windowed') : 'Available in the desktop application.'}</span></div>
            <button class="panel-secondary" disabled={!native} onclick={fullscreen}>{settings.fullscreen ? 'Use a window' : 'Go fullscreen'}</button>
          </div>
        </section>
      {:else if activeSection.id === 'tv'}
        <TvSettings {native} />
      {:else if activeSection.id === 'sound'}
        <section class="settings-group">
          <h3>Interface sounds</h3>
          <SettingsChoices {settings} section="sound" disabledChoice={choice => choice.key === 'soundLevel' && settings.interfaceSounds === 'off'} />
          <div class="settings-actions">
            <button class="panel-secondary" disabled={settings.interfaceSounds === 'off'} onclick={() => { setSoundSettings(settings.interfaceSounds, settings.soundLevel); previewSounds(); }}>Preview sounds</button>
            <button class="panel-secondary" onclick={resetSound}>Reset sound settings</button>
          </div>
          {#if resetNote}<p class="panel-description" role="status">{resetNote}</p>{/if}
        </section>
      {:else if activeSection.id === 'viewing'}
        <ViewingSettings {native} />
      {:else if activeSection.id === 'apps'}
        <section class="settings-group">
          <h3>Apps on this PC</h3>
          <p class="panel-description">Find apps on this PC or choose their locations below.</p>
          <div class="settings-actions">
            <button class="panel-secondary" aria-disabled={busy} disabled={!native} onclick={findApps}>Find installed apps</button>
            <button class="panel-secondary is-primary" aria-disabled={busy} disabled={!native} onclick={saveApps}>Save app settings</button>
          </div>
        </section>
        <details class="settings-group settings-advanced">
          <summary class="settings-advanced-summary" tabindex="0">
            <span class="settings-advanced-copy"><strong>App locations</strong><small>Choose a program if Luma can't find it.</small></span>
            <span class="settings-advanced-mark" aria-hidden="true">›</span>
          </summary>
          <div class="settings-advanced-body">
            <div class="settings-fields">
              {#each (Object.keys(settings.paths) as ExecutableAppId[]).filter(id => id !== 'Steam' && id !== 'Stremio') as id (id)}
                <label class="field">{id}<input bind:value={settings.paths[id]} disabled={!native || busy} placeholder="C:\Program Files\App\App.exe" spellcheck="false" /></label>
              {/each}
            </div>
            <div class="settings-actions"><button class="panel-secondary is-primary" aria-disabled={busy} disabled={!native} onclick={saveApps}>Save app locations</button></div>
          </div>
        </details>
      {:else if activeSection.id === 'steam'}
        {#if steam && native}
          <section class="settings-group">
            <h3>Custom Steam games</h3>
            <p class="panel-description">Add a Windows game to your Steam library.</p>
            <form class="settings-fields" onsubmit={addSteamGame}>
              <label class="field app-path-field"><span class="app-path-label">Game .exe path</span><input bind:value={shortcutExe} placeholder="C:\Games\My Game\Game.exe" spellcheck="false" required /></label>
              <label class="field app-path-field"><span class="app-path-label">Game name (optional)</span><input bind:value={shortcutName} placeholder="Defaults to the .exe name" spellcheck="false" /></label>
              <label class="field app-path-field"><span class="app-path-label">Start in folder (optional)</span><input bind:value={shortcutDir} placeholder="Defaults to the .exe's folder" spellcheck="false" /></label>
              <div class="settings-actions"><button type="submit" class="panel-secondary is-primary" disabled={!native || shortcutBusy}>Add to Steam</button></div>
            </form>
            {#if shortcutStatus}<p role="status" class="panel-description">{shortcutStatus}</p>{/if}
            {#if steam.shortcuts.length}
              <div class="steam-shortcut-list">
                {#each steam.shortcuts as shortcut (shortcut.appid)}
                  <div class="settings-row">
                    <div class="settings-row-copy">
                      <strong>{shortcut.name}</strong>
                      <span>{shortcut.exe} · Start in: {shortcut.startDir || 'Default'}</span>
                    </div>
                    <button class="panel-secondary" disabled={!native || shortcutBusy} onclick={() => removeSteamGame(shortcut.appid, shortcut.name)}>Remove</button>
                  </div>
                {/each}
              </div>
            {/if}
          </section>
        {:else}
          <p class="panel-description">Steam shortcuts are available in the desktop app.</p>
        {/if}
      {:else if activeSection.id === 'accounts'}
        <SpotifyIntegration {native} />
        <StremioIntegration {native} />
        <ActivitySettings {activity} {native} view="accounts" />
      {:else if activeSection.id === 'remote'}
        <FramePairing {native} />
      {:else if activeSection.id === 'controller'}
        <ControllerSettings bind:settings oncapturechange={oncontrollercapture} />
      {:else if activeSection.id === 'keyboard'}
        <section class="settings-group">
          <h3>Keyboard</h3>
          <div class="settings-row">
            <div class="settings-row-copy"><strong>Return home</strong><span>Current shortcut: {settings.homeShortcut}. Works while Luma has focus.</span></div>
            <button class="panel-secondary" onkeydown={recordShortcut} onblur={() => recordingShortcut = false} onclick={() => { recordingShortcut = true; shortcutFeedback = ''; }}>{recordingShortcut ? 'Press the keys now' : 'Change shortcut'}</button>
            <button class="panel-secondary" disabled={settings.homeShortcut === 'Ctrl+W'} onclick={() => { settings.homeShortcut = 'Ctrl+W'; shortcutFeedback = 'Home shortcut reset.'; }}>Reset to Ctrl+W</button>
          </div>
          {#if shortcutFeedback}<p role="status" class="panel-description">{shortcutFeedback}</p>{/if}
        </section>
      {:else if activeSection.id === 'recording'}
        <ActivitySettings {activity} {native} view="history" />
      {:else if activeSection.id === 'startup'}
        <BackgroundSettings {native} />
        <section class="settings-group">
          <h3>Background apps</h3>
          <div class="settings-toggle-list">
            <label class="setting-toggle">
              <span class="setting-toggle-copy"><span>Fast app switching</span><small>{native ? 'Keeps YouTube and Stremio ready to reopen faster, using more memory.' : 'Available in the desktop application.'}</small></span>
              <input type="checkbox" bind:checked={settings.fastAppSwitch} disabled={!native} />
            </label>
            <label class="setting-toggle">
              <span class="setting-toggle-copy"><span>Keep launcher ready</span><small>Reopens Luma from the tray faster by keeping its browser runtime in memory.</small></span>
              <input type="checkbox" bind:checked={settings.keepLauncherReady} disabled={!native} />
            </label>
          </div>
          <div class="settings-actions"><button class="panel-secondary" onclick={resetSystem}>Reset fast app switching</button></div>
          {#if resetNote}<p class="panel-description" role="status">{resetNote}</p>{/if}
        </section>
      {:else if activeSection.id === 'updates'}
        <UpdateSettings {native} onnotes={onupdatenotes} />
      {:else if activeSection.id === 'installed'}
        <PluginSettings bind:this={pluginPanel} {native} {revision} {plugins} loading={pluginsLoading} error={pluginsError} reload={onpluginsreload} />
      {:else if activeSection.id === 'ai' || activeSection.id === 'versions'}
        {#key activeSection.id}
          <CustomizationSettings {native} {revision} section={activeSection.id} onchanged={() => { homeDraft = null; layoutError = ''; }} />
        {/key}
      {/if}
    </div>
  </div>
</section>
