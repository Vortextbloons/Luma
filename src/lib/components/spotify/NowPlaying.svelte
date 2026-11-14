<script lang="ts">
  import { onMount } from 'svelte';
  import { spotify } from '../../state/spotify.svelte';
  import PlaybackControls from './PlaybackControls.svelte';

  let { onclose }: { onclose: () => void } = $props();
  const playback = $derived(spotify.playback);
  const connection = $derived(spotify.connection);
  let showQueue = $state(false);
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
    if (showQueue) {
      queueLoading = true;
      try { await spotify.loadQueue(); }
      finally { queueLoading = false; }
    }
  }

  async function refreshQueue() {
    queueLoading = true;
    try { await spotify.loadQueue(); }
    finally { queueLoading = false; }
  }

  async function playQueue(uri?: string) {
    if (await spotify.playUri(uri)) showQueue = false;
  }
</script>

<dialog open class="now-playing" aria-modal="true" aria-labelledby="now-playing-title">
  {#if playback.artworkUrl}<div class="now-playing-ambient" style:background-image={`url("${playback.artworkUrl}")`} aria-hidden="true"></div>{:else}<div class="now-playing-ambient" aria-hidden="true"></div>{/if}
  <header class="now-playing-header">
    <span class="panel-kicker">NOW PLAYING · SPOTIFY</span>
    <button class="now-playing-close" aria-label="Close player" onclick={onclose}>×</button>
  </header>

  <div class="now-playing-layout">
    <div class="now-playing-art">
      {#if playback.artworkUrl}<img src={playback.artworkUrl} alt={`${playback.album ?? 'Album'} artwork`} />{:else}<span aria-hidden="true">♪</span>{/if}
    </div>

    <div class="now-playing-copy">
      <h1 id="now-playing-title">{playback.title ?? 'Nothing is playing'}</h1>
      <p class="now-playing-artist">{playback.artist ?? 'Spotify'}</p>
      {#if playback.album}<p class="now-playing-album">{playback.album}</p>{/if}
      {#if playback.deviceName}<p class="music-device-label">On {playback.deviceName}</p>{/if}

      <div class="now-playing-progress">
        <input type="range" min="0" max={playback.durationMs ?? 0} step="1000" value={Math.round(spotify.positionMs)} disabled={!playback.canSeek || !playback.durationMs} aria-label="Playback position" onchange={seek} />
        <div class="now-playing-time"><span>{time(spotify.positionMs)}</span><span>{time(playback.durationMs)}</span></div>
      </div>

      <PlaybackControls {playback} busy={spotify.busy} onaction={action} />

      <div class="now-playing-extra">
        <button class="music-toggle" class:active={playback.shuffle === true} disabled={playback.shuffle == null || spotify.busy} onclick={() => action('shuffle', String(!playback.shuffle))}>Shuffle</button>
        <button class="music-toggle" class:active={Boolean(playback.repeat && playback.repeat !== 'off')} disabled={playback.repeat == null || spotify.busy} onclick={repeat}>Repeat: {playback.repeat === 'track' ? 'One' : playback.repeat === 'context' ? 'All' : 'Off'}</button>
        <button class="music-toggle" class:active={showQueue} onclick={queue}>Queue</button>
      </div>

      <label class="now-playing-volume">
        <span>Volume</span>
        <input type="range" min="0" max="100" step="1" value={playback.volume ?? 50} disabled={playback.volume == null} aria-label="Spotify volume" onchange={volume} />
        <span>{playback.volume == null ? 'Account only' : `${playback.volume}%`}</span>
      </label>

      {#if spotify.error}<p class="music-inline-error" role="status">{spotify.error}</p>{/if}
      {#if !connection.authenticated}<p class="music-note">Connect a Spotify account for volume, queue, search, playlists, saved music, and device controls. Playback stays in the Spotify app, and basic controls work without an account.</p>{/if}
    </div>
  </div>

  {#if showQueue}
    <section class="now-playing-queue" aria-label="Playback queue">
      <div class="row-heading"><div><h2>Queue</h2><p>Up next on {playback.deviceName ?? 'Spotify'}</p></div><button class="text-link" disabled={queueLoading} onclick={refreshQueue}>{queueLoading ? 'Loading…' : 'Refresh'}</button></div>
      {#if queueLoading && !spotify.queue}
        <p class="music-empty-line">Loading queue…</p>
      {:else if spotify.queue?.items.length}
        <div class="music-track-list compact">
          {#each spotify.queue.items as track, index (track.id ?? `${track.title}-${index}`)}
            <button class="music-track" onclick={() => playQueue(track.uri)}>
              <span class="music-track-art" aria-hidden="true">♪</span>
              <span class="music-track-copy"><b>{track.title}</b><small>{track.artist}</small></span>
              <span class="music-track-album">{track.album}</span>
            </button>
          {/each}
        </div>
      {:else}
        <p class="music-empty-line">{connection.authenticated ? 'The queue is empty or unavailable for this playback.' : 'Connect Spotify to view the queue.'}</p>
      {/if}
    </section>
  {/if}
</dialog>
