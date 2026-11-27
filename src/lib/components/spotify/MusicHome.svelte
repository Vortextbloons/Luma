<script lang="ts">
  import MusicIcon from './MusicIcon.svelte';
  import PageHeader from '../PageHeader.svelte';
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
  let accountMenu: HTMLDetailsElement;
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
    if (accountMenu?.open) {
      accountMenu.open = false;
      accountMenu.querySelector('summary')?.focus();
      return true;
    }
    if (!collections.length) return false;
    collections = collections.slice(0, -1);
    ondetailchange?.(collections.length > 0);
    void tick().then(() => {
      if (collections.length) document.querySelector<HTMLElement>('.collection-back')?.focus();
      else collectionPrevious?.focus();
    });
    return true;
  }

  const sections = [
    { name: 'Listen now', icon: 'home', group: '' },
    { name: 'Search', icon: 'search', group: '' },
    { name: 'Library', icon: 'library', group: 'Library' },
    { name: 'Devices', icon: 'speaker', group: 'Library' },
  ];

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

  function time(value?: number) {
    if (value == null) return '0:00';
    const seconds = Math.max(0, Math.floor(value / 1000));
    return `${Math.floor(seconds / 60)}:${String(seconds % 60).padStart(2, '0')}`;
  }

  function animateSection(_section: string) {
    return (element: HTMLElement) => {
      if (window.matchMedia('(prefers-reduced-motion: reduce)').matches) return;
      const animation = element.animate(
        [{ opacity: .4, transform: 'translateY(10px)' }, { opacity: 1, transform: 'none' }],
        { duration: 180, easing: 'cubic-bezier(.22,.8,.24,1)' },
      );
      return () => animation.cancel();
    };
  }
</script>

