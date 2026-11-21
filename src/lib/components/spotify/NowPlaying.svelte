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
  let panel = $state<'queue' | 'devices' | null>(null);
  export function closeOverlay() {
    if (panel) { const previous = panel; panel = null; document.getElementById(`${previous}-toggle`)?.focus(); return true; }
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

  function remaining(value?: number) {
    if (value == null) return '-0:00';
    return `-${time(value)}`;
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

  async function toggle(target: 'queue' | 'devices') {
    const open = panel !== target;
    panel = open ? target : null;
    if (!open) return;
    if (target === 'queue') await refreshQueue();
    await tick();
    document.getElementById(`player-${target}`)?.scrollIntoView({ block: 'nearest' });
    document.querySelector<HTMLElement>(`#player-${target} button`)?.focus();
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
    <button class="now-playing-back" onclick={onclose}><MusicIcon name="chevron-right" size={18} /><span>Minimize</span></button>
    <span class="now-playing-kicker">Playing from {playback.album ?? playback.deviceName ?? 'Spotify'} · Spotify</span>
    <button class="now-playing-close" aria-label="Close player" onclick={onclose}><MusicIcon name="x" size={20} /></button>
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

      <div class="now-playing-scrubber">
        <input type="range" min="0" max={playback.durationMs ?? 0} step="1000" value={Math.round(spotify.positionMs)} disabled={spotify.busy || !playback.canSeek || !playback.durationMs} aria-label="Playback position" onchange={seek} />
        <div class="now-playing-time"><span>{time(spotify.positionMs)}</span><span>{remaining((playback.durationMs ?? 0) - spotify.positionMs)}</span></div>
      </div>

      <div class="now-playing-transport">
        <PlaybackControls {playback} busy={spotify.busy} onaction={action} />
      </div>

      <div class="now-playing-extra">
        <button class={["music-toggle", { "active": playback.shuffle === true }]} aria-pressed={playback.shuffle === true} disabled={playback.shuffle == null || spotify.busy} onclick={() => action('shuffle', String(!playback.shuffle))}><MusicIcon name="shuffle" size={16} /> Shuffle</button>
        <button class={["music-toggle", { "active": Boolean(playback.repeat && playback.repeat !== 'off') }]} disabled={playback.repeat == null || spotify.busy} onclick={repeat}><MusicIcon name="repeat" size={16} /> {playback.repeat === 'track' ? 'Repeat one' : playback.repeat === 'context' ? 'Repeat all' : 'Repeat'}</button>
        <button class={["music-toggle", { "active": panel === 'queue' }]} id="queue-toggle" aria-controls="player-queue" aria-expanded={panel === 'queue'} disabled={!connection.authenticated || queueLoading} onclick={() => toggle('queue')}><MusicIcon name="list-music" size={16} /> Playing next</button>
        <button class={["music-toggle", { "active": panel === 'devices' }]} id="devices-toggle" aria-controls="player-devices" aria-expanded={panel === 'devices'} disabled={!connection.authenticated} onclick={() => toggle('devices')}><MusicIcon name="speaker" size={16} /> Devices</button>
      </div>

      <label class="now-playing-volume">
        <MusicIcon name="speaker" size={16} />
        <input type="range" min="0" max="100" step="1" value={playback.volume ?? 50} disabled={spotify.busy || playback.volume == null} aria-label="Spotify volume" onchange={volume} />
        <span>{playback.volume == null ? 'Unavailable' : `${playback.volume}%`}</span>
      </label>

      {#if spotify.error}<p class="music-inline-error" role="status">{spotify.error}</p>{/if}
      {#if !connection.authenticated}<p class="music-note">Connect a Spotify account for volume, queue, search, playlists, saved music, and device controls. Playback stays in the Spotify app, and basic controls work without an account.</p>{/if}
    </div>
  </div>

  {#if panel === 'devices'}<div class="player-devices" id="player-devices"><DevicePicker /></div>{/if}
  {#if panel === 'queue'}
    <section id="player-queue" class="now-playing-queue" aria-label="Playback queue">
      <div class="row-heading"><div><h2>Playing next</h2><p>Up next on {playback.deviceName ?? 'Spotify'}</p></div><button class="text-link" disabled={queueLoading} onclick={refreshQueue}>{queueLoading ? 'Loading…' : 'Refresh'}</button></div>
      {#if spotify.queueError}<p class="music-inline-error" role="status">{spotify.queueError}</p>{/if}
      {#if spotify.queue?.currentlyPlaying}<p class="queue-current"><MusicIcon name="music" size={14} /> {spotify.queue.currentlyPlaying.title} — {spotify.queue.currentlyPlaying.artist}</p>{/if}
      {#if queueLoading && !spotify.queue}
        <p class="music-empty-line">Loading queue…</p>
      {:else if spotify.queue?.items.length}
        <TrackList tracks={spotify.queue.items} onplay={async uri => { if (await spotify.playUri(uri)) await refreshQueue(); }} />
      {:else if !spotify.queueError}
        <p class="music-empty-line">{connection.authenticated ? 'No tracks queued. Choose an album or playlist to keep listening.' : 'Connect Spotify to view the queue.'}</p>
      {/if}
    </section>
  {/if}
</dialog>

<style>
  .now-playing-header { display: grid; grid-template-columns: auto minmax(0,1fr) auto; align-items: center; gap: 16px; }
  .now-playing-back { display: inline-flex; align-items: center; gap: 6px; min-height: 40px; padding: 0 12px 0 8px; border-radius: 99px; background: #ffffff12; color: var(--text-dim); font-size: clamp(12px,.82vw,14px); font-weight: 600; }
  .now-playing-back :global(svg) { transform: rotate(180deg); }
  .now-playing-back:hover { background: #ffffff1c; color: var(--text); }
  .now-playing-kicker { justify-self: center; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; color: var(--dim); font-size: clamp(10px,.72vw,12px); font-weight: 600; letter-spacing: .1em; text-transform: uppercase; }
  .now-playing-close { width: 40px; height: 40px; display: grid; place-items: center; border-radius: 50%; background: #ffffff12; border: 1px solid #ffffff1f; }
  .now-playing-close:hover { background: #ffffff1f; }
  .now-playing-art { border-radius: 10px; }
  .now-playing-art img { object-fit: cover; }
  .now-playing-scrubber { margin: clamp(16px,2vw,26px) 0 clamp(12px,1.4vw,18px); max-width: 720px; }
  .now-playing-scrubber input { width: 100%; height: 6px; }
  .now-playing-transport { display: flex; justify-content: flex-start; }
  .now-playing-extra { display: flex; align-items: center; gap: 8px; flex-wrap: wrap; margin: clamp(16px,1.8vw,24px) 0 14px; }
  .music-toggle { display: inline-flex; align-items: center; gap: 7px; padding: 8px 14px; border-radius: 99px; background: #ffffff0d; border: 1px solid #ffffff16; font-size: clamp(12px,.8vw,14px); font-weight: 550; color: var(--text-dim); }
  .music-toggle:hover:not(:disabled) { background: #ffffff15; color: var(--text); }
  .music-toggle.active { background: var(--accent-soft); border-color: color-mix(in srgb, var(--accent) 55%, transparent); color: var(--text); }
  .now-playing-volume { display: flex; align-items: center; gap: 12px; max-width: 420px; color: var(--muted); font-size: clamp(12px,.8vw,14px); }
  .now-playing-volume input { flex: 1; }
  .now-playing-time { display: flex; justify-content: space-between; margin-top: 6px; color: var(--dim); font-size: clamp(10px,.75vw,12px); font-variant-numeric: tabular-nums; }
  .music-note { font-size: clamp(12px,.85vw,15px); line-height: 1.6; }
  .music-inline-error,.music-empty-line,.queue-current { font-size: clamp(12px,.9vw,16px); }
  .queue-current { display: flex; align-items: center; gap: 7px; color: var(--dim); margin: 0 0 12px; }
  .queue-current :global(svg) { color: var(--accent); flex: none; }
  .now-playing-queue,.player-devices { max-width: 1400px; margin: 34px auto 0; }
  .now-playing-queue { padding: 0; background: transparent; border: 0; }
  .now-playing-queue :global(.row-heading h2) { font-size: clamp(20px,1.7vw,28px); }
  .now-playing-queue :global(.row-heading p),.now-playing-queue :global(.text-link) { font-size: clamp(12px,.82vw,15px); }
  .player-devices { margin-top: 26px; }
  button:focus-visible,input:focus-visible { outline: 3px solid var(--accent); outline-offset: 4px; }
  @media(max-width:900px) { .now-playing-layout { grid-template-columns: 1fr; align-items: start; gap: 26px; } .now-playing-art { max-width: min(340px,74vw); } .now-playing-kicker { display: none; } .now-playing-header { grid-template-columns: auto minmax(0,1fr); } .now-playing-close { justify-self: end; } }
</style>
