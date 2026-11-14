<script lang="ts">
  import MusicIcon from './MusicIcon.svelte';
  import { spotify } from '../../state/spotify.svelte';
  import PlaybackControls from './PlaybackControls.svelte';

  let { onopen }: { onopen: () => void } = $props();
  const playback = $derived(spotify.playback);
  const visible = $derived(playback.playerAvailable || Boolean(playback.title));
</script>

{#if visible}
  <aside class="mini-player" aria-label="Mini player">
    <button class="mini-player-open" onclick={onopen} aria-label="Open now playing">
      <span class="mini-player-art">{#if playback.artworkUrl}<img src={playback.artworkUrl} alt="" />{:else}<span aria-hidden="true"><MusicIcon name="music" /></span>{/if}</span>
      <span class="mini-player-copy">
        <b>{playback.title ?? 'Spotify'}</b>
        <small>{playback.artist ?? (playback.playerAvailable ? 'Ready to play' : 'Spotify')}</small>
      </span>
    </button>
    <PlaybackControls {playback} busy={spotify.busy} compact onaction={action => spotify.action(action)} />
    <span class={["mini-player-status", { "playing": playback.playing }]} aria-hidden="true"></span>
    <div class="mini-player-progress" aria-hidden="true"><span style:transform={`scaleX(${spotify.progress})`}></span></div>
  </aside>
{/if}

<style>
  .mini-player { width: min(880px,calc(100vw - 2 * var(--side))); padding: 14px 20px; gap: 20px; }
  .mini-player-open { min-height: 60px; gap: 16px; border-radius: 8px; }
  .mini-player-open:focus-visible { outline: 3px solid var(--accent); outline-offset: 4px; }
  .mini-player-art { width: 58px; height: 58px; } .mini-player-art img { object-fit: contain; }
  .mini-player-copy b { font-size: 20px; } .mini-player-copy small { font-size: 16px; }
  @media (max-width: 710px) {
    .mini-player { width: calc(100vw - 20px); padding: 10px 14px; gap: 12px; }
    .mini-player-open { min-height: 48px; gap: 10px; }
    .mini-player-art { width: 44px; height: 44px; }
    .mini-player-copy b { font-size: 16px; }
    .mini-player-copy small { font-size: 13px; }
  }
</style>
