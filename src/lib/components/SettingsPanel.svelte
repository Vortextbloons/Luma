<script lang="ts">
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
  import { themeById } from '../services/theme';
  import type { HomeDraft } from '../services/customization';
  import type { PluginsState } from '../services/plugins';
  import type { ActivityState } from '../state/activity.svelte';
  import type { ExecutableAppId, MediaItem, Settings } from '../types';

  let { settings = $bindable(), homeDraft = $bindable(null), revision = 0, onlayoutpreview, onlayoutrestore, native, close, fullscreen, savePaths, detect, message, activity, startCategory = 'Display', popular = [], featured = [], plugins = null, pluginsLoading = false, pluginsError = '', onpluginsreload = async () => {} }: {
    homeDraft?: HomeDraft | null;
    revision?: number;
    onlayoutpreview?: (draft: HomeDraft) => Promise<void>;
    onlayoutrestore?: () => Promise<void>;
    message: string; detect: () => Promise<void>; settings: Settings; native: boolean; close: () => void;
    fullscreen: () => void; savePaths: () => Promise<void>;
    activity: ActivityState;
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
  const activeCategory = $derived(settingsCategory(category));
  const activeSection = $derived(activeCategory.sections.find(item => item.id === section) ?? activeCategory.sections[0]);
  const theme = $derived(themeById(settings.theme));
  let resetNote = $state('');
  let recordingShortcut = $state(false);
  let shortcutFeedback = $state('');

  function recordShortcut(event: KeyboardEvent) {
    if (!recordingShortcut) return;
    event.preventDefault();
    event.stopPropagation();
    if (event.key === 'Escape') { recordingShortcut = false; shortcutFeedback = ''; return; }
    const shortcut = homeShortcut(event);
    if (!shortcut) { shortcutFeedback = 'Hold Ctrl or Alt and press a letter, number, Home, or function key other than F11.'; return; }
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
  $effect(() => { if (activeSection.id === 'layout' && !homeDraft) reloadHome(); });

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

  let content = $state<HTMLDivElement>();
  function selectCategory(name: string) {
    const next = settingsCategories.find(item => item.name === name);
    if (!next) return;
    category = next.name;
    section = next.sections[0].id;
    resetNote = '';
    layoutError = '';
    if (content) content.scrollTop = 0;
    requestAnimationFrame(() => document.querySelector<HTMLElement>(`.settings-categories .settings-category[data-category="${next.name}"]`)?.focus());
  }
  function selectSection(id: string) {
    section = id;
    resetNote = '';
    layoutError = '';
    if (content) content.scrollTop = 0;
    requestAnimationFrame(() => document.querySelector<HTMLElement>(`.settings-section-tab[data-section="${id}"]`)?.focus());
  }
  function resetLayout() {
    resetSettingChoices(settings, ['density', 'appTileShape', 'textSize', 'reducedMotion']);
    resetNote = 'Layout size and motion reset. Home rows, widgets, and theme were not changed.';
  }
  function resetSound() {
    resetSettingChoices(settings, ['interfaceSounds', 'soundLevel']);
    resetNote = 'Sound settings reset. Other settings were not changed.';
  }
  function resetSystem() {
    settings.fastAppSwitch = true;
    resetNote = 'System preferences reset. Startup and tray behavior were not changed.';
  }

  async function findApps() {
    busy = true;
    try { await detect(); } finally { busy = false; }
  }
  async function saveApps() {
    busy = true;
    try { await savePaths(); } finally { busy = false; }
  }
  function mountPage(element: HTMLElement) { (element.querySelector<HTMLButtonElement>('.settings-category.selected') ?? element.querySelector<HTMLButtonElement>('button'))?.focus(); window.scrollTo(0, 0); }
  function mountContent(element: HTMLDivElement) { content = element; }
</script>

<section {@attach mountPage} class="settings-page" aria-labelledby="settings-title">
  <header class="settings-page-header">
    <div class="settings-brand">
      <img class="settings-brand-logo" src={logo} alt="Luma TV" />
      <h1 id="settings-title">Settings</h1>
      <p>Appearance, display, apps, remotes, history, system, plugins, and customization. Most changes save automatically; Home layout previews before saving.</p>
    </div>
    <div class="settings-header-actions">
      <span class="settings-theme-chip"><i style:--chip-bg={theme.tokens['--bg']} style:--chip-accent={theme.tokens['--accent']}></i>{theme.name}</span>
      <button class="btn btn-glass" onclick={close}>Back to Luma</button>
    </div>
  </header>
  <div class="settings-layout">
    <nav class="settings-categories" aria-label="Settings categories">
      {#each settingsCategories as item (item.name)}
        <button class={['settings-category', { selected: category === item.name }]} data-category={item.name} aria-current={category === item.name ? 'page' : undefined} onclick={() => selectCategory(item.name)}>
          <span class="settings-category-copy"><strong>{item.name}</strong><small>{item.description}</small></span>
          <span class="settings-category-mark" aria-hidden="true">›</span>
        </button>
      {/each}
    </nav>
    <div class="settings-content" {@attach mountContent}>
      <header class="settings-content-header">
        <h2>{activeCategory.name}</h2>
        <p>{activeCategory.description}</p>
      </header>
      {#if activeCategory.sections.length > 1}
        <nav class="settings-sections" aria-label={`${activeCategory.name} sections`}>
          {#each activeCategory.sections as item (item.id)}
            <button class={['settings-section-tab', { selected: section === item.id }]} data-section={item.id} aria-current={section === item.id ? 'page' : undefined} onclick={() => selectSection(item.id)}>{item.name}</button>
          {/each}
        </nav>
        <p class="settings-section-note">{activeSection.description}</p>
      {/if}
      {#if message}<p role="status" class="panel-feedback">{message}</p>{/if}
      {#if activeCategory.name === 'Appearance & Home'}
        {#if activeSection.id === 'theme'}
          <PluginOwnershipNotice {plugins} fields={['theme']} onmanage={() => selectCategory('Plugins')} />
          <ThemeSettings {settings} />
        {:else if activeSection.id === 'layout'}
          <PluginOwnershipNotice {plugins} fields={['density', 'appTileShape', 'textSize', 'reducedMotion', 'homeColumns', 'home']} onmanage={() => selectCategory('Plugins')} />
          <section class="settings-group">
            <h3>Layout size</h3>
            <p class="panel-description">Choose how much room cards get and how large text appears on this screen. These changes save automatically.</p>
            <SettingsChoices {settings} section="layout" />
            <div class="settings-actions"><button class="panel-secondary" onclick={resetLayout}>Reset size and motion</button></div>
            {#if resetNote}<p class="panel-description" role="status">{resetNote}</p>{/if}
          </section>
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
              widget={settings.widgets.spotify}
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
        {:else if activeSection.id === 'widgets'}
          <PluginOwnershipNotice {plugins} fields={['widgets']} onmanage={() => selectCategory('Plugins')} />
          <WidgetSettings {settings} />
        {/if}
      {:else if activeCategory.name === 'Plugins'}
        <PluginSettings {native} {revision} {plugins} loading={pluginsLoading} error={pluginsError} reload={onpluginsreload} />
      {:else if activeCategory.name === 'MCP customization'}
        <CustomizationSettings {native} {revision} section={activeSection.id} onchanged={() => { homeDraft = null; layoutError = ''; }} />
      {:else if activeCategory.name === 'Display & Sound'}
        {#if activeSection.id === 'display'}
          <section class="settings-group">
            <h3>Fullscreen</h3>
            <p class="panel-description">Choose how Luma fills this display. Fullscreen changes apply immediately.</p>
            <div class="settings-row">
              <div class="settings-row-copy"><strong>Luma window</strong><span>{native ? (settings.fullscreen ? 'Luma is filling the screen.' : 'Luma is running in a window.') : 'Available in the desktop application.'}</span></div>
              <button class="panel-secondary" disabled={!native} onclick={fullscreen}>{settings.fullscreen ? 'Switch to windowed mode' : 'Switch to fullscreen'}</button>
            </div>
            <SettingsChoices {settings} section="display" />
          </section>
          <ViewingSettings {native} />
        {:else if activeSection.id === 'tv'}
          <TvSettings {native} />
        {:else if activeSection.id === 'sound'}
          <section class="settings-group">
            <h3>Interface sounds</h3>
            <p class="panel-description">Short feedback for focus, select, back, and app launch. Plays on this screen and on the web remote.</p>
            <SettingsChoices {settings} section="sound" disabledChoice={choice => choice.key === 'soundLevel' && settings.interfaceSounds === 'off'} />
            <div class="settings-actions">
              <button class="panel-secondary" disabled={settings.interfaceSounds === 'off'} onclick={() => { setSoundSettings(settings.interfaceSounds, settings.soundLevel); previewSounds(); }}>Preview sounds</button>
              <button class="panel-secondary" onclick={resetSound}>Reset sound settings</button>
            </div>
            {#if resetNote}<p class="panel-description" role="status">{resetNote}</p>{/if}
          </section>
        {/if}
      {:else if activeCategory.name === 'Apps & Services'}
        {#if activeSection.id === 'apps'}
          <section class="settings-group">
            <h3>Apps on this PC</h3>
            <p class="panel-description">Luma can find apps installed in the usual places. Use App order below to arrange the dock, and open App locations only if an app is missing. App locations save when you choose Save app settings.</p>
            <div class="settings-actions"><button class="panel-secondary" disabled={!native || busy} onclick={findApps}>Find installed apps</button></div>
          </section>
          <AppOrderSettings {settings} onchange={order => settings.order = order} />
          <details class="settings-group settings-advanced">
            <summary class="settings-advanced-summary" tabindex="0">
              <span class="settings-advanced-copy"><strong>App locations</strong><small>Only needed when Luma cannot find an app automatically. Changes here are saved with Save app settings.</small></span>
              <span class="settings-advanced-mark" aria-hidden="true">›</span>
            </summary>
            <div class="settings-advanced-body">
              <p class="panel-description">A program is the .exe file Luma opens. Leave a location blank if you use the built-in version or do not use that app.</p>
              {#each settings.order as id (id)}
                {#if id !== 'YouTube' && id !== 'Steam' && id !== 'Stremio'}
                  <label class="field app-path-field"><span class="app-path-label">{id} program</span><input bind:value={settings.paths[id as ExecutableAppId]} placeholder="C:\Program Files\App\App.exe" spellcheck="false" /></label>
                {/if}
              {/each}
              <div class="settings-actions settings-actions-end"><button class="panel-secondary is-primary" disabled={!native || busy} onclick={saveApps}>Save app settings</button></div>
            </div>
          </details>
        {:else if activeSection.id === 'accounts'}
          <SpotifyIntegration {native} />
          <StremioIntegration {native} />
          <ActivitySettings {activity} {native} view="accounts" />
        {/if}
      {:else if activeCategory.name === 'Remotes & Devices'}
        {#if activeSection.id === 'remote'}
          <section class="settings-group">
            <h3>Remote settings</h3>
            <p class="panel-description">Pair a phone or Display Box to control Luma from another screen. A phone remote can keep its own theme; layout, sound, and other choices apply to Luma.</p>
          </section>
          <FramePairing {native} />
        {:else if activeSection.id === 'keyboard'}
          <section class="settings-group">
            <h3>Keyboard</h3>
            <div class="settings-row">
              <div class="settings-row-copy"><strong>Return home</strong><span>Current shortcut: {settings.homeShortcut}. Works while Luma has focus.</span></div>
              <button class="panel-secondary" onkeydown={recordShortcut} onblur={() => recordingShortcut = false} onclick={() => { recordingShortcut = true; shortcutFeedback = ''; }}>{recordingShortcut ? 'Press a shortcut? Escape cancels' : 'Change shortcut'}</button>
              <button class="panel-secondary" disabled={settings.homeShortcut === 'Ctrl+W'} onclick={() => { settings.homeShortcut = 'Ctrl+W'; shortcutFeedback = 'Home shortcut reset to Ctrl+W and saved.'; }}>Reset to Ctrl+W</button>
            </div>
            {#if shortcutFeedback}<p role="status" class="panel-description">{shortcutFeedback}</p>{/if}
          </section>
        {/if}
      {:else if activeCategory.name === 'History & Privacy'}
        {#if activeSection.id === 'recording'}
          <ActivitySettings {activity} {native} view="history" />
        {:else if activeSection.id === 'clear'}
          <ActivitySettings {activity} {native} view="clear" />
        {/if}
      {:else if activeCategory.name === 'System'}
        {#if activeSection.id === 'startup'}
          <section class="settings-group">
            <h3>Background apps</h3>
            <p class="panel-description">YouTube and Stremio pause when you switch away. Fast app switching keeps them loaded in the background so they can reopen without reloading.</p>
            <div class="settings-toggle-list">
              <label class="setting-toggle">
                <span class="setting-toggle-copy"><span>Fast app switching</span><small>{native ? 'Keeps YouTube and Stremio in memory after you leave them. Uses more RAM while Luma runs. When off, Luma closes them to save memory.' : 'Available in the desktop application.'}</small></span>
                <input type="checkbox" bind:checked={settings.fastAppSwitch} disabled={!native} />
              </label>
            </div>
            <div class="settings-actions"><button class="panel-secondary" onclick={resetSystem}>Reset system settings</button></div>
            {#if resetNote}<p class="panel-description" role="status">{resetNote}</p>{/if}
          </section>
          <BackgroundSettings {native} />
        {:else if activeSection.id === 'updates'}
          <UpdateSettings {native} />
        {/if}
      {/if}
    </div>
  </div>
</section>

<style>
  .settings-header-actions { display: flex; align-items: center; justify-content: flex-end; gap: 10px; flex-wrap: wrap; flex: none; }
  .settings-theme-chip { display: inline-flex; align-items: center; gap: 8px; min-height: 38px; padding: 0 14px 0 10px; border: 1px solid var(--line); border-radius: 99px; background: color-mix(in srgb, var(--surface) 62%, transparent); color: var(--text-dim); font-size: 12.5px; font-weight: 600; white-space: nowrap; }
  .settings-theme-chip i { width: 16px; height: 16px; border-radius: 50%; background: var(--chip-bg); border: 2px solid var(--chip-accent); box-shadow: 0 0 12px color-mix(in srgb, var(--chip-accent) 38%, transparent); }
  @media (max-width: 640px) {
    .settings-header-actions { width: 100%; justify-content: space-between; }
    .settings-theme-chip { min-height: 34px; font-size: 12px; }
  }
</style>
