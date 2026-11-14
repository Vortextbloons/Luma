<script lang="ts">
  import type { ActivityItem, MediaState } from '../types';
  import spotify from '../../assets/spotify.svg';
  let { state, busy, control, launch, refresh, lastPlayed }: { state: MediaState; busy: boolean; control: (action: 'toggle' | 'previous' | 'next') => void; launch: () => void; refresh: () => void; lastPlayed: ActivityItem | null } = $props();
  const title = $derived(state.available ? state.title || 'Spotify' : lastPlayed?.title ?? 'Spotify is not playing');
  const subtitle = $derived(state.available ? state.error || state.artist || 'Open Spotify and start a track to connect.' : lastPlayed ? [lastPlayed.subtitle, 'Last played'].filter(Boolean).join(' · ') : 'Open Spotify and start a track to connect.');
</script>

<section id="music" class="content-row music-row">
  <div class="row-heading"><div><h2>Music on your PC</h2><p>Spotify · Windows media session</p></div><button class="text-link" onclick={refresh}>Refresh session</button></div>
  <div class="music-player">
    <img class="app-icon" src={spotify} alt="" />
    <div class="min-w-0 flex-1"><h3 class="truncate font-semibold">{title}</h3><p class="text-sm text-neutral-400">{subtitle}</p>{#if state.available}<p class="text-xs text-neutral-500 mt-1">{state.playing ? 'Playing' : 'Paused'}</p>{/if}</div>
    <div class="flex gap-3"><button class="control" disabled={busy || !state.canPrevious} aria-label="Previous track" onclick={() => control('previous')}>⏮</button><button class="control" disabled={busy || !state.canToggle} aria-label={state.playing ? 'Pause' : 'Play'} onclick={() => control('toggle')}>{state.playing ? 'Ⅱ' : '▶'}</button><button class="control" disabled={busy || !state.canNext} aria-label="Next track" onclick={() => control('next')}>⏭</button></div>
    <button class="btn btn-glass" onclick={launch}>Open Spotify</button>
  </div>
</section>
