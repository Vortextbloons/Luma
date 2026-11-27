<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { saverLibrary, type SaverLibrary, type SaverMode, type SaverSettings } from '../services/screensaver';

  let { config = $bindable(), onpreview }: { config: SaverSettings; onpreview: () => void } = $props();
  let media = $state<{ photos: number; video?: string }>();
  let loading = $state(true);
  let busy = $state(false);
  let status = $state('');
  let error = $state('');
  let mediaPicker = $state<HTMLInputElement>();

  const scenes: { id: SaverMode; name: string; icon: string; description: string }[] = [
    { id: 'movies', name: 'Popular movies', icon: '▤', description: 'Movie artwork and titles' },
    { id: 'smart', name: 'Smart', icon: '✦', description: 'Photos or album art' },
    { id: 'photos', name: 'Your photos', icon: '▧', description: 'A slideshow from this PC' },
    { id: 'music', name: 'Album art', icon: '♫', description: 'Art for the track playing' },
    { id: 'room', name: 'Pixel art room', icon: '▥', description: 'A room that changes with the time' },
    { id: 'video', name: 'Silent video', icon: '▶', description: 'Loop a video from this PC' },
    { id: 'clock', name: 'Clock', icon: '◷', description: 'Time and date' },
    { id: 'black', name: 'Black screen', icon: '■', description: 'A blank display' },
  ];
  const usesPhotos = $derived(config.mode === 'smart' || config.mode === 'photos');
  const usesVideo = $derived(config.mode === 'video');
  const hasOverlay = $derived(config.clock || ['movies', 'smart', 'music', 'room'].includes(config.mode)
    || (usesPhotos && config.music));

  function updateSummary(library: SaverLibrary) {
    media = { photos: library.photos.length, video: library.video?.name };
  }

  onMount(() => {
    let active = true;
    void saverLibrary().then(library => {
      if (active) updateSummary(library);
    }).catch(() => {}).finally(() => { if (active) loading = false; });
    return () => { active = false; };
  });

  function selectScene(mode: SaverMode) {
    config.mode = mode;
    status = '';
    error = '';
  }

  async function updateLibrary(kind: 'photos' | 'video', input?: HTMLInputElement, removeButton?: HTMLButtonElement) {
    if (busy) return;
    const files = Array.from(input?.files ?? []);
    if (input) input.value = '';
    if (input && !files.length) return;
    const matching = files.filter(file => file.type.startsWith(kind === 'photos' ? 'image/' : 'video/'));
    status = '';
    error = '';
    if (input && !matching.length) {
      error = kind === 'photos' ? 'Choose image files or a folder with photos.' : 'Choose a video file.';
      return;
    }

    const trigger = input ?? removeButton;
    const restoreFocus = trigger && document.activeElement === trigger;
    busy = true;
    try {
      const library = await saverLibrary();
      if (kind === 'video') library.video = matching[0];
      else library.photos = matching;
      await saverLibrary(library);
      updateSummary(library);
      status = kind === 'video'
        ? library.video ? 'Video saved.' : 'Video removed.'
        : library.photos.length ? 'Photos saved.' : 'Photos removed.';
    } catch {
      error = 'Could not save local media. Try a smaller selection.';
    } finally {
      busy = false;
    }
    if (restoreFocus) {
      await tick();
      if (document.activeElement === document.body) (trigger.isConnected ? trigger : mediaPicker)?.focus();
    }
  }
</script>

