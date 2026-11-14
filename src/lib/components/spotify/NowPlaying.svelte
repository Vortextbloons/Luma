<script lang="ts">
  import MusicIcon from './MusicIcon.svelte';
  import { onMount, tick } from 'svelte';
  import { spotify } from '../../state/spotify.svelte';
  import DevicePicker from './DevicePicker.svelte';
  import TrackList from './TrackList.svelte';
  import PlaybackControls from './PlaybackControls.svelte';

  let { onclose }: { onclose: () => void } = $props();
  const playback = $derived(spotify.playback);
  const connection = $derived(spotify.connection);
  let showQueue = $state(false);
  let showDevices = $state(false);
  export function closeOverlay() {
    if (showQueue) { showQueue = false; document.getElementById('queue-toggle')?.focus(); return true; }
    if (showDevices) { showDevices = false; document.getElementById('devices-toggle')?.focus(); return true; }
    return false;
  }
  let queueLoading = $state(false);

  onMount(() => {
    requestAnimationFrame(() => document.querySelector<HTMLElement>('.now-playing-close')?.focus());
  });

  function time(value?: number) {
    if (value == null) return '0:00';
    const seconds = Math.max(0, Math.floor(value / 1000));
    return `${Math.floor(seconds / 60)}:${String(seconds % 60).padStart(2, '0')}`;
  }

  function action(name: string, value?: string) { void spotify.action(name, value); }

  function seek(event: Event) {
    action('seek', String(Math.round(Number((event.currentTarget as HTMLInputElement).value))));
  }

  function volume(event: Event) {
    action('volume', String(Math.round(Number((event.currentTarget as HTMLInputElement).value))));
  }

  function repeat() {
    const next = !playback.repeat || playback.repeat === 'off' ? 'context' : playback.repeat === 'context' ? 'track' : 'off';
    action('repeat', next);
  }

  async function queue() {
    showQueue = !showQueue;
    showDevices = false;
    if (showQueue) {
      queueLoading = true;
      try { await spotify.loadQueue(); }
      finally { queueLoading = false; }
      await tick();
      document.getElementById('player-queue')?.scrollIntoView({ block: 'start' });
      document.querySelector<HTMLElement>('#player-queue button')?.focus();
    }
  }

  async function devices() {
    showDevices = !showDevices; showQueue = false;
    if (showDevices) {
      await tick();
      document.getElementById('player-devices')?.scrollIntoView({ block: 'start' });
      document.querySelector<HTMLElement>('#player-devices button')?.focus();
    }
  }

  async function refreshQueue() {
    queueLoading = true;
    try { await spotify.loadQueue(); }
    finally { queueLoading = false; }
  }


</script>

