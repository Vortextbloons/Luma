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
</script>

<section class="music-search" aria-label="Search Spotify">
  <h2>Find your next listen</h2>
  <p class="search-intro">Search songs, albums, and playlists on Spotify.</p>
  <div class="music-search-field">
    <span aria-hidden="true"><MusicIcon name="search" /></span>
    <input type="search" bind:value={query} oninput={schedule} placeholder="Search Spotify" aria-label="Search Spotify" spellcheck="false" autocomplete="off" />
    {#if busy}<span class="music-search-busy">Searching…</span>{/if}
  </div>
  {#if error}<p class="music-inline-error" role="status">{error}</p>{/if}
  {#if results}
    {#if results.tracks.length || results.albums.length || results.playlists.length}
      <div class="music-search-results">
        {#if results.tracks.length}
          <div>
            <h3>Tracks</h3>
            {#each results.tracks as track, index (track.id ?? `${track.title}-${index}`)}
              <button class="music-result" disabled={spotify.busy || !track.uri} onclick={() => play(track.uri)}>
                {#if track.artworkUrl}<img src={track.artworkUrl} alt="" loading="lazy" decoding="async" />{:else}<span class="music-result-art" aria-hidden="true"><MusicIcon name="music" /></span>{/if}
                <span><b>{track.title}</b><small>{track.artist}{track.album ? ` · ${track.album}` : ''}</small></span>
              </button>
            {/each}
          </div>
        {/if}
        {#if results.playlists.length}
          <div>
            <h3>Playlists</h3>
            {#each results.playlists as playlist (playlist.id)}
              <button class="music-result" onclick={() => onbrowse({ id: playlist.id, title: playlist.name, uri: playlist.uri, artworkUrl: playlist.artworkUrl, subtitle: playlist.owner, kind: 'playlist' })}>
                {#if playlist.artworkUrl}<img src={playlist.artworkUrl} alt="" loading="lazy" decoding="async" />{:else}<span class="music-result-art" aria-hidden="true"><MusicIcon name="list-music" /></span>{/if}
                <span><b>{playlist.name}</b><small>{playlist.owner ? `By ${playlist.owner}` : 'Playlist'}</small></span>
              </button>
            {/each}
          </div>
        {/if}
        {#if results.albums.length}
          <div>
            <h3>Albums</h3>
            {#each results.albums as album (album.id)}
              <button class="music-result" onclick={() => onbrowse({ id: album.id, title: album.name, uri: album.uri, artworkUrl: album.artworkUrl, subtitle: album.artist, kind: 'album' })}>
                {#if album.artworkUrl}<img src={album.artworkUrl} alt="" loading="lazy" decoding="async" />{:else}<span class="music-result-art" aria-hidden="true"><MusicIcon name="disc-3" /></span>{/if}
                <span><b>{album.name}</b><small>{album.artist}{album.year ? ` · ${album.year}` : ''}</small></span>
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
  .music-search { max-width: none; } h2 { font-size: clamp(30px,2.6vw,48px); margin: 0 0 12px; letter-spacing: -.04em; } .search-intro { font-size: 20px; color: var(--muted); margin: 0 0 28px; }
  .music-search-field { padding: 10px 22px; } .music-search-field input { font-size: 24px; min-height: 64px; }
  .music-search-results { max-height: none; padding: 24px; gap: 24px; } .music-search-results h3 { font-size: 20px; letter-spacing: 0; text-transform: none; margin-bottom: 18px; color: var(--muted); }
  .music-result { gap: 16px; min-height: 88px; padding: 12px; } .music-result img,.music-result-art { width: 60px; height: 60px; object-fit: contain; }
  .music-result b { font-size: 20px; } .music-result small { font-size: 17px; margin-top: 6px; } .music-result:focus-visible { outline: 3px solid var(--accent); outline-offset: 2px; }
  .music-search-busy,.music-inline-error,.music-empty-line { font-size: 18px; }
  @media(max-width:1100px) { .music-search-results { grid-template-columns: 1fr; } }
</style>
