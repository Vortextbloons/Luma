<script lang="ts">
  import { settingChoices } from '../services/settings';
  import { homeShortcut } from '../services/navigation';
  import { previewSounds, setSoundSettings } from '../services/sound';
  import logo from '../../assets/luma.png';
  import BackgroundSettings from './BackgroundSettings.svelte';
  import UpdateSettings from './UpdateSettings.svelte';
  import TvSettings from './TvSettings.svelte';
  import FramePairing from './FramePairing.svelte';
  import StremioIntegration from './StremioIntegration.svelte';
  import SpotifyIntegration from './spotify/SpotifyIntegration.svelte';
  import ActivitySettings from './activity/ActivitySettings.svelte';
  import ThemeSettings from './ThemeSettings.svelte';
  import HomeLayoutSettings from './HomeLayoutSettings.svelte';
  import WidgetSettings from './WidgetSettings.svelte';
  import AppOrderSettings from './AppOrderSettings.svelte';
  import { defaultHomeRows, moveHomeRow, moveHomeRowTo, toggleHomeRow } from '../services/home';
  import { themeById } from '../services/theme';
  import type { ActivityState } from '../state/activity.svelte';
  import type { ExecutableAppId, MediaItem, Settings } from '../types';
  let { settings = $bindable(), native, close, fullscreen, savePaths, detect, message, activity, startCategory = 'Display', popular = [], featured = [] }: {
    message: string; detect: () => Promise<void>; settings: Settings; native: boolean; close: () => void;
    fullscreen: () => void; savePaths: () => Promise<void>;
    activity: ActivityState;
    startCategory?: string;
    popular?: MediaItem[];
    featured?: MediaItem[];
  } = $props();
  const categoryGroups = [
    { name: 'Picture', categories: [
      { name: 'Display', description: 'Fullscreen, layout density, and screen fit.' },
      { name: 'Theme', description: 'Backgrounds, surfaces, and accent together.' },
      { name: 'Home layout', description: 'Order and hide the rows on Home.' },
      { name: 'Widgets', description: 'Now playing and future widgets.' },
    ] },
    { name: 'TV', categories: [
      { name: 'TV setup', description: 'TV screen, sound output, and PC audio.' },
    ] },
    { name: 'Audio', categories: [
      { name: 'Sound', description: 'Interface sounds and level.' },
    ] },
    { name: 'Library', categories: [
      { name: 'Activity', description: 'Watch history and connected services.' },
      { name: 'Apps', description: 'Find, choose, and arrange apps.' },
    ] },
    { name: 'Connections', categories: [
      { name: 'Integrations', description: 'Spotify and Stremio.' },
      { name: 'Remote & Startup', description: 'Display Box, web remote, and startup.' },
      { name: 'Updates', description: 'Version, downloads, and restart.' },
    ] },
  ] as const;
  const categories: { name: string; description: string }[] = categoryGroups.flatMap(group => [...group.categories]);
  let category = $derived(startCategory);
  const activeCategory = $derived(categories.find(item => item.name === category) ?? categories[0]);
  const theme = $derived(themeById(settings.theme));
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
  let content = $state<HTMLDivElement>();
  function selectCategory(name: string) {
    category = name;
    if (content) content.scrollTop = 0;
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
      <p>Picture, TV, sound, apps, and connections. Changes save automatically.</p>
    </div>
    <div class="settings-header-actions">
      <span class="settings-theme-chip"><i style:--chip-bg={theme.tokens['--bg']} style:--chip-accent={theme.tokens['--accent']}></i>{theme.name}</span>
      <button class="btn btn-glass" onclick={close}>Back to Luma</button>
    </div>
  </header>
  <div class="settings-layout">
    <nav class="settings-categories" aria-label="Settings categories">
      {#each categoryGroups as group (group.name)}
        <p class="settings-category-group">{group.name}</p>
        {#each group.categories as item (item.name)}
          <button class={['settings-category', { selected: category === item.name }]} aria-current={category === item.name ? 'page' : undefined} onclick={() => selectCategory(item.name)}>
            <span class="settings-category-copy"><strong>{item.name}</strong><small>{item.description}</small></span>
            <span class="settings-category-mark" aria-hidden="true">›</span>
          </button>
        {/each}
      {/each}
    </nav>
    <div class="settings-content" {@attach mountContent}>
      <header class="settings-content-header">
        <h2>{activeCategory.name}</h2>
        <p>{activeCategory.description}</p>
      </header>
      {#if message}<p role="status" class="panel-feedback">{message}</p>{/if}
      {#if category === 'Display'}
        <section class="settings-group">
          <h3>Layout</h3>
          <p class="panel-description">Choose how Luma fills this display and how much room its cards get.</p>
          <div class="settings-row">
            <div class="settings-row-copy"><strong>Fullscreen</strong><span>{native ? (settings.fullscreen ? 'Luma is filling the screen.' : 'Luma is running in a window.') : 'Available in the desktop application.'}</span></div>
            <button class="panel-secondary" disabled={!native} onclick={fullscreen}>{settings.fullscreen ? 'Switch to windowed mode' : 'Switch to fullscreen'}</button>
          </div>
          <div class="settings-form-grid">
            {#each settingChoices.filter(choice => choice.group === 'Display') as choice (choice.key)}
              <label class="field">{choice.label}
                <select bind:value={settings[choice.key]}>
                  {#each choice.options as option (option.value)}<option value={option.value}>{option.label}</option>{/each}
                </select>
                <span class="field-hint">{choice.description}</span>
              </label>
            {/each}
          </div>
        </section>
      {:else if category === 'TV setup'}
        <TvSettings {native} />
      {:else if category === 'Sound'}
        <section class="settings-group">
          <h3>Interface sounds</h3>
          <p class="panel-description">Short feedback for focus, select, back, and app launch. Plays on this screen and on the web remote.</p>
          <div class="settings-form-grid">
            {#each settingChoices.filter(choice => choice.group === 'Sound') as choice (choice.key)}
              <label class="field">{choice.label}
                <select bind:value={settings[choice.key]} disabled={choice.key === 'soundLevel' && settings.interfaceSounds === 'off'}>
                  {#each choice.options as option (option.value)}<option value={option.value}>{option.label}</option>{/each}
                </select>
                <span class="field-hint">{choice.description}</span>
              </label>
            {/each}
          </div>
          <div class="settings-actions"><button class="panel-secondary" disabled={settings.interfaceSounds === 'off'} onclick={() => { setSoundSettings(settings.interfaceSounds, settings.soundLevel); previewSounds(); }}>Preview sounds</button></div>
        </section>
      {:else if category === 'Theme'}
        <ThemeSettings {settings} />
      {:else if category === 'Home layout'}
        <HomeLayoutSettings
          rows={settings.home}
          {popular}
          {featured}
          activityItems={activity.items}
          apps={settings.order}
          widget={settings.widgets.spotify}
          onmove={(id, direction) => settings.home = moveHomeRow(settings.home, id, direction)}
          onmoveto={(id, index) => settings.home = moveHomeRowTo(settings.home, id, index)}
          ontoggle={id => settings.home = toggleHomeRow(settings.home, id)}
          onreset={() => settings.home = defaultHomeRows()}
        />
      {:else if category === 'Widgets'}
        <WidgetSettings {settings} />
      {:else if category === 'Activity'}
        <ActivitySettings {activity} {native} />
      {:else if category === 'Apps'}
        <section class="settings-group">
          <h3>Apps on this PC</h3>
          <p class="panel-description">Luma can find apps installed in the usual places. Use App order below to arrange the dock, and open App locations only if an app is missing.</p>
          <div class="settings-actions"><button class="panel-secondary" disabled={!native || busy} onclick={findApps}>Find installed apps</button></div>
        </section>
        <AppOrderSettings {settings} onchange={order => settings.order = order} />
        <details class="settings-group settings-advanced">
          <summary class="settings-advanced-summary" tabindex="0">
            <span class="settings-advanced-copy"><strong>App locations</strong><small>Only needed when Luma cannot find an app automatically.</small></span>
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
      {:else if category === 'Integrations'}
        <SpotifyIntegration {native} />
        <StremioIntegration {native} />
      {:else if category === 'Updates'}
        <UpdateSettings {native} />
      {:else}
        <section class="settings-group">
          <h3>Keyboard</h3>
          <div class="settings-row">
            <div class="settings-row-copy"><strong>Return home</strong><span>Current shortcut: {settings.homeShortcut}. Works while Luma has focus.</span></div>
            <button class="panel-secondary" onkeydown={recordShortcut} onblur={() => recordingShortcut = false} onclick={() => { recordingShortcut = true; shortcutFeedback = ''; }}>{recordingShortcut ? 'Press a shortcut? Escape cancels' : 'Change shortcut'}</button>
            <button class="panel-secondary" disabled={settings.homeShortcut === 'Ctrl+W'} onclick={() => { settings.homeShortcut = 'Ctrl+W'; shortcutFeedback = 'Home shortcut reset to Ctrl+W.'; }}>Reset to Ctrl+W</button>
          </div>
          {#if shortcutFeedback}<p role="status" class="panel-description">{shortcutFeedback}</p>{/if}
        </section>
        <BackgroundSettings {native} />
        <FramePairing {native} />
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