<dialog open class="now-playing" aria-modal="true" aria-labelledby="now-playing-title">
  {#if playback.artworkUrl}<div class="now-playing-ambient" style:background-image={`url("${playback.artworkUrl}")`} aria-hidden="true"></div>{:else}<div class="now-playing-ambient" aria-hidden="true"></div>{/if}
  <header class="now-playing-header">
    <span class="panel-kicker">NOW PLAYING · SPOTIFY</span>
    <button class="now-playing-close" aria-label="Close player" onclick={onclose}><MusicIcon name="x" /></button>
  </header>

  <div class="now-playing-layout">
    <div class="now-playing-art">
      {#if playback.artworkUrl}<img src={playback.artworkUrl} alt={`${playback.album ?? 'Album'} artwork`} decoding="async" />{:else}<span aria-hidden="true"><MusicIcon name="music" size={96} /></span>{/if}
    </div>

    <div class="now-playing-copy">
      <h1 id="now-playing-title">{playback.title ?? 'Nothing is playing'}</h1>
      <p class="now-playing-artist">{playback.artist ?? 'Spotify'}</p>
      {#if playback.album}<p class="now-playing-album">{playback.album}</p>{/if}
      {#if playback.deviceName}<p class="music-device-label">On {playback.deviceName}</p>{/if}

      <div class="now-playing-progress">
        <input type="range" min="0" max={playback.durationMs ?? 0} step="1000" value={Math.round(spotify.positionMs)} disabled={spotify.busy || !playback.canSeek || !playback.durationMs} aria-label="Playback position" onchange={seek} />
        <div class="now-playing-time"><span>{time(spotify.positionMs)}</span><span>{time(playback.durationMs)}</span></div>
      </div>

      <PlaybackControls {playback} busy={spotify.busy} onaction={action} />

      <div class="now-playing-extra">
        <button class={["music-toggle", { "active": playback.shuffle === true }]} aria-pressed={playback.shuffle === true} disabled={playback.shuffle == null || spotify.busy} onclick={() => action('shuffle', String(!playback.shuffle))}><MusicIcon name="shuffle" /> Shuffle</button>
        <button class={["music-toggle", { "active": Boolean(playback.repeat && playback.repeat !== 'off') }]} disabled={playback.repeat == null || spotify.busy} onclick={repeat}><MusicIcon name="repeat" /> Repeat: {playback.repeat === 'track' ? 'One' : playback.repeat === 'context' ? 'All' : 'Off'}</button>
        <button class={["music-toggle", { "active": showQueue }]} id="queue-toggle" aria-controls="player-queue" aria-expanded={showQueue} disabled={!connection.authenticated || queueLoading} onclick={queue}><MusicIcon name="list-music" /> Queue</button>
        <button class={["music-toggle", { "active": showDevices }]} id="devices-toggle" aria-controls="player-devices" aria-expanded={showDevices} disabled={!connection.authenticated} onclick={devices}><MusicIcon name="speaker" /> Devices</button>
      </div>

      <label class="now-playing-volume">
        <span>Volume</span>
        <input type="range" min="0" max="100" step="1" value={playback.volume ?? 50} disabled={spotify.busy || playback.volume == null} aria-label="Spotify volume" onchange={volume} />
        <span>{playback.volume == null ? 'Unavailable' : `${playback.volume}%`}</span>
      </label>

      {#if spotify.error}<p class="music-inline-error" role="status">{spotify.error}</p>{/if}
      {#if !connection.authenticated}<p class="music-note">Connect a Spotify account for volume, queue, search, playlists, saved music, and device controls. Playback stays in the Spotify app, and basic controls work without an account.</p>{/if}
    </div>
  </div>

  {#if showDevices}<div class="player-devices" id="player-devices"><DevicePicker /></div>{/if}
  {#if showQueue}
    <section id="player-queue" class="now-playing-queue" aria-label="Playback queue">
      <div class="row-heading"><div><h2>Queue</h2><p>Up next on {playback.deviceName ?? 'Spotify'}</p></div><button class="text-link" disabled={queueLoading} onclick={refreshQueue}>{queueLoading ? 'Loading…' : 'Refresh'}</button></div>
      {#if spotify.queueError}<p class="music-inline-error" role="status">{spotify.queueError}</p>{/if}
      {#if spotify.queue?.currentlyPlaying}<p class="queue-current">Playing now: {spotify.queue.currentlyPlaying.title} - {spotify.queue.currentlyPlaying.artist}</p>{/if}
      {#if queueLoading && !spotify.queue}
        <p class="music-empty-line">Loading queue…</p>
      {:else if spotify.queue?.items.length}
        <TrackList tracks={spotify.queue.items} onplay={async uri => { if (await spotify.playUri(uri)) await spotify.loadQueue(); }} />
      {:else if !spotify.queueError}
        <p class="music-empty-line">{connection.authenticated ? 'No tracks queued. Choose an album or playlist to keep listening.' : 'Connect Spotify to view the queue.'}</p>
      {/if}
    </section>
  {/if}
</dialog>

<style>
  .now-playing-art img { object-fit: contain; } .now-playing-art { border-radius: 12px; }
  .now-playing-close { width: 60px; height: 60px; display: grid; place-items: center; }
  .music-toggle { display: inline-flex; align-items: center; gap: 10px; min-height: 56px; font-size: 18px; padding: 14px 20px; }
  .now-playing-volume { max-width: 560px; font-size: 18px; } .now-playing-volume input,.now-playing-progress input { min-height: 40px; }
  .now-playing-time,.music-device-label,.music-note { font-size: 18px; } .music-note { line-height: 1.6; }
  .music-inline-error,.music-empty-line,.queue-current { font-size: 19px; } .queue-current { color: #bdc9de; }
  .now-playing-queue,.player-devices { max-width: 1400px; margin: 38px auto 0; }
  .now-playing-queue :global(.row-heading h2) { font-size: 32px; } .now-playing-queue :global(.row-heading p),.now-playing-queue :global(.text-link) { font-size: 18px; }
  button:focus-visible,input:focus-visible { outline: 3px solid var(--accent); outline-offset: 5px; }
  @media(max-width:800px) { .now-playing-layout { grid-template-columns: 1fr; } .now-playing-art { max-width: 360px; } }
</style>
