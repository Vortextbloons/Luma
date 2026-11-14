<script lang="ts">
  import { onDestroy } from 'svelte';
  import { spotify } from '../../state/spotify.svelte';
  import type { SpotifySearchResults } from '../../types/spotify';

  let { onopen }: { onopen: () => void } = $props();
  let query = $state('');
  let results = $state<SpotifySearchResults | null>(null);
  let busy = $state(false);
  let error = $state('');
  let timer: ReturnType<typeof setTimeout> | undefined;

  onDestroy(() => { if (timer) clearTimeout(timer); });

  function schedule() {
    if (timer) clearTimeout(timer);
    error = '';
    if (!query.trim()) { results = null; busy = false; return; }
    busy = true;
    timer = setTimeout(search, 350);
  }

  async function search() {
    const value = query.trim();
    if (!value) return;
    try {
      const data = await spotify.search(value);
      if (value === query.trim()) results = data;
    } catch (cause) {
      if (value === query.trim()) error = String(cause);
    } finally {
      if (value === query.trim()) busy = false;
    }
  }

  async function play(uri?: string) {
    if (!uri) return;
    if (await spotify.playUri(uri)) onopen();
  }
</script>

<section class="music-search" aria-label="Search Spotify">
  <div class="music-search-field">
    <span aria-hidden="true">⌕</span>
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
              <button class="music-result" onclick={() => play(track.uri)}>
                {#if track.artworkUrl}<img src={track.artworkUrl} alt="" loading="lazy" />{:else}<span class="music-result-art" aria-hidden="true">♪</span>{/if}
                <span><b>{track.title}</b><small>{track.artist}{track.album ? ` · ${track.album}` : ''}</small></span>
              </button>
            {/each}
          </div>
        {/if}
        {#if results.playlists.length}
          <div>
            <h3>Playlists</h3>
            {#each results.playlists as playlist (playlist.id)}
              <button class="music-result" onclick={() => play(playlist.uri)}>
                {#if playlist.artworkUrl}<img src={playlist.artworkUrl} alt="" loading="lazy" />{:else}<span class="music-result-art" aria-hidden="true">≡</span>{/if}
                <span><b>{playlist.name}</b><small>{playlist.owner ? `By ${playlist.owner}` : 'Playlist'}</small></span>
              </button>
            {/each}
          </div>
        {/if}
        {#if results.albums.length}
          <div>
            <h3>Albums</h3>
            {#each results.albums as album (album.id)}
              <button class="music-result" onclick={() => play(album.uri)}>
                {#if album.artworkUrl}<img src={album.artworkUrl} alt="" loading="lazy" />{:else}<span class="music-result-art" aria-hidden="true">◎</span>{/if}
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
