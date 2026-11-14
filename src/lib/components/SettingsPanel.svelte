<script lang="ts">
  import logo from '../../assets/luma.png';
  import BackgroundSettings from './BackgroundSettings.svelte';
  import FramePairing from './FramePairing.svelte';
  import StremioIntegration from './StremioIntegration.svelte';
  import SpotifyIntegration from './spotify/SpotifyIntegration.svelte';
  import ActivitySettings from './activity/ActivitySettings.svelte';
  import ProviderIcon from './activity/ProviderIcon.svelte';
  import { appProviders } from '../services/activity';
  import type { ActivityState } from '../state/activity.svelte';
  import type { AppId, ExecutableAppId, Settings } from '../types';
  import { moveApp } from '../state/settings.svelte';
  let { settings, native, close, fullscreen, savePaths, detect, message, report, activity, startCategory = 'Display' }: {
    message: string; detect: () => Promise<void>; settings: Settings; native: boolean; close: () => void;
    fullscreen: () => void; savePaths: () => Promise<void>;
    report: (message: string) => void; activity: ActivityState;
    startCategory?: string;
  } = $props();
  const categoryGroups = [
    { name: 'Picture', categories: [
      { name: 'Display', description: 'Fullscreen, layout density, and screen fit.' },
      { name: 'Appearance', description: 'Accent, background, and home rows.' },
    ] },
    { name: 'Library', categories: [
      { name: 'Activity', description: 'Watch history and connected services.' },
      { name: 'Apps', description: 'Find, choose, and arrange apps.' },
    ] },
    { name: 'Connections', categories: [
      { name: 'Integrations', description: 'Spotify and Stremio.' },
      { name: 'Remote & Startup', description: 'Display Box, phone remote, and startup.' },
    ] },
  ] as const;
  const categories: { name: string; description: string }[] = categoryGroups.flatMap(group => [...group.categories]);
  const accents = [
    { id: 'violet', label: 'Violet', color: '#a59bff' },
    { id: 'blue', label: 'Blue', color: '#72bfff' },
    { id: 'mint', label: 'Mint', color: '#88e6c5' },
    { id: 'peach', label: 'Peach', color: '#ffae92' },
  ] as const;
  const homeSections = [
    { id: 'continue', label: 'Continue Watching', detail: 'In-progress movies and shows from this PC and connected services.' },
    { id: 'recent', label: 'Recently played', detail: 'A quick row of the titles you played most recently.' },
    { id: 'recentApps', label: 'Recently used apps', detail: 'Apps you opened from Luma on this PC.' },
    { id: 'popular', label: 'Popular movies', detail: 'The shared movie catalog.' },
    { id: 'apps', label: 'Your apps', detail: 'The app dock on the home screen.' },
    { id: 'featured', label: 'Top-rated movies', detail: 'Movies ranked by IMDb rating.' },
  ] as const;
  let category = $derived(startCategory);
  const activeCategory = $derived(categories.find(item => item.name === category) ?? categories[0]);
  let busy = $state(false);
  let content = $state<HTMLDivElement>();
  function selectCategory(name: string) {
    category = name;
    if (content) content.scrollTop = 0;
  }
  function builtInNote(id: AppId) {
    if (id === 'YouTube') return 'YouTube TV opens in its own Luma window.';
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
  async function wallpaper(event: Event) {
    const file = (event.currentTarget as HTMLInputElement).files?.[0];
    if (!file) return;
    if (!['image/jpeg', 'image/png', 'image/webp', 'image/gif'].includes(file.type) || file.size > 3_000_000) { report('Choose a PNG, JPEG, WebP, or GIF under 3 MB.'); return; }
    const reader = new FileReader();
    reader.onload = () => settings.wallpaper = String(reader.result);
    reader.onerror = () => report('The background image could not be read.');
    reader.readAsDataURL(file);
  }
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
        <section class="settings-group">
          <h3>Screen</h3>
          <p class="panel-description">Choose how Luma fills this display and how much room its cards get.</p>
          <div class="settings-row">
            <div class="settings-row-copy"><strong>Fullscreen</strong><span>{native ? (settings.fullscreen ? 'Luma is filling the screen.' : 'Luma is running in a window.') : 'Available in the desktop application.'}</span></div>
            <button class="panel-secondary" disabled={!native} onclick={fullscreen}>{settings.fullscreen ? 'Switch to windowed mode' : 'Switch to fullscreen'}</button>
          </div>
          <div class="settings-form-grid">
            <label class="field">App tile shape
              <select bind:value={settings.appTileShape}><option value="wide">Wide rectangles</option><option value="square">Squares</option></select>
              <span class="field-hint">Wide cards suit artwork; squares fit more apps.</span>
            </label>
            <label class="field">Layout density
              <select bind:value={settings.density}><option value="comfortable">Comfortable</option><option value="compact">Compact</option></select>
              <span class="field-hint">Compact fits more rows on screen.</span>
            </label>
          </div>
        </section>
      {:else if category === 'Appearance'}
        <section class="settings-group">
          <h3>Accent color</h3>
          <p class="panel-description">Used for focus rings, progress, and selected states across Luma.</p>
          <div class="swatches" role="group" aria-label="Accent color">
            {#each accents as accent (accent.id)}
              <button class={['swatch-choice', { 'is-active': settings.accent === accent.id }]} aria-pressed={settings.accent === accent.id} onclick={() => settings.accent = accent.id as Settings['accent']}>
                <span class="swatch" style:--sw={accent.color}></span>
                <span>{accent.label}</span>
              </button>
            {/each}
          </div>
        </section>
        <section class="settings-group">
          <h3>Background image</h3>
          <p class="panel-description">Use your own image behind the home screen. PNG, JPEG, WebP, or GIF up to 3 MB.</p>
          {#if settings.wallpaper}<img class="wallpaper-preview" src={settings.wallpaper} alt="Current home screen background" />{/if}
          <label class="field">Choose an image<input type="file" accept="image/png,image/jpeg,image/webp,image/gif" onchange={wallpaper} /></label>
          <div class="settings-actions"><button class="text-link" disabled={!settings.wallpaper} onclick={() => settings.wallpaper = ''}>Reset background</button></div>
        </section>
        <section class="settings-group">
          <h3>Home screen rows</h3>
          <p class="panel-description">Choose which rows appear on Home. Hiding a row does not remove its content.</p>
          <div class="settings-toggle-list">
            {#each homeSections as section (section.id)}
              <label class="setting-toggle">
                <span class="setting-toggle-copy"><span>{section.label}</span><small>{section.detail}</small></span>
                <input type="checkbox" bind:checked={settings.sections[section.id]} />
              </label>
            {/each}
          </div>
        </section>
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
                    {:else if settings.paths[id as ExecutableAppId]}
                      <span class="settings-badge is-good">Ready</span>
                    {:else if id === 'Apple TV+' || id === 'Hulu' || id === 'Max' || id === 'Peacock'}
                      <span class="settings-badge is-muted">Opens in a window</span>
                    {:else}
                      <span class="settings-badge is-warn">Choose app</span>
                    {/if}
                  </div>
                  {#if id === 'YouTube' || id === 'Stremio'}
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
              {#if id !== 'YouTube' && id !== 'Stremio'}
                <label class="field app-path-field"><span class="app-path-label">{id} program</span><input bind:value={settings.paths[id as ExecutableAppId]} placeholder="C:\Program Files\App\App.exe" spellcheck="false" /></label>
              {/if}
            {/each}
            <div class="settings-actions settings-actions-end"><button class="panel-secondary is-primary" disabled={!native || busy} onclick={saveApps}>Save app settings</button></div>
          </div>
        </details>
      {:else if category === 'Integrations'}
        <SpotifyIntegration {native} />
        <StremioIntegration {native} />
      {:else}
        <BackgroundSettings {native} />
        <FramePairing {native} />
      {/if}
    </div>
  </div>
</section>
