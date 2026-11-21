<script lang="ts">
  import MusicIcon from './MusicIcon.svelte';
  import { onDestroy } from 'svelte';
  import { spotify } from '../../state/spotify.svelte';
  import type { MusicCollection, SpotifySearchResults } from '../../types/spotify';

  let { onopen, onbrowse }: { onopen: () => void; onbrowse: (collection: MusicCollection) => void } = $props();
  let query = $state('');
  let results = $state<SpotifySearchResults | null>(null);
  let busy = $state(false);
  let error = $state('');
  let generation = 0;
  let timer: ReturnType<typeof setTimeout> | undefined;

  onDestroy(() => { generation++; if (timer) clearTimeout(timer); });

  function schedule() {
    if (timer) clearTimeout(timer);
    generation++;
    results = null;
    error = '';
    if (!query.trim()) { results = null; busy = false; return; }
    busy = true;
    timer = setTimeout(search, 350);
  }

  async function search() {
    const request = generation;
    const value = query.trim();
    if (!value) return;
    try {
      const data = await spotify.search(value);
      if (request === generation) results = data;
    } catch (cause) {
      if (request === generation) error = String(cause);
    } finally {
      if (request === generation) busy = false;
    }
  }

  async function play(uri?: string) {
    if (!uri) return;
    if (await spotify.playUri(uri)) onopen();
    else error = spotify.error;
  }

  const empty = $derived(results && !results.tracks.length && !results.albums.length && !results.playlists.length);
</script>

<section class="music-search" aria-label="Search Spotify">
  <div class="music-search-field">
    <MusicIcon name="search" size={16} />
    <input type="search" bind:value={query} oninput={schedule} placeholder="Artists, songs, or albums" aria-label="Search Spotify" spellcheck="false" autocomplete="off" />
    {#if busy}<span class="music-search-busy">Searching…</span>{/if}
  </div>
  {#if error}<p class="music-inline-error" role="status">{error}</p>{/if}
  {#if results}
    {#if !empty}
      <div class="music-search-results">
        {#if results.tracks.length}
          <div class="music-search-group">
            <h3>Songs</h3>
            {#each results.tracks as track, index (track.id ?? `${track.title}-${index}`)}
              <button class="music-result" disabled={spotify.busy || !track.uri} onclick={() => play(track.uri)}>
                {#if track.artworkUrl}<img src={track.artworkUrl} alt="" loading="lazy" decoding="async" />{:else}<span class="music-result-art" aria-hidden="true"><MusicIcon name="music" /></span>{/if}
                <span><b>{track.title}</b><small>{track.artist}{track.album ? ` · ${track.album}` : ''}</small></span>
                <MusicIcon name="play" size={14} />
              </button>
            {/each}
          </div>
        {/if}
        {#if results.albums.length}
          <div class="music-search-group">
            <h3>Albums</h3>
            {#each results.albums as album (album.id)}
              <button class="music-result" onclick={() => onbrowse({ id: album.id, title: album.name, uri: album.uri, artworkUrl: album.artworkUrl, subtitle: album.artist, kind: 'album' })}>
                {#if album.artworkUrl}<img src={album.artworkUrl} alt="" loading="lazy" decoding="async" />{:else}<span class="music-result-art" aria-hidden="true"><MusicIcon name="disc-3" /></span>{/if}
                <span><b>{album.name}</b><small>{album.artist}{album.year ? ` · ${album.year}` : ''}</small></span>
              </button>
            {/each}
          </div>
        {/if}
        {#if results.playlists.length}
          <div class="music-search-group">
            <h3>Playlists</h3>
            {#each results.playlists as playlist (playlist.id)}
              <button class="music-result" onclick={() => onbrowse({ id: playlist.id, title: playlist.name, uri: playlist.uri, artworkUrl: playlist.artworkUrl, subtitle: playlist.owner, kind: 'playlist' })}>
                {#if playlist.artworkUrl}<img src={playlist.artworkUrl} alt="" loading="lazy" decoding="async" />{:else}<span class="music-result-art" aria-hidden="true"><MusicIcon name="list-music" /></span>{/if}
                <span><b>{playlist.name}</b><small>{playlist.owner ? `By ${playlist.owner}` : 'Playlist'}</small></span>
              </button>
            {/each}
          </div>
        {/if}
      </div>
    {:else if !busy}
      <p class="music-empty-line">No Spotify results for “{query}”.</p>
    {/if}
  {/if}
</section>

<style>
  .music-search { max-width: none; margin: 0; }
  .music-search-field { display: flex; align-items: center; gap: 10px; max-width: 520px; padding: 4px 14px; border-radius: 10px; background: #ffffff0d; border: 1px solid #ffffff14; color: var(--dim); }
  .music-search-field :global(svg) { flex: none; }
  .music-search-field input { flex: 1; min-width: 0; border: 0; background: transparent; padding: 10px 0; font-size: clamp(13px,.9vw,16px); color: var(--text); }
  .music-search-field input:focus { outline: 0; }
  .music-search-field:focus-within { border-color: transparent; outline: 2px solid var(--accent); outline-offset: 2px; }
  .music-search-busy { flex: none; font-size: clamp(10px,.7vw,12px); color: var(--muted); white-space: nowrap; }
  .music-search-results { display: grid; grid-template-columns: repeat(3,minmax(0,1fr)); gap: clamp(14px,1.6vw,28px); margin-top: 20px; }
  .music-search-results h3 { font-size: clamp(10px,.72vw,12px); font-weight: 700; letter-spacing: .12em; text-transform: uppercase; color: var(--dim); margin: 0 0 8px; padding: 0 8px; }
  .music-search-group { min-width: 0; }
  .music-result { display: grid; grid-template-columns: 44px minmax(0,1fr) auto; align-items: center; gap: 12px; width: 100%; text-align: left; background: transparent; border-radius: 8px; padding: 7px 8px; min-width: 0; }
  .music-result:hover,.music-result:focus-visible { background: #ffffff0f; }
  .music-result:focus-visible { outline: 2px solid var(--accent); outline-offset: -2px; }
  .music-result :global(svg) { color: var(--dim); opacity: 0; }
  .music-result:hover :global(svg),.music-result:focus-visible :global(svg) { opacity: 1; }
  .music-result img,.music-result-art { width: 44px; height: 44px; border-radius: 5px; object-fit: cover; flex: none; background: var(--surface-3); display: grid; place-items: center; color: #ffffff42; }
  .music-result span:nth-child(2) { min-width: 0; display: grid; gap: 2px; }
  .music-result b { font-size: clamp(12px,.85vw,15px); font-weight: 550; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .music-result small { font-size: clamp(11px,.74vw,13px); color: var(--muted); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .music-search-busy,.music-inline-error,.music-empty-line { font-size: clamp(12px,.85vw,15px); }
  .music-empty-line { margin-top: 20px; }
  @media(max-width:1180px) { .music-search-results { grid-template-columns: repeat(2,minmax(0,1fr)); } }
  @media(max-width:760px) { .music-search-results { grid-template-columns: 1fr; } }
</style>
