<script lang="ts">
  import MusicIcon from './MusicIcon.svelte';
  import { onDestroy, untrack, tick } from 'svelte';
  import CollectionDialog from './CollectionDialog.svelte';
  import DevicePicker from './DevicePicker.svelte';
  import TrackList from './TrackList.svelte';
  import { native } from '../../services/tauri';
  import { spotify } from '../../state/spotify.svelte';
  import type { MusicCollection } from '../../types/spotify';
  import MusicSearch from './MusicSearch.svelte';
  import PlaylistRow from './PlaylistRow.svelte';
  import PlaybackControls from './PlaybackControls.svelte';

  let { onopen, onsettings, ondetailchange }: { onopen: () => void; onsettings: () => void; ondetailchange?: (open: boolean) => void } = $props();
  const playback = $derived(spotify.playback);
  const connection = $derived(spotify.connection);
  const home = $derived(spotify.home);

  let section = $state('Listen now');
  let collections = $state<MusicCollection[]>([]);
  const collection = $derived(collections.at(-1));
  let collectionPrevious: HTMLElement | null = null;
  onDestroy(() => ondetailchange?.(false));
  function browse(item: MusicCollection) {
    if (!collections.length) collectionPrevious = document.activeElement as HTMLElement;
    collections = [...collections, item];
    ondetailchange?.(true);
  }
  export function closeDetail() {
    if (!collections.length) return false;
    collections = collections.slice(0, -1);
    ondetailchange?.(collections.length > 0);
    void tick().then(() => {
      if (collections.length) document.querySelector<HTMLElement>('.collection-back')?.focus();
      else collectionPrevious?.focus();
    });
    return true;
  }


  const playlists = $derived(home.playlists.map(item => ({
    id: item.id, title: item.name, uri: item.uri, artworkUrl: item.artworkUrl,
    subtitle: item.trackCount ? `${item.trackCount} tracks` : item.owner,
  })));
  const albums = $derived(home.albums.map(item => ({
    id: item.id, title: item.name, uri: item.uri, artworkUrl: item.artworkUrl,
    subtitle: [item.artist, item.year].filter(Boolean).join(' · '),
  })));
  const recent = $derived(home.recentlyPlayed.map((item, index) => ({
    id: `${item.id ?? item.title}-${index}`, title: item.title, uri: item.uri, artworkUrl: item.artworkUrl,
    subtitle: item.artist,
  })));

  $effect(() => {
    if (spotify.connection.authenticated) untrack(() => { void spotify.loadHome(); });
  });


  async function play(uri?: string) {
    if (await spotify.playUri(uri)) onopen();
  }

  function action(name: string) { void spotify.action(name); }
</script>

