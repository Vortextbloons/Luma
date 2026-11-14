<script lang="ts">
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
  import ProviderIcon from './activity/ProviderIcon.svelte';
  import ThemeSettings from './ThemeSettings.svelte';
  import HomeLayoutSettings from './HomeLayoutSettings.svelte';
  import WidgetSettings from './WidgetSettings.svelte';
  import { appProviders } from '../services/activity';
  import { defaultHomeRows, moveHomeRow, moveHomeRowTo, toggleHomeRow } from '../services/home';
  import type { ActivityState } from '../state/activity.svelte';
  import type { AppId, ExecutableAppId, MediaItem, Settings } from '../types';
  import { moveApp } from '../state/settings.svelte';
  let { settings, native, close, fullscreen, savePaths, detect, message, activity, startCategory = 'Display', popular = [], featured = [] }: {
    message: string; detect: () => Promise<void>; settings: Settings; native: boolean; close: () => void;
    fullscreen: () => void; savePaths: () => Promise<void>;
    activity: ActivityState;
    startCategory?: string;
    popular?: MediaItem[];
    featured?: MediaItem[];
  } = $props();
  const categoryGroups = [
    { name: 'Picture', categories: [
      { name: 'Display', description: 'Fullscreen, layout, and interface sounds.' },
      { name: 'Theme', description: 'Backgrounds, surfaces, and accent together.' },
      { name: 'Home layout', description: 'Order and hide the rows on Home.' },
      { name: 'Widgets', description: 'Now playing and future widgets.' },
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
  function builtInNote(id: AppId) {
    if (id === 'YouTube') return 'YouTube TV opens in its own Luma window.';
    if (id === 'Steam') return 'Steam opens directly in Big Picture.';
    if (id === 'Stremio') return 'Stremio opens inside Luma.';
    return '';
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
      <p>Picture, apps, and connections for this screen.</p>
    </div>
    <button class="btn btn-glass" onclick={close}>Back to Luma</button>
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
        <TvSettings {native} />
        <section class="settings-group">
          <h3>Interface sounds</h3>
          <p class="panel-description">Short feedback for focus, select, back, and app launch. Plays on this screen and on the web remote.</p>
          <div class="settings-form-grid">
            <label class="field">Sound set
              <select bind:value={settings.interfaceSounds}>
                <option value="real">Real hardware</option>
                <option value="google">Google TV</option>
                <option value="roku">Roku</option>
                <option value="off">Off</option>
              </select>
              <span class="field-hint">Real hardware uses recorded remote and Roku clicks.</span>
            </label>
            <label class="field">Level
              <select bind:value={settings.soundLevel} disabled={settings.interfaceSounds === 'off'}>
                <option value="soft">Soft</option>
                <option value="standard">Standard</option>
                <option value="loud">Loud</option>
              </select>
              <span class="field-hint">Standard is tuned for a quiet room.</span>
            </label>
          </div>
          <div class="settings-actions"><button class="panel-secondary" disabled={settings.interfaceSounds === 'off'} onclick={() => { setSoundSettings(settings.interfaceSounds, settings.soundLevel); previewSounds(); }}>Preview sounds</button></div>
        </section>
        <section class="settings-group">
          <h3>Screen</h3>
          <p class="panel-description">Choose how Luma fills this display and how much room its cards get.</p>
          <div class="settings-row">
            <div class="settings-row-copy"><strong>Fullscreen</strong><span>{native ? (settings.fullscreen ? 'Luma is filling the screen.' : 'Luma is running in a window.') : 'Available in the desktop application.'}</span></div>
            <button class="panel-secondary" disabled={!native} onclick={fullscreen}>{settings.fullscreen ? 'Switch to windowed mode' : 'Switch to fullscreen'}</button>
          </div>
          <div class="settings-form-grid">
            <label class="field">App tile shape
              <select bind:value={settings.appTileShape}><option value="wide">Pills</option><option value="square">Squares</option></select>
              <span class="field-hint">Wide cards suit artwork; squares fit more apps.</span>
            </label>
            <label class="field">Layout density
              <select bind:value={settings.density}><option value="comfortable">Comfortable</option><option value="compact">Compact</option></select>
              <span class="field-hint">Compact fits more rows on screen.</span>
            </label>
          </div>
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
          <p class="panel-description">Luma can find apps installed in the usual places. Arrange them below, and open App locations only if an app is missing.</p>
          <div class="settings-actions"><button class="panel-secondary" disabled={!native || busy} onclick={findApps}>Find installed apps</button></div>
          <div class="app-order-list">
            {#each settings.order as id, index (id)}
              <article class="app-order-row">
                <span class="app-order-index" aria-hidden="true">{index + 1}</span>
                <div class="app-order-main">
                  <div class="app-order-title">
                    <ProviderIcon provider={appProviders[id]} />
                    <strong>{id}</strong>
                    {#if id === 'YouTube'}
                      <span class="settings-badge is-muted">Own window</span>
                    {:else if id === 'Stremio'}
                      <span class="settings-badge is-muted">Inside Luma</span>
                    {:else if id === 'Steam'}
                      <span class="settings-badge is-muted">Big Picture</span>
                    {:else if settings.paths[id as ExecutableAppId]}
                      <span class="settings-badge is-good">Ready</span>
                    {:else if id === 'Apple TV+' || id === 'Hulu' || id === 'Max' || id === 'Peacock'}
                      <span class="settings-badge is-muted">Opens in a window</span>
                    {:else}
                      <span class="settings-badge is-warn">Choose app</span>
                    {/if}
                  </div>
                  {#if id === 'YouTube' || id === 'Steam' || id === 'Stremio'}
                    <p class="app-order-note">{builtInNote(id)}</p>
                  {:else if !settings.paths[id as ExecutableAppId]}
                    <p class="app-order-note">{id === 'Apple TV+' || id === 'Hulu' || id === 'Max' || id === 'Peacock' ? 'No program is needed. It opens in its own window.' : 'Choose this app under App locations if you use it.'}</p>
                  {/if}
                </div>
                <div class="app-order-actions">
                  <button disabled={id === 'Custom' || index === 0} onclick={() => moveApp(settings, id, -1)} aria-label={`Move ${id} earlier`}>Move up</button>
                  <button disabled={id === 'Custom' || index === settings.order.length - 1 || settings.order[index + 1] === 'Custom'} onclick={() => moveApp(settings, id, 1)} aria-label={`Move ${id} later`}>Move down</button>
                </div>
              </article>
            {/each}
          </div>
        </section>
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
