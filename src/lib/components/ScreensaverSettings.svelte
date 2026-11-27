<script lang="ts">
  import { saverLibrary, type SaverSettings } from '../services/screensaver';
  let { config, onpreview }: { config: SaverSettings; onpreview: () => void } = $props();
  let status = $state('');
  let busy = $state(false);

  async function updateLibrary(event?: Event, video = false) {
    const input = event?.currentTarget as HTMLInputElement | undefined;
    const files = Array.from(input?.files ?? []);
    if (input) input.value = '';
    if (event && !files.length) return;
    busy = true;
    try {
      const library = await saverLibrary();
      if (video) library.video = files.find(file => file.type.startsWith('video/'));
      else library.photos = files.filter(file => file.type.startsWith('image/'));
      await saverLibrary(library);
      status = video ? (library.video?.name ?? 'Video cleared') : `${library.photos.length} photos saved`;
    } catch { status = 'Could not save local media. Try a smaller selection.'; }
    finally { busy = false; }
  }
</script>

<section class="settings-group">
  <h3>Screensaver</h3>
  <label class="setting-toggle">Start automatically <input type="checkbox" bind:checked={config.enabled} /></label>
  <label class="setting-toggle">Start after
    <select bind:value={config.minutes}><option value={1}>1 minute</option><option value={3}>3 minutes</option><option value={5}>5 minutes</option><option value={10}>10 minutes</option><option value={20}>20 minutes</option></select>
  </label>
  <label class="setting-toggle">Allow in Luma’s music player <input type="checkbox" bind:checked={config.player} /></label>
  <p class="panel-description">Starts on Home and, if enabled, in the music player. Press a button or move the pointer to return.</p>
  <label class="setting-toggle">Scene
    <select bind:value={config.mode}>
      <option value="movies">Popular movies</option><option value="smart">Smart</option><option value="photos">Photos</option><option value="music">Album art gallery</option><option value="room">Pixel art room</option><option value="video">Silent video loop</option><option value="clock">Clock</option><option value="black">Black screen</option>
    </select>
  </label>
  {#if config.mode !== 'black'}
    <label class="setting-toggle">
      <span>Brightness</span>
      <span class="brightness-control">
        <input type="range" min="10" max="100" step="5" bind:value={config.brightness} aria-label="Brightness" aria-valuetext={`${config.brightness}%`} />
        <span aria-hidden="true">{config.brightness}%</span>
      </span>
    </label>
    {#if config.mode !== 'clock'}<label class="setting-toggle">Show clock <input type="checkbox" bind:checked={config.clock} /></label>{/if}
  {/if}
  {#if config.mode === 'smart' || config.mode === 'photos'}
    <p class="panel-description">Smart shows your photos when nothing is playing. With the music overlay off, it switches to full-screen album art during playback. Choose a folder or individual photos to replace the current collection.</p>
    <label class="setting-toggle">Choose photos <input type="file" accept="image/*" multiple disabled={busy} onchange={event => updateLibrary(event)} /></label>
    <label class="setting-toggle">Choose photo folder <input type="file" webkitdirectory disabled={busy} onchange={event => updateLibrary(event)} /></label>
    <button class="secondary" disabled={busy} onclick={() => updateLibrary()}>Clear photos</button>
    <label class="setting-toggle">Shuffle <input type="checkbox" bind:checked={config.shuffle} /></label>
    <label class="setting-toggle">Time per photo <select bind:value={config.seconds}><option value={5}>5 seconds</option><option value={15}>15 seconds</option><option value={30}>30 seconds</option><option value={60}>1 minute</option></select></label>
    <label class="setting-toggle">Photo sizing <select bind:value={config.fit}><option value="contain">Fit</option><option value="cover">Fill</option></select></label>
    <label class="setting-toggle">Show music over photos <input type="checkbox" bind:checked={config.music} /></label>
  {/if}
  {#if config.mode === 'movies'}
    <label class="setting-toggle">Time per movie <select bind:value={config.seconds}><option value={5}>5 seconds</option><option value={15}>15 seconds</option><option value={30}>30 seconds</option><option value={60}>1 minute</option></select></label>
  {/if}
  {#if config.mode === 'video'}
    <label class="setting-toggle">Choose video <input type="file" accept="video/*" disabled={busy} onchange={event => updateLibrary(event, true)} /></label>
    <button class="secondary" disabled={busy} onclick={() => updateLibrary(undefined, true)}>Clear video</button>
  {/if}
  {#if config.mode !== 'black' && config.mode !== 'clock'}
    <label class="setting-toggle">Overlay position <select bind:value={config.placement}><option value="left">Left</option><option value="right">Right</option></select></label>
  {/if}
  {#if status}<p role="status" class="panel-description">{status}</p>{/if}
  <button class="secondary" disabled={busy} onclick={onpreview}>Preview now</button>
</section>

<style>
  .brightness-control { display: flex; align-items: center; gap: 12px; }
  .brightness-control input { width: clamp(70px, 20vw, 220px); margin: 0; padding: 0; background: transparent; border: 0; }
  .brightness-control span { min-width: 4ch; text-align: end; font-variant-numeric: tabular-nums; }
</style>
