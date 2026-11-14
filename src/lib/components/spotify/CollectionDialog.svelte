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
  <header><button class="collection-back" onclick={onclose}><MusicIcon name="arrow-left" /> Back</button><span>YOUR MUSIC - SPOTIFY</span></header>
  <div class="collection-hero">
    <div class="cover">{#if collection.artworkUrl}<img src={collection.artworkUrl} alt="" decoding="async" />{:else}<MusicIcon name={collection.kind === 'tracks' ? 'heart' : 'library'} size={80} />{/if}</div>
    <div><p class="kind">{collection.kind === 'playlist' ? 'Playlist' : collection.kind === 'album' ? 'Album' : 'Your library'}</p><h1 id="collection-title">{collection.title}</h1>{#if collection.subtitle && collection.subtitle !== `${page?.total} tracks`}<p>{collection.subtitle}</p>{/if}{#if page}<p>{page.total} {collection.kind === 'playlists' ? 'playlists' : collection.kind === 'albums' ? 'albums' : 'tracks'}</p>{/if}
    {#if collection.uri}<button class="play" disabled={spotify.busy} onclick={() => onplay(collection.uri)}><MusicIcon name="play" /> Play {collection.kind}</button>{/if}</div>
  </div>
  {#if spotify.error}<p class="error" role="status">{spotify.error}</p>{/if}
  {#if error}<p class="error" role="status">{error}</p><button onclick={() => load(!!page)}>Try again</button>{/if}
  {#if page}
    <TrackList tracks={page.tracks} artworkUrl={collection.artworkUrl} album={collection.kind === 'album' ? collection.title : undefined} onplay={uri => onplay(uri, collection.uri)} />
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
  dialog { position: fixed; inset: 4vh 4vw; width: 92vw; height: 92vh; max-width: none; max-height: none; margin: auto; padding: clamp(24px,3vw,56px); color: var(--text); border: 1px solid #ffffff24; border-radius: 24px; background: linear-gradient(145deg,var(--surface),var(--surface-2) 60%); overflow-y: auto; z-index: 89; }
  dialog::backdrop { background: #060910dd; }
  header { display: flex; align-items: center; justify-content: space-between; gap: 24px; margin-bottom: 36px; } header span,.kind { color: var(--text-dim); font-size: 16px; letter-spacing: .12em; }
  button { display: inline-flex; gap: 12px; align-items: center; justify-content: center; min-height: 56px; padding: 14px 24px; border-radius: 99px; font-size: 19px; background: #ffffff15; }
  button:focus-visible { outline: 3px solid var(--accent); outline-offset: 5px; } button:disabled { opacity: .5; }
  .collection-hero { display: grid; grid-template-columns: minmax(160px,22vw) 1fr; align-items: end; gap: 36px; margin-bottom: 36px; } .cover { aspect-ratio: 1; display: grid; place-items: center; background: var(--surface-2); border-radius: 12px; overflow: hidden; } img { width: 100%; height: 100%; object-fit: contain; }
  h1 { font-size: clamp(36px,4vw,76px); letter-spacing: -.045em; line-height: 1.05; margin: 12px 0 18px; overflow-wrap: anywhere; } p { font-size: 20px; color: var(--text-dim); line-height: 1.5; } .play { background: var(--text); color: var(--surface-2); margin-top: 12px; }
  .cards { display: grid; grid-template-columns: repeat(auto-fill,minmax(180px,1fr)); gap: 28px; } .more { margin: 28px 0; } .empty { padding: 28px 0; } .error { position: sticky; top: 0; z-index: 1; color: var(--danger); background: #261b25; border-radius: 12px; padding: 16px 20px; }
  @media(max-width:650px) { .collection-hero { grid-template-columns: 1fr; } .cover { width: 160px; } header span { display: none; } }
</style>
