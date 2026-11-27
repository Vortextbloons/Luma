<script lang="ts">
  import type { WidgetProps } from '../../services/widgetModel';
  import MusicIcon from './MusicIcon.svelte';
  import { spotify } from '../../state/spotify.svelte';
  import PlaybackControls from './PlaybackControls.svelte';

  let { config, context, compact = false, moving = false, onopen }: WidgetProps = $props();
  const playback = $derived(spotify.playback);
  const duration = $derived(playback.durationMs ?? 0);
  const placement = $derived(config.position ? 'free' : config.placement);
  const bar = $derived(placement === 'bar' && !compact && !moving);
  const variant = $derived(config.variant);
  function clock(ms: number) {
    const total = Math.max(0, Math.floor(ms / 1000));
    return `${Math.floor(total / 60)}:${String(total % 60).padStart(2, '0')}`;
  }
</script>

<aside
  class={['mini-player', { 'is-docked': !bar, 'is-compact': compact, 'is-ticker': variant === 'ticker', 'is-minimal': variant === 'minimal', 'is-cinematic': variant === 'cinematic' }]}
  data-placement={placement}
  data-context={context}
  data-variant={variant}
  aria-label="Now playing"
>
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

<style>
  .mini-player.is-minimal { box-shadow: none; }
  .mini-player.is-minimal .mini-player-timeline,
  .mini-player.is-ticker .mini-player-timeline { display: none; }
  .mini-player.is-ticker :global(.playback-controls) { display: none; }
  .mini-player.is-cinematic { background: linear-gradient(135deg, color-mix(in srgb, var(--surface-2) 92%, var(--accent) 8%), var(--surface-2)); }
  .mini-player.is-cinematic .mini-player-art { width: clamp(56px, 4.4vw, 76px); height: clamp(56px, 4.4vw, 76px); }
  .mini-player.is-compact { padding: 8px 12px; }
</style>
