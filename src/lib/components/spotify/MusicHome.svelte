<script lang="ts">
  import { onMount } from 'svelte';
  import { native } from '../../services/tauri';
  import { spotify } from '../../state/spotify.svelte';
  import type { SpotifyDevice } from '../../types/spotify';
  import MusicSearch from './MusicSearch.svelte';
  import PlaylistRow from './PlaylistRow.svelte';
  import PlaybackControls from './PlaybackControls.svelte';

  let { onopen, onsettings }: { onopen: () => void; onsettings: () => void } = $props();
  const playback = $derived(spotify.playback);
  const connection = $derived(spotify.connection);
  const home = $derived(spotify.home);

  let transferring = $state<string | null>(null);

  const playlists = $derived(home.playlists.map(item => ({
    id: item.id, title: item.name, uri: item.uri, artworkUrl: item.artworkUrl,
    subtitle: item.trackCount ? `${item.trackCount} tracks` : item.owner,
  })));
  const albums = $derived(home.albums.map(item => ({
    id: item.id, title: item.name, uri: item.uri, artworkUrl: item.artworkUrl,
    subtitle: [item.artist, item.year].filter(Boolean).join(' · '),
  })));
  const recent = $derived(home.recentlyPlayed.map((item, index) => ({
    id: item.id ?? `${item.title}-${index}`, title: item.title, uri: item.uri, artworkUrl: item.artworkUrl,
    subtitle: item.artist,
  })));

  $effect(() => {
    if (spotify.connection.authenticated) void spotify.loadHome();
  });

  onMount(() => { void spotify.refresh(); });

  async function play(uri?: string) {
    if (await spotify.playUri(uri)) onopen();
  }

  async function useDevice(device: SpotifyDevice) {
    if (!device.id || device.active || device.restricted) return;
    transferring = device.id;
    try {
      await spotify.transfer(device.id);
      await spotify.loadHome(true);
    } finally { transferring = null; }
  }

  function action(name: string) { void spotify.action(name); }
</script>

<section class="music-home" aria-labelledby="music-title">
  <header class="music-page-header">
    <div>
      <span class="panel-kicker">LUMA MUSIC</span>
      <h1 id="music-title">Music</h1>
      <p>Spotify on this PC, controlled from Luma.</p>
    </div>
    <div class="music-status">
      <span class:inactive={!playback.playerAvailable} class="music-status-led" aria-hidden="true"></span>
      <span>{connection.authenticated ? (connection.displayName ? `Connected as ${connection.displayName}` : 'Spotify account connected') : playback.playerAvailable ? 'Spotify controls ready' : connection.processRunning ? 'Spotify is running' : 'Spotify is not active'}</span>
      {#if connection.authenticated}<button class="text-link" disabled={spotify.homeLoading} onclick={() => spotify.loadHome(true)}>{spotify.homeLoading ? 'Refreshing…' : 'Refresh library'}</button>{/if}
      <button class="text-link" onclick={onsettings}>Manage Spotify</button>
    </div>
  </header>

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

  {#if connection.processRunning || playback.playerAvailable || playback.title}
  <section class="music-now-card" aria-label="Currently playing">
    <div class="music-now-art">
      {#if playback.artworkUrl}<img src={playback.artworkUrl} alt="" />{:else}<span aria-hidden="true">♪</span>{/if}
    </div>
    <div class="music-now-copy">
      <span class="panel-kicker">{playback.playing ? 'NOW PLAYING' : playback.title ? 'PAUSED' : 'READY WHEN YOU ARE'}</span>
      <h2>{playback.title ?? 'Spotify is not playing'}</h2>
      <p>{[playback.artist, playback.album].filter(Boolean).join(' · ') || (playback.playerAvailable ? 'Start a track in Spotify or search your library.' : connection.processRunning ? 'Spotify is running. Start a track in Spotify to connect.' : 'Start Spotify and play something to connect.')}</p>
      <div class="music-progress" aria-hidden="true"><span style:width={`${Math.round(spotify.progress * 100)}%`}></span></div>
      <div class="music-now-actions">
        <PlaybackControls {playback} busy={spotify.busy} onaction={action} />
        <button class="btn btn-glass" onclick={onopen}>Open full player</button>
      </div>
      {#if playback.deviceName}<p class="music-device-label">Playing on {playback.deviceName}</p>{/if}
    </div>
  </section>
  {/if}

  {#if connection.authenticated}
    <MusicSearch {onopen} />

    {#if home.warnings.length}<p class="music-inline-error" role="status">Some Spotify sections could not be loaded. {home.warnings[0]}</p>{/if}

    <PlaylistRow title="Recently played" subtitle="From your Spotify account" items={recent} empty={spotify.homeLoading ? 'Loading recent tracks…' : 'No recent Spotify tracks yet.'} kind="Track" onselect={item => play(item.uri)} />
    <PlaylistRow title="Your playlists" subtitle="Owned and followed playlists" items={playlists} empty={spotify.homeLoading ? 'Loading playlists…' : 'No playlists found.'} kind="Playlist" onselect={item => play(item.uri)} />
    <PlaylistRow title="Saved albums" subtitle="From your Spotify library" items={albums} empty={spotify.homeLoading ? 'Loading saved albums…' : 'No saved albums.'} kind="Album" onselect={item => play(item.uri)} />

    <section class="content-row music-row">
      <div class="row-heading"><div><h2>Saved tracks</h2><p>From your Spotify library</p></div></div>
      {#if home.tracks.length}
        <div class="music-track-list">
          {#each home.tracks as track, index (track.id ?? `${track.title}-${index}`)}
            <button class="music-track" onclick={() => play(track.uri)}>
              <span class="music-track-art" aria-hidden="true">♪</span>
              <span class="music-track-copy"><b>{track.title}</b><small>{track.artist}</small></span>
              <span class="music-track-album">{track.album}</span>
            </button>
          {/each}
        </div>
      {:else}
        <p class="music-empty-line">{spotify.homeLoading ? 'Loading saved tracks…' : 'No saved tracks.'}</p>
      {/if}
    </section>

    <section class="content-row music-row">
      <div class="row-heading"><div><h2>Playback devices</h2><p>Spotify Connect devices on your network</p></div><button class="text-link" onclick={() => spotify.loadHome(true)}>Refresh</button></div>
      {#if home.devices.length}
        <div class="music-devices">
          {#each home.devices as device (device.id ?? device.name)}
            <div class:active={device.active} class="music-device">
              <div><b>{device.name}</b><small>{device.kind}{device.active ? ' · Active' : ''}{device.restricted ? ' · Restricted' : ''}</small></div>
              <button class="text-link" disabled={!device.id || device.active || device.restricted || transferring === device.id} onclick={() => useDevice(device)}>{device.active ? 'In use' : transferring === device.id ? 'Switching…' : 'Use device'}</button>
            </div>
          {/each}
        </div>
      {:else}
        <p class="music-empty-line">No Spotify Connect devices are available. Open Spotify on this PC.</p>
      {/if}
    </section>
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
