<script lang="ts">
  import { spotify } from '../../state/spotify.svelte';
  import PlaybackControls from './PlaybackControls.svelte';

  let { onopen }: { onopen: () => void } = $props();
  const playback = $derived(spotify.playback);
  const visible = $derived(playback.playerAvailable || Boolean(playback.title));
</script>

{#if visible}
  <aside class="mini-player" aria-label="Mini player">
    <button class="mini-player-open" onclick={onopen} aria-label="Open now playing">
      <span class="mini-player-art">{#if playback.artworkUrl}<img src={playback.artworkUrl} alt="" />{:else}<span aria-hidden="true">♪</span>{/if}</span>
      <span class="mini-player-copy">
        <b>{playback.title ?? 'Spotify'}</b>
        <small>{playback.artist ?? (playback.playerAvailable ? 'Ready to play' : 'Spotify')}</small>
      </span>
    </button>
    <PlaybackControls {playback} busy={spotify.busy} compact onaction={action => spotify.action(action)} />
    <span class="mini-player-status" class:playing={playback.playing} aria-hidden="true"></span>
    <div class="mini-player-progress" aria-hidden="true"><span style:width={`${Math.round(spotify.progress * 100)}%`}></span></div>
  </aside>
{/if}
