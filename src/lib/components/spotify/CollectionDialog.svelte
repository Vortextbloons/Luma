<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { spotify } from '../../state/spotify.svelte';
  import type { MusicCollection, SpotifyBrowsePage } from '../../types/spotify';
  import MusicIcon from './MusicIcon.svelte';
  import TrackList from './TrackList.svelte';
  import MusicCard from './MusicCard.svelte';
  let { collection, onclose, onselect, onplay }: { collection: MusicCollection; onclose: () => void; onselect: (item: MusicCollection) => void; onplay: (uri?: string, contextUri?: string) => void } = $props();
  let page = $state.raw<SpotifyBrowsePage | null>(null);
  let loading = $state(false);
  let error = $state('');
  onMount(() => { void load(); });
  async function load(more = false) {
    if (loading) return;
    loading = true; error = '';
    try {
      const previousCount = page ? page.tracks.length + page.albums.length + page.playlists.length : 0;
      const data = await spotify.browse(collection.kind, collection.id, more ? page?.nextOffset : 0);
      page = more && page ? { ...data, tracks: [...page.tracks, ...data.tracks], playlists: [...page.playlists, ...data.playlists], albums: [...page.albums, ...data.albums] } : data;
      if (more) {
        await tick();
        document.querySelectorAll<HTMLElement>('.collection-dialog .tracks button, .collection-dialog .cards button')[previousCount]?.focus();
      }
    } catch (cause) { error = String(cause); }
    finally { loading = false; }
  }
  function mountDialog(node: HTMLDialogElement) {
    node.showModal();
    return () => node.close();
  }