<section class="music-home" aria-labelledby="music-title">
  <div class="music-shell">
    <aside class="music-rail" aria-label="Music navigation">
      <div class="music-rail-brand">
        <span class="music-rail-mark" aria-hidden="true"><MusicIcon name="music" size={18} /></span>
        <span><strong>Music</strong><small>Luma</small></span>
      </div>

      <nav class="music-sections">
        {#each [{ group: '', items: sections.filter(item => !item.group) }, { group: 'Library', items: sections.filter(item => item.group) }] as block (block.group)}
          {#if block.items.length}
            <div class="music-group">
              {#if block.group}<span class="music-group-label">{block.group}</span>{/if}
              {#each block.items as item (item.name)}
                <button class={{ "active": section === item.name }} aria-current={section === item.name ? 'page' : undefined} onclick={() => section = item.name}><MusicIcon name={item.icon} />{item.name}</button>
              {/each}
            </div>
          {/if}
        {/each}
      </nav>

      <details class="music-menu" bind:this={accountMenu}>
        <summary aria-label="Music options"><span class="music-avatar" aria-hidden="true">{#if connection.displayName}{connection.displayName.slice(0, 1).toUpperCase()}{:else}<MusicIcon name="music" size={16} />{/if}</span><span class="music-account-copy"><b>{connection.displayName || 'Your music'}</b><small>{connection.authenticated ? 'Connected' : 'Not connected'}</small></span><MusicIcon name="chevron-down" size={14} /></summary>
        <div class="music-menu-panel">
          {#if connection.authenticated}<button disabled={spotify.homeLoading} onclick={() => spotify.loadHome(true)}>{spotify.homeLoading ? 'Refreshing...' : 'Refresh library'}</button>{/if}
          <button onclick={onsettings}>{connection.authenticated ? 'Account settings' : 'Connect Spotify'}</button>
        </div>
      </details>
    </aside>

    <div class="music-content">
      <PageHeader flush divided titleId="music-title"
        eyebrow={section === 'Listen now' ? 'YOUR LIBRARY' : section.toUpperCase()}
        title={section === 'Listen now' ? 'Listen now' : section}
        description={section === 'Listen now' ? 'Pick up where you left off.' : section === 'Search' ? 'Find songs, albums, and playlists on Spotify.' : section === 'Library' ? 'Everything saved to your Spotify library.' : 'Send playback to another speaker or device.'} />

      <div class="music-section-body" {@attach animateSection(section)}>
      {#if !native}
        <p class="music-inline-error">Music controls require the Luma desktop app.</p>
      {/if}

      {#if spotify.error}<p class="music-inline-error" role="status">{spotify.error}</p>{/if}
      {#if connection.error}<p class="music-inline-error" role="status">{connection.error}</p>{:else if playback.error && connection.processRunning}<p class="music-inline-error" role="status">{playback.error}</p>{/if}

      {#if connection.starting}
        <section class="music-starting">
          <span class="music-pulse" aria-hidden="true"></span>
          <div><h2>Getting your music ready</h2><p>Just a moment...</p></div>
        </section>
      {:else if !connection.processRunning && !connection.playerAvailable}
        <section class="music-starting">
          <div>
            <h2>{connection.installed ? 'Ready to listen?' : 'Spotify was not found'}</h2>
            <p>{connection.installed ? 'Start Spotify in the background to begin listening.' : 'Install the Spotify desktop app on this PC, then return to Luma.'}</p>
          </div>
          {#if connection.installed}<button class="btn btn-glass" disabled={spotify.busy} onclick={() => spotify.start()}>Start Spotify</button>{:else}<button class="btn btn-glass" onclick={onsettings}>Open settings</button>{/if}
        </section>
      {/if}

      {#if section === 'Listen now' && (connection.processRunning || playback.playerAvailable || playback.title)}
      <section class="music-now-card" aria-label="Currently playing">
        <div class="music-now-art">
          {#if playback.artworkUrl}<img src={playback.artworkUrl} alt="" decoding="async" />{:else}<span aria-hidden="true"><MusicIcon name="music" size={64} /></span>{/if}
        </div>
        <div class="music-now-copy">
          <div class="music-now-head">
            <span class="eyebrow is-flush">{playback.playing ? 'NOW PLAYING' : playback.title ? 'PAUSED' : 'READY WHEN YOU ARE'}</span>
            <button class="btn btn-glass" onclick={onopen}>Open player</button>
          </div>
          <h2>{playback.title ?? 'Find your next favorite'}</h2>
          <p>{[playback.artist, playback.album].filter(Boolean).join(' · ') || 'Explore your library or search for something new.'}</p>
          <div class="music-progress" aria-hidden="true"><span style:transform={`scaleX(${spotify.progress})`}></span></div>
          <div class="music-now-time" aria-hidden="true"><span>{time(spotify.positionMs)}</span><span>{time(playback.durationMs)}</span></div>
          <div class="music-now-actions">
            <PlaybackControls {playback} busy={spotify.busy} onaction={action} />
            {#if playback.deviceName}<p class="music-device-label">Playing on {playback.deviceName}</p>{/if}
          </div>
        </div>
      </section>
      {/if}

      {#if connection.authenticated}
        <div hidden={section !== 'Search'}><MusicSearch {onopen} onbrowse={browse} /></div>
        {#if section === 'Devices'}<DevicePicker />{/if}
        {#if section === 'Listen now' || section === 'Library'}

        {#if home.warnings.length}<p class="music-inline-error" role="status">Some music is unavailable right now. Try refreshing your library from the account menu.</p>{/if}

        {#if section === 'Listen now'}<PlaylistRow title="Recently played" items={recent} empty={spotify.homeLoading ? 'Loading recent tracks…' : 'No recent Spotify tracks yet.'} kind="Track" onselect={item => play(item.uri)} />{/if}
        <PlaylistRow title="Your playlists" items={playlists} empty={spotify.homeLoading ? 'Loading playlists…' : 'No playlists found.'} kind="Playlist" onselect={item => browse({ ...item, kind: 'playlist' })} onmore={() => browse({ id: '', title: 'Your playlists', kind: 'playlists' })} />
        <PlaylistRow title="Saved albums" items={albums} empty={spotify.homeLoading ? 'Loading saved albums…' : 'No saved albums.'} kind="Album" onselect={item => browse({ ...item, kind: 'album' })} onmore={() => browse({ id: '', title: 'Saved albums', kind: 'albums' })} />

        <section class="content-row music-row">
          <div class="row-heading">
            <div><h2>Liked songs</h2></div>
            <button class="music-more" onclick={() => browse({ id: '', title: 'Liked songs', subtitle: 'Saved in your Spotify library', kind: 'tracks' })}>See all <MusicIcon name="chevron-right" size={14} /></button>
          </div>
          {#if home.tracks.length}
            <div class="music-list-card"><TrackList tracks={home.tracks.slice(0, 8)} onplay={play} /></div>
          {:else}
            <p class="music-empty-line">{spotify.homeLoading ? 'Loading liked songs...' : 'Save songs in Spotify to find them here.'}</p>
          {/if}
        </section>

        {/if}

      {:else}
        <section class="music-account">
          <div>
            <span class="eyebrow is-flush">MADE FOR YOU</span>
            <h2>Connect your Spotify account</h2>
            <p>Bring your playlists, favorite albums, and liked songs together.</p>
          </div>
          <div class="music-account-actions">
            <button class="btn btn-glass" onclick={onsettings}>Connect Spotify</button>
          </div>
        </section>
      {/if}
      </div>
    </div>
  </div>
</section>

{#if collection}
  {#key collection}
    <CollectionDialog {collection} onclose={closeDetail} onselect={browse} onplay={async (uri, contextUri) => { if (await spotify.playUri(uri, contextUri)) { collections = []; ondetailchange?.(false); await tick(); collectionPrevious?.focus(); onopen(); } }} />
  {/key}
{/if}

<style>
  .music-home { --music-ink: var(--text); --music-muted: var(--muted); color: var(--music-ink); }
  .music-home .btn { min-height: 44px; font-size: clamp(13px,.88vw,16px); padding: 0 20px; }
  .music-home .btn { width: auto; min-width: 0; }
  .music-menu { position: relative; flex: none; z-index: 2; margin-top: auto; }
  .music-menu summary { display: flex; align-items: center; gap: 10px; min-height: 52px; padding: 6px 10px; border: 1px solid #ffffff14; border-radius: 12px; cursor: pointer; list-style: none; background: #ffffff06; }
  .music-menu summary::-webkit-details-marker { display: none; }
  .music-menu summary > :global(svg) { flex: none; color: var(--dim); transform: rotate(0deg); }
  .music-avatar { display: grid; place-items: center; flex: none; width: 30px; height: 30px; border-radius: 50%; background: var(--accent-soft); color: var(--text); font-weight: 700; font-size: 13px; }
  .music-account-copy { display: grid; gap: 1px; min-width: 0; flex: 1; }
  .music-account-copy b { font-size: clamp(12px,.82vw,14px); font-weight: 600; }
  .music-account-copy small { font-size: clamp(10px,.7vw,12px); color: var(--muted); }
  .music-account-copy b, .music-account-copy small { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .music-menu-panel { position: absolute; left: 0; top: calc(100% + 8px); min-width: 210px; max-width: 100%; padding: 6px; background: var(--surface-3); border: 1px solid #ffffff20; border-radius: 12px; box-shadow: 0 16px 40px #0006; z-index: 3; }
  .music-menu-panel button { display: block; width: 100%; min-height: 42px; padding: 10px 12px; text-align: left; border-radius: 8px; font-size: clamp(12px,.82vw,14px); }
  .music-menu-panel button:hover { background: #ffffff0c; }
  .music-menu summary:focus-visible,.music-menu-panel button:focus-visible { outline: 2px solid var(--accent); outline-offset: 2px; }
  .music-home :global(.music-row .row-heading h2) { font-size: clamp(18px,1.45vw,26px); }
  .music-home :global(.music-row .row-heading p) { font-size: clamp(12px,.82vw,15px); color: var(--music-muted); }
  .music-home :global(.music-inline-error),.music-home :global(.music-empty-line) { font-size: clamp(12px,.85vw,15px); }
  .music-account p,.music-starting p { font-size: clamp(13px,.9vw,16px); }
  @media(max-width:1080px) { .music-shell { grid-template-columns: minmax(0,1fr); } .music-rail { position: static; flex-direction: row; flex-wrap: wrap; align-items: center; gap: 14px; } .music-sections { flex-direction: row; flex-wrap: wrap; } .music-sections button { width: auto; padding: 0 14px; border: 1px solid #ffffff14; border-radius: 99px; } .music-group { flex-direction: row; gap: 6px; align-items: center; } .music-group-label { display: none; } .music-menu { margin: 0; } .music-menu-panel { left: auto; right: 0; bottom: auto; top: calc(100% + 8px); } }
  @media(max-width:720px) { .music-rail-brand small { display: none; } }
</style>