<section class="music-home" aria-labelledby="music-title">
  <header class="music-page-header">
    <div>
      <span class="panel-kicker">LUMA MUSIC</span>
      <h1 id="music-title">Music</h1>
      <p>Your music. Room to listen.</p>
    </div>
    <div class="music-status">
      <span class={["music-status-led", { "inactive": !playback.playerAvailable }]} aria-hidden="true"></span>
      <span>{connection.authenticated ? (connection.displayName ? `Connected as ${connection.displayName}` : 'Spotify account connected') : playback.playerAvailable ? 'Spotify controls ready' : connection.processRunning ? 'Spotify is running' : 'Spotify is not active'}</span>
      {#if connection.authenticated}<button class="text-link" disabled={spotify.homeLoading} onclick={() => spotify.loadHome(true)}>{spotify.homeLoading ? 'Refreshing…' : 'Refresh library'}</button>{/if}
      <button class="text-link" onclick={onsettings}>Manage Spotify</button>
    </div>
  </header>

  <nav class="music-sections" aria-label="Music sections">
    {#each ['Listen now', 'Library', 'Search', 'Devices'] as name (name)}
      <button class={{ "active": section === name }} aria-current={section === name ? 'page' : undefined} onclick={() => section = name}><MusicIcon name={name === 'Listen now' ? 'music' : name === 'Library' ? 'library' : name === 'Search' ? 'search' : 'speaker'} />{name}</button>
    {/each}
  </nav>
  {#if !native}
    <p class="music-inline-error">Music controls require the Luma desktop app.</p>
  {/if}

  {#if spotify.error}<p class="music-inline-error" role="status">{spotify.error}</p>{/if}
  {#if connection.error}<p class="music-inline-error" role="status">{connection.error}</p>{:else if playback.error && connection.processRunning}<p class="music-inline-error" role="status">{playback.error}</p>{/if}

  {#if connection.starting}
    <section class="music-starting">
      <span class="music-pulse" aria-hidden="true"></span>
      <div><h2>Starting Spotify in the background</h2><p>Luma is waiting for Spotify to be ready. Spotify may briefly show a window while it starts.</p></div>
    </section>
  {:else if !connection.processRunning && !connection.playerAvailable}
    <section class="music-starting">
      <div>
        <h2>{connection.installed ? 'Spotify is not running' : 'Spotify was not found'}</h2>
        <p>{connection.installed ? 'Start Spotify and Luma will handle the playback controls.' : 'Install the Spotify desktop app on this PC, then return to Luma.'}</p>
      </div>
      {#if connection.installed}<button class="btn btn-glass" disabled={spotify.busy} onclick={() => spotify.start()}>Start Spotify</button>{:else}<button class="btn btn-glass" onclick={onsettings}>Open settings</button>{/if}
    </section>
  {/if}

  {#if section === 'Listen now' && (connection.processRunning || playback.playerAvailable || playback.title)}
  <section class="music-now-card" aria-label="Currently playing">
    <div class="music-now-art">
      {#if playback.artworkUrl}<img src={playback.artworkUrl} alt="" />{:else}<span aria-hidden="true"><MusicIcon name="music" size={72} /></span>{/if}
    </div>
    <div class="music-now-copy">
      <span class="panel-kicker">{playback.playing ? 'NOW PLAYING' : playback.title ? 'PAUSED' : 'READY WHEN YOU ARE'}</span>
      <h2>{playback.title ?? 'Spotify is not playing'}</h2>
      <p>{[playback.artist, playback.album].filter(Boolean).join(' · ') || (playback.playerAvailable ? 'Start a track in Spotify or search your library.' : connection.processRunning ? 'Spotify is running. Start a track in Spotify to connect.' : 'Start Spotify and play something to connect.')}</p>
      <div class="music-progress" aria-hidden="true"><span style:transform={`scaleX(${spotify.progress})`}></span></div>
      <div class="music-now-actions">
        <PlaybackControls {playback} busy={spotify.busy} onaction={action} />
        <button class="btn btn-glass" onclick={onopen}>Open full player</button>
      </div>
      {#if playback.deviceName}<p class="music-device-label">Playing on {playback.deviceName}</p>{/if}
    </div>
  </section>
  {/if}

  {#if connection.authenticated}
    <div hidden={section !== 'Search'}><MusicSearch {onopen} onbrowse={browse} /></div>
    {#if section === 'Devices'}<DevicePicker />{/if}
    {#if section === 'Listen now' || section === 'Library'}

    {#if home.warnings.length}<p class="music-inline-error" role="status">Some Spotify sections could not be loaded. {home.warnings[0]}</p>{/if}

    {#if section === 'Listen now'}<PlaylistRow title="Recently played" subtitle="From your Spotify account" items={recent} empty={spotify.homeLoading ? 'Loading recent tracks…' : 'No recent Spotify tracks yet.'} kind="Track" onselect={item => play(item.uri)} />{/if}
    <PlaylistRow title="Your playlists" subtitle="Owned and followed playlists" items={playlists} empty={spotify.homeLoading ? 'Loading playlists…' : 'No playlists found.'} kind="Playlist" onselect={item => browse({ ...item, kind: 'playlist' })} />
    <button class="library-more" onclick={() => browse({ id: '', title: 'Your playlists', kind: 'playlists' })}>Browse all playlists <MusicIcon name="chevron-right" /></button>
    <PlaylistRow title="Saved albums" subtitle="From your Spotify library" items={albums} empty={spotify.homeLoading ? 'Loading saved albums…' : 'No saved albums.'} kind="Album" onselect={item => browse({ ...item, kind: 'album' })} />
    <button class="library-more" onclick={() => browse({ id: '', title: 'Saved albums', kind: 'albums' })}>Browse all albums <MusicIcon name="chevron-right" /></button>

    <section class="content-row music-row">
      <div class="row-heading"><div><h2>Liked songs</h2><p>From your Spotify library</p></div></div>
      <TrackList tracks={home.tracks.slice(0, 5)} onplay={play} />
      {#if !home.tracks.length}<p class="music-empty-line">{spotify.homeLoading ? 'Loading liked songs...' : 'Save songs in Spotify to find them here.'}</p>{/if}
      <button class="library-more" onclick={() => browse({ id: '', title: 'Liked songs', subtitle: 'Saved in your Spotify library', kind: 'tracks' })}>Browse all liked songs <MusicIcon name="chevron-right" /></button>
    </section>

    {/if}

  {:else}
    <section class="music-account">
      <div>
        <span class="panel-kicker">OPTIONAL</span>
        <h2>Connect your Spotify account</h2>
        <p>Adds playlists, saved music, search, recent listening, and device controls. Playback stays in the Spotify desktop app. The controls above work without signing in.</p>
      </div>
      <div class="music-account-actions">
        <button class="btn btn-glass" onclick={onsettings}>Set up Spotify</button>
        <p class="music-note">Spotify Premium is required for Web API playback control. Development Mode apps have limited endpoint access.</p>
      </div>
    </section>
  {/if}
</section>

{#if collection}
  {#key collection}
    <CollectionDialog {collection} onclose={closeDetail} onselect={browse} onplay={async (uri, contextUri) => { if (await spotify.playUri(uri, contextUri)) { collections = []; ondetailchange?.(false); await tick(); collectionPrevious?.focus(); onopen(); } }} />
  {/key}
{/if}

<style>
  .music-home { --music-ink: #edf1fc; --music-muted: #aeb8cf; color: var(--music-ink); }
  .music-sections { display: flex; flex-wrap: wrap; gap: 12px; margin-bottom: 36px; }
  .music-sections button,.library-more { display: inline-flex; align-items: center; justify-content: center; gap: 12px; min-height: 58px; padding: 14px 24px; border-radius: 99px; background: #ffffff09; border: 1px solid #ffffff15; font-size: clamp(18px,1.2vw,24px); color: #b8c3da; }
  .music-sections .active { background: #e1e8fa; color: #142036; border-color: transparent; }
  .music-sections button:focus-visible,.library-more:focus-visible { outline: 3px solid var(--accent); outline-offset: 5px; }
  .music-home .btn { min-height: 56px; font-size: 18px; padding: 14px 24px; }
  .music-now-card { background: linear-gradient(125deg,#263954,#151c2a 75%); border-radius: 24px; }
  .music-now-art img { object-fit: contain; } .music-now-art { border-radius: 10px; }
  .music-status { font-size: 16px; border-radius: 16px; max-width: 440px; padding: 16px 20px; }
  .music-status .text-link { font-size: 16px; min-height: 44px; }
  .library-more { margin: -8px 0 36px; background: transparent; border-color: #ffffff24; }
  .music-home :global(.music-row .row-heading h2) { font-size: clamp(26px,2vw,38px); }
  .music-home :global(.music-row .row-heading p) { font-size: 18px; color: var(--music-muted); }
  .music-home :global(.music-inline-error),.music-home :global(.music-empty-line) { font-size: 18px; }
  .music-account p,.music-starting p { font-size: 19px; } .music-note { font-size: 16px; }
  @media(max-width:900px) { .music-page-header { align-items: flex-start; flex-direction: column; } .music-status { max-width: none; justify-content: flex-start; } }
</style>
