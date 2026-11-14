<script lang="ts">
  import MusicIcon from './MusicIcon.svelte';
  import type { SpotifyPlaybackState } from '../../types/spotify';
  let { playback, busy = false, onaction, compact = false }: {
    playback: SpotifyPlaybackState;
    busy?: boolean;
    onaction: (action: string) => void;
    compact?: boolean;
  } = $props();

  function stop(event: MouseEvent) {
    event.stopPropagation();
  }
</script>

<div class={["playback-controls", { compact }]} aria-label="Playback controls">
  <button class="playback-button" disabled={busy || !playback.canSkipPrevious} aria-label="Previous track" onclick={(event) => { stop(event); onaction('previous'); }}><MusicIcon name="skip-back" /></button>
  <button class="playback-button primary" disabled={busy || !playback.canToggle} aria-label={playback.playing ? 'Pause' : 'Play'} onclick={(event) => { stop(event); onaction('toggle'); }}><MusicIcon name={playback.playing ? 'pause' : 'play'} size={28} /></button>
  <button class="playback-button" disabled={busy || !playback.canSkipNext} aria-label="Next track" onclick={(event) => { stop(event); onaction('next'); }}><MusicIcon name="skip-forward" /></button>
</div>

<style>
  .playback-button { width: clamp(64px, 4.8vw, 84px); height: clamp(64px, 4.8vw, 84px); } .playback-button.primary { width: clamp(82px, 6.2vw, 104px); height: clamp(82px, 6.2vw, 104px); }
  .playback-button:focus-visible { outline: 3px solid var(--accent); outline-offset: 5px; }
  .playback-controls.compact .playback-button { width: 48px; height: 48px; } .playback-controls.compact .playback-button.primary { width: 56px; height: 56px; }
</style>
