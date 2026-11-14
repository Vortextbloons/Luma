<script lang="ts">
  import MusicIcon from './MusicIcon.svelte';
  import { spotify } from '../../state/spotify.svelte';
  import type { SpotifyTrack } from '../../types/spotify';
  let { tracks, onplay, artworkUrl, album }: { tracks: SpotifyTrack[]; onplay: (uri?: string) => void; artworkUrl?: string; album?: string } = $props();
  function duration(ms?: number) { return ms == null ? '' : `${Math.floor(ms / 60000)}:${String(Math.floor(ms / 1000) % 60).padStart(2, '0')}`; }
</script>
<div class="tracks">
  {#each tracks as track, index (`${track.uri}-${index}`)}
    <button class={{ "current": Boolean(track.id && track.id === spotify.playback.trackId) }} disabled={spotify.busy || !track.uri?.startsWith('spotify:track:')} onclick={() => onplay(track.uri)} aria-label={`Play ${track.title} by ${track.artist}`}>
      <span class="number">{index + 1}</span>
      <span class="art">{#if track.artworkUrl || artworkUrl}<img src={track.artworkUrl ?? artworkUrl} alt="" loading="lazy" decoding="async" />{:else}<MusicIcon />{/if}</span>
      <span class="copy"><b>{track.title}</b><small>{track.artist || 'Unknown artist'}{!track.uri?.startsWith('spotify:track:') ? ' - Unavailable' : ''}</small></span>
      <span class="album">{track.album || album || ''}</span>
      <span class="duration">{duration(track.durationMs)}</span>
      <MusicIcon name="play" size={18} />
    </button>
  {/each}
</div>
<style>
  .tracks { display: grid; gap: 5px; }
  button { display: flex; align-items: center; gap: 18px; min-height: 84px; padding: 12px 18px; text-align: left; border-radius: 12px; background: transparent; color: #f3f4fa; }
  button:hover, button:focus-visible { background: #ffffff10; }
  button:focus-visible { outline: 3px solid var(--accent); outline-offset: 2px; }
  button:disabled { opacity: .45; }
  .current .copy b { color: var(--accent); }
  .number, .duration { color: #aab1c2; font-size: 18px; font-variant-numeric: tabular-nums; }
  .number { width: 26px; text-align: center; }
  .art { width: 56px; height: 56px; flex: none; display: grid; place-items: center; background: #252c3d; border-radius: 6px; overflow: hidden; }
  img { width: 100%; height: 100%; object-fit: contain; }
  .copy { display: grid; gap: 5px; flex: 1; min-width: 0; }
  b { font-size: clamp(18px,1.4vw,25px); font-weight: 600; }
  small, .album { color: #aab1c2; font-size: clamp(16px,1.1vw,20px); }
  b, small, .album { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .album { max-width: 25%; }
  @media(max-width:700px) { .album,.number { display: none; } button { gap: 12px; padding: 10px; } }
</style>
