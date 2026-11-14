<script lang="ts">
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

<div class:compact class="playback-controls" aria-label="Playback controls">
  <button class="playback-button" disabled={busy || !playback.canSkipPrevious} aria-label="Previous track" onclick={(event) => { stop(event); onaction('previous'); }}>⏮</button>
  <button class="playback-button primary" disabled={busy || !playback.canToggle} aria-label={playback.playing ? 'Pause' : 'Play'} onclick={(event) => { stop(event); onaction('toggle'); }}>{playback.playing ? 'Ⅱ' : '▶'}</button>
  <button class="playback-button" disabled={busy || !playback.canSkipNext} aria-label="Next track" onclick={(event) => { stop(event); onaction('next'); }}>⏭</button>
</div>