</script>
<dialog class="collection-dialog" aria-labelledby="collection-title" oncancel={(event) => { event.preventDefault(); onclose(); }} {@attach mountDialog}>
  <header><button class="collection-back" onclick={onclose}><MusicIcon name="arrow-left" size={16} /> Back</button><span>Your music · Spotify</span></header>
  <div class="collection-hero">
    <div class="cover">{#if collection.artworkUrl}<img src={collection.artworkUrl} alt="" decoding="async" />{:else}<MusicIcon name={collection.kind === 'tracks' ? 'heart' : 'library'} size={72} />{/if}</div>
    <div class="collection-meta">
      <p class="kind">{collection.kind === 'playlist' ? 'Playlist' : collection.kind === 'album' ? 'Album' : 'Your library'}</p>
      <h1 id="collection-title">{collection.title}</h1>
      {#if collection.subtitle && collection.subtitle !== `${page?.total} tracks`}<p>{collection.subtitle}</p>{/if}
      {#if page}<p class="collection-count">{page.total} {collection.kind === 'playlists' ? 'playlists' : collection.kind === 'albums' ? 'albums' : 'songs'}</p>{/if}
      {#if collection.uri}<div class="collection-actions"><button class="collection-play" disabled={spotify.busy} onclick={() => onplay(collection.uri)}><MusicIcon name="play" size={18} /> Play</button></div>{/if}
    </div>
  </div>
  {#if spotify.error}<p class="error" role="status">{spotify.error}</p>{/if}
  {#if error}<p class="error" role="status">{error}</p><button onclick={() => load(!!page)}>Try again</button>{/if}
  {#if page}
    {#if page.tracks.length}<div class="collection-tracks"><TrackList tracks={page.tracks} artworkUrl={collection.artworkUrl} album={collection.kind === 'album' ? collection.title : undefined} onplay={uri => onplay(uri, collection.uri)} /></div>{/if}
    <div class="cards">
      {#each page.playlists as item (item.id)}<MusicCard title={item.name} artworkUrl={item.artworkUrl} subtitle={item.owner} kind="Playlist" onselect={() => onselect({ id: item.id, title: item.name, uri: item.uri, artworkUrl: item.artworkUrl, subtitle: item.owner, kind: 'playlist' })} />{/each}
      {#each page.albums as item (item.id)}<MusicCard title={item.name} artworkUrl={item.artworkUrl} subtitle={item.artist} kind="Album" onselect={() => onselect({ id: item.id, title: item.name, uri: item.uri, artworkUrl: item.artworkUrl, subtitle: item.artist, kind: 'album' })} />{/each}
    </div>
    {#if !page.tracks.length && !page.albums.length && !page.playlists.length && !loading}<p class="empty">No music to show in this collection.</p>{/if}
    {#if page.nextOffset != null}<button class="more" disabled={loading} onclick={() => load(true)}>{loading ? 'Loading...' : 'Load more'}</button>{/if}
  {/if}
  {#if loading && !page}<p class="empty" role="status">Loading your music...</p>{/if}
</dialog>
<style>
  dialog { position: fixed; inset: 4vh 4vw; width: 92vw; height: 92vh; max-width: none; max-height: none; margin: auto; padding: clamp(20px,2.4vw,44px); color: var(--text); border: 1px solid #ffffff1f; border-radius: 18px; background: linear-gradient(180deg,var(--surface),var(--surface-2) 70%); overflow-y: auto; z-index: 89; }
  dialog::backdrop { background: #060910e0; }
  header { display: flex; align-items: center; justify-content: space-between; gap: 20px; margin-bottom: clamp(20px,2.4vw,34px); }
  header span { color: var(--dim); font-size: clamp(10px,.72vw,12px); letter-spacing: .12em; text-transform: uppercase; }
  .collection-back { display: inline-flex; align-items: center; gap: 7px; min-height: 40px; padding: 0 14px 0 11px; border-radius: 99px; background: #ffffff0f; font-size: clamp(12px,.82vw,14px); font-weight: 600; }
  .collection-back:hover { background: #ffffff1a; }
  button { display: inline-flex; gap: 9px; align-items: center; justify-content: center; min-height: 42px; padding: 0 18px; border-radius: 99px; font-size: clamp(12px,.85vw,15px); background: #ffffff12; }
  button:focus-visible { outline: 3px solid var(--accent); outline-offset: 4px; }
  button:disabled { opacity: .5; }
  .collection-hero { display: grid; grid-template-columns: clamp(140px,17vw,232px) minmax(0,1fr); align-items: end; gap: clamp(20px,2.4vw,40px); margin-bottom: clamp(22px,2.6vw,36px); }
  .cover { aspect-ratio: 1; display: grid; place-items: center; background: linear-gradient(140deg,var(--surface-3),var(--surface-2)); border-radius: 10px; overflow: hidden; box-shadow: 0 20px 50px #0007; color: #ffffff38; }
  img { width: 100%; height: 100%; object-fit: cover; }
  .collection-meta { min-width: 0; }
  .kind { color: var(--dim); font-size: clamp(10px,.72vw,12px); font-weight: 700; letter-spacing: .12em; text-transform: uppercase; margin: 0 0 8px; }
  h1 { font-size: clamp(28px,3.2vw,64px); font-weight: 700; letter-spacing: -.045em; line-height: 1.04; margin: 0 0 12px; overflow-wrap: anywhere; }
  .collection-meta p { font-size: clamp(12px,.9vw,16px); color: var(--text-dim); line-height: 1.5; margin: 0 0 6px; }
  .collection-count { color: var(--muted); }
  .collection-actions { display: flex; gap: 10px; margin-top: 14px; }
  .collection-play { background: var(--accent); color: var(--accent-ink); font-weight: 700; }
  .collection-play:hover:not(:disabled) { filter: brightness(1.08); }
  .collection-tracks { padding: 6px; border: 1px solid #ffffff12; border-radius: 14px; background: #ffffff06; }
  .cards { display: grid; grid-template-columns: repeat(auto-fill,minmax(clamp(140px,11vw,180px),1fr)); gap: clamp(16px,1.6vw,26px); }
  .more { margin: 26px 0 4px; }
  .empty { padding: 24px 0; color: var(--muted); font-size: clamp(12px,.9vw,16px); }
  .error { position: sticky; top: 0; z-index: 1; color: var(--danger); background: #261b25; border-radius: 10px; padding: 14px 18px; font-size: clamp(12px,.9vw,15px); }
  @media(max-width:650px) { .collection-hero { grid-template-columns: 1fr; } .cover { width: 140px; } header span { display: none; } }
</style>
