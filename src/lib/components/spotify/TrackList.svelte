<script lang="ts">
  import MusicIcon from './MusicIcon.svelte';
  import { spotify } from '../../state/spotify.svelte';
  import type { SpotifyTrack } from '../../types/spotify';
  let { tracks, onplay, artworkUrl, album }: { tracks: SpotifyTrack[]; onplay: (uri?: string) => void; artworkUrl?: string; album?: string } = $props();
  function duration(ms?: number) { return ms == null ? '' : `${Math.floor(ms / 60000)}:${String(Math.floor(ms / 1000) % 60).padStart(2, '0')}`; }
</script>
<div class="tracks">
  {#each tracks as track, index (`${track.uri}-${index}`)}
    {@const current = Boolean(track.id && track.id === spotify.playback.trackId)}
    <button class={{ "current": current, "playing": current && spotify.playback.playing }} disabled={spotify.busy || !track.uri?.startsWith('spotify:track:')} onclick={() => onplay(track.uri)} aria-label={`Play ${track.title} by ${track.artist}`}>
      <span class="number">
        {#if current}<span class="eq" aria-hidden="true"><i></i><i></i><i></i></span>{:else}<span class="index">{index + 1}</span><MusicIcon name="play" size={14} />{/if}
      </span>
      <span class="art">{#if track.artworkUrl || artworkUrl}<img src={track.artworkUrl ?? artworkUrl} alt="" loading="lazy" decoding="async" />{:else}<MusicIcon />{/if}</span>
      <span class="copy"><b>{track.title}</b><small>{track.artist || 'Unknown artist'}{!track.uri?.startsWith('spotify:track:') ? ' · Unavailable' : ''}</small></span>
      <span class="album">{track.album || album || ''}</span>
      <span class="duration">{duration(track.durationMs)}</span>
    </button>
  {/each}
</div>
<style>
  .tracks { display: grid; gap: 1px; }
  button { display: grid; grid-template-columns: 26px 44px minmax(0,1fr) minmax(0,.85fr) 54px; align-items: center; gap: 14px; min-height: 56px; padding: 6px 12px; text-align: left; border-radius: 8px; border-bottom: 1px solid #ffffff09; background: transparent; color: var(--text); }
  button:last-child { border-bottom: 0; }
  button:hover, button:focus-visible { background: #ffffff0d; }
  button:focus-visible { outline: 2px solid var(--accent); outline-offset: -2px; }
  button:disabled { opacity: .45; }
  .number { position: relative; display: grid; place-items: center; justify-items: center; width: 26px; height: 20px; }
  .index { color: var(--dim); font-size: 13px; font-variant-numeric: tabular-nums; }
  .number :global(svg) { display: none; color: var(--text); }
  button:hover .index, button:focus-visible .index { display: none; }
  button:hover .number :global(svg), button:focus-visible .number :global(svg) { display: block; }
  .eq { display: inline-flex; align-items: flex-end; gap: 2px; height: 14px; }
  .eq i { width: 2.5px; height: 40%; border-radius: 2px; background: var(--accent); animation: eq-bounce .9s ease-in-out infinite; }
  .eq i:nth-child(2) { height: 90%; animation-delay: .2s; } .eq i:nth-child(3) { height: 60%; animation-delay: .4s; }
  .playing .eq i { animation-play-state: running; }
  .current:not(.playing) .eq i { animation: none; height: 3px; opacity: .65; }
  @keyframes eq-bounce { 0%,100% { height: 22%; } 50% { height: 100%; } }
  .art { width: 44px; height: 44px; flex: none; display: grid; place-items: center; background: var(--surface-3); border-radius: 5px; overflow: hidden; color: #ffffff42; }
  img { width: 100%; height: 100%; object-fit: cover; }
  .copy { display: grid; gap: 2px; min-width: 0; }
  b { font-size: clamp(13px,.92vw,16px); font-weight: 550; line-height: 1.3; }
  .current b { color: var(--accent); }
  small, .album, .duration { font-size: clamp(11px,.76vw,13px); color: var(--muted); }
  b, small, .album { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .duration { text-align: right; font-variant-numeric: tabular-nums; color: var(--dim); }
  @media(max-width:900px) { .album { display: none; } button { grid-template-columns: 24px 40px minmax(0,1fr) 48px; } .art { width: 40px; height: 40px; } }
  @media(max-width:560px) { .number { display: none; } button { grid-template-columns: 40px minmax(0,1fr) 44px; } }
</style>
