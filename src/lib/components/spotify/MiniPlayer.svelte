<script lang="ts">
  import type { WidgetPlacement } from '../../types';
  import MusicIcon from './MusicIcon.svelte';
  import { spotify } from '../../state/spotify.svelte';
  import PlaybackControls from './PlaybackControls.svelte';

  let { onopen, placement }: { onopen: () => void; placement: WidgetPlacement } = $props();
  const playback = $derived(spotify.playback);
  const duration = $derived(playback.durationMs ?? 0);
  function clock(ms: number) {
    const total = Math.max(0, Math.floor(ms / 1000));
    return `${Math.floor(total / 60)}:${String(total % 60).padStart(2, '0')}`;
  }
</script>

  <aside class={['mini-player', { 'is-docked': placement !== 'bar' }]} data-placement={placement} aria-label="Now playing">
    <div class="mini-player-lcd">
      <button class="mini-player-open" onclick={onopen} aria-label="Open now playing">
        <span class="mini-player-art">{#if playback.artworkUrl}<img src={playback.artworkUrl} alt="" decoding="async" />{:else}<span aria-hidden="true"><MusicIcon name="music" /></span>{/if}</span>
        <span class="mini-player-copy">
          <b>{playback.title ?? 'Spotify'}</b>
          <small>{playback.artist ?? (playback.playerAvailable ? 'Ready to play' : 'Open Spotify to connect')}</small>
        </span>
      </button>
      <div class="mini-player-timeline" aria-hidden="true">
        <span class="mini-player-time">{clock(spotify.positionMs)}</span>
        <span class="mini-player-progress" style:--progress={spotify.progress}><span style:transform={`scaleX(${spotify.progress})`}></span><i></i></span>
        <span class="mini-player-time">-{clock(Math.max(0, duration - spotify.positionMs))}</span>
      </div>
    </div>
    <PlaybackControls {playback} busy={spotify.busy} compact onaction={action => spotify.action(action)} />
    <span class={['mini-player-status', { playing: playback.playing }]} aria-hidden="true"></span>
  </aside>
  {#if placement === 'bar'}<div class="mini-player-spacer" aria-hidden="true"></div>{/if}