<div class="saver-settings">
  <header class="settings-group saver-intro">
    <div>
      <h3>Screensaver</h3>
      <p class="panel-description">Press a button or move the pointer to leave the preview.</p>
    </div>
    <button class="panel-secondary is-primary" aria-disabled={busy} onclick={() => { if (!busy) onpreview(); }}>Preview screensaver</button>
  </header>

  <div class="saver-grid">
    <section class="settings-group saver-scenes" aria-labelledby="saver-scene">
      <h3 id="saver-scene">Choose a scene</h3>
      <div class="scene-grid" role="group" aria-labelledby="saver-scene" data-settings-grid>
        {#each scenes as scene (scene.id)}
          <button class="scene-card" aria-pressed={config.mode === scene.id} onclick={() => selectScene(scene.id)}>
            <span class={['scene-icon', scene.id]} aria-hidden="true">{scene.icon}</span>
            <span class="settings-row-copy"><strong>{scene.name}</strong><span>{scene.description}</span></span>
          </button>
        {/each}
      </div>

      {#if usesPhotos || usesVideo || config.mode === 'movies'}
        <div class="scene-options">
          <h4>{usesPhotos ? 'Photo slideshow' : usesVideo ? 'Video loop' : 'Movie slideshow'}</h4>
          {#if config.mode === 'smart'}
            <p class="panel-description">Smart shows album art during playback. To keep showing photos while music plays, turn on the music overlay.</p>
          {/if}
          {#if usesPhotos || usesVideo}
            <div class="media-tools" aria-busy={busy}>
              <p class="panel-description media-summary">
                {#if loading}Checking saved media…
                {:else if usesVideo && media?.video}{media.video}
                {:else if usesPhotos && media?.photos}{media.photos} {media.photos === 1 ? 'photo' : 'photos'} saved
                {:else if !media}Saved media could not be read.
                {:else}Choose {usesVideo ? 'a video' : 'photos'} to use this scene. The clock appears until then.
                {/if}
              </p>
              {#if usesPhotos}
                <label class="field">{media?.photos ? 'Replace photos' : 'Choose photos'}<input bind:this={mediaPicker} type="file" accept="image/*" multiple disabled={busy || loading} onchange={event => updateLibrary('photos', event.currentTarget)} /></label>
                <label class="field">Choose a photo folder<input type="file" webkitdirectory disabled={busy || loading} onchange={event => updateLibrary('photos', event.currentTarget)} /></label>
              {:else}
                <label class="field">{media?.video ? 'Replace video' : 'Choose a video'}<input bind:this={mediaPicker} type="file" accept="video/*" disabled={busy || loading} onchange={event => updateLibrary('video', event.currentTarget)} /></label>
              {/if}
              {#if usesVideo ? media?.video : media?.photos}
                <button class="text-link" aria-disabled={busy} onclick={event => updateLibrary(usesVideo ? 'video' : 'photos', undefined, event.currentTarget)}>{usesVideo ? 'Remove video' : 'Remove photos'}</button>
              {/if}
              {#if error}<p class="error" role="alert">{error}</p>{/if}
              {#if status}<p class="panel-description" role="status">{status}</p>{/if}
            </div>
          {/if}
          {#if !usesVideo}
            <label class="settings-row field settings-choice">
              <span class="settings-row-copy"><strong>{usesPhotos ? 'Time per photo' : 'Time per movie'}</strong></span>
              <select bind:value={config.seconds}><option value={5}>5 seconds</option><option value={15}>15 seconds</option><option value={30}>30 seconds</option><option value={60}>1 minute</option></select>
            </label>
          {/if}
          {#if usesPhotos}
            <label class="settings-row field settings-choice">
              <span class="settings-row-copy"><strong>Photo sizing</strong></span>
              <select bind:value={config.fit}><option value="contain">Fit on screen</option><option value="cover">Fill the screen</option></select>
            </label>
            <label class="setting-toggle"><span>Shuffle photos</span><input type="checkbox" bind:checked={config.shuffle} /></label>
            <label class="setting-toggle"><span>Show track info over photos</span><input type="checkbox" bind:checked={config.music} /></label>
          {/if}
        </div>
      {:else if config.mode === 'music'}
        <p class="panel-description scene-tip">Start music to see the current album. The clock appears when music is paused.</p>
      {/if}
    </section>

    <div class="saver-controls">
      <section class="settings-group" aria-labelledby="saver-start">
        <h3 id="saver-start">When it starts</h3>
        <div class="saver-options">
          <label class="setting-toggle"><span>Start automatically</span><input type="checkbox" bind:checked={config.enabled} /></label>
          {#if config.enabled}
            <label class="settings-row field settings-choice">
              <span class="settings-row-copy"><strong>Wait time</strong><span>Time without input on Home</span></span>
              <select bind:value={config.minutes}><option value={1}>1 minute</option><option value={3}>3 minutes</option><option value={5}>5 minutes</option><option value={10}>10 minutes</option><option value={20}>20 minutes</option></select>
            </label>
            <label class="setting-toggle"><span><span>Start during music</span><small>Also start in Luma’s music player.</small></span><input type="checkbox" bind:checked={config.player} /></label>
          {/if}
        </div>
      </section>

      {#if config.mode !== 'black'}
        <section class="settings-group" aria-labelledby="saver-display">
          <h3 id="saver-display">On screen</h3>
          <div class="saver-options">
            <label class="settings-row settings-choice brightness-row">
              <span class="settings-row-copy"><strong>Brightness</strong></span>
              <span class="brightness-control">
                <input type="range" min="10" max="100" step="5" bind:value={config.brightness} aria-label="Brightness" aria-valuetext={`${config.brightness}%`} />
                <span aria-hidden="true">{config.brightness}%</span>
              </span>
            </label>
            {#if config.mode !== 'clock'}
              <label class="setting-toggle"><span>Show clock</span><input type="checkbox" bind:checked={config.clock} /></label>
              {#if hasOverlay}
                <label class="settings-row field settings-choice">
                  <span class="settings-row-copy"><strong>Clock and caption position</strong></span>
                  <select bind:value={config.placement}><option value="left">Left</option><option value="right">Right</option></select>
                </label>
              {/if}
            {/if}
          </div>
        </section>
      {/if}
    </div>
  </div>
</div>

<style>
  .saver-settings { container-type: inline-size; }
  .saver-intro { display: flex; align-items: center; justify-content: space-between; flex-wrap: wrap; gap: 14px 24px; }
  .saver-intro .panel-description { margin: 0; }
  .saver-grid { display: grid; grid-template-columns: minmax(0, 1.25fr) minmax(0, 1fr); gap: 14px; align-items: start; }
  .saver-controls { min-width: 0; }
  .scene-grid { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 10px; }
  .scene-card { display: flex; align-items: center; gap: 12px; min-height: 78px; padding: 12px; border: 1px solid var(--line); border-radius: 12px; background: var(--surface-2); text-align: left; }
  .scene-card:hover { border-color: var(--line-strong); background: var(--surface-3); }
  .scene-card[aria-pressed='true'] { border-color: var(--accent); background: var(--accent-soft); }
  .scene-icon { flex: none; display: grid; place-items: center; width: 40px; height: 40px; border-radius: 9px; background: var(--surface-3); color: var(--accent); font-size: 22px; }
  .scene-icon.room { background: #493958; color: #ffd4a5; }
  .scene-icon.black { background: #07080b; color: #515565; }
  .scene-card[aria-pressed='true'] .scene-icon { box-shadow: inset 0 0 0 1px var(--accent); }
  .scene-options, .media-tools, .saver-options { display: grid; gap: 9px; }
  .scene-options { margin-top: 18px; padding-top: 16px; border-top: 1px solid var(--line); }
  .scene-options h4 { margin: 0; font-size: clamp(15px, 1.05vw, 20px); font-weight: 650; }
  .scene-options :is(.panel-description, .error), .media-tools .field { margin: 0; }
  .media-tools { margin-bottom: 6px; }
  .media-tools .text-link { justify-self: start; }
  .media-summary { overflow-wrap: anywhere; }
  .scene-tip { margin-top: 14px; }
  .saver-settings .settings-choice { grid-template-columns: minmax(0, 1fr) minmax(min(144px, 50%), 44%); }
  .brightness-control { display: flex; align-items: center; gap: 10px; min-width: 0; }
  .brightness-control input { width: 100%; min-width: 0; height: 32px; margin: 0; accent-color: var(--accent); cursor: pointer; }
  .brightness-control span { min-width: 4ch; text-align: end; font-variant-numeric: tabular-nums; }
  @container (max-width: 790px) {
    .saver-grid { grid-template-columns: minmax(0, 1fr); gap: 0; }
  }
  @container (max-width: 430px) {
    .scene-card { align-items: flex-start; flex-direction: column; gap: 8px; }
    .saver-settings .brightness-row { grid-template-columns: minmax(0, 1fr); }
  }
</style>
