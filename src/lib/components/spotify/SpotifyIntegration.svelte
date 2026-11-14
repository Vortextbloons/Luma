<script lang="ts">
  import { onMount } from 'svelte';
  import { spotify } from '../../state/spotify.svelte';

  let { native }: { native: boolean } = $props();
  let clientId = $state('');
  let synced = $state(false);
  let saving = $state(false);
  let message = $state('');

  const connection = $derived(spotify.connection);

  $effect(() => {
    if (!synced && spotify.clientId) {
      clientId = spotify.clientId;
      synced = true;
    }
  });

  onMount(() => { void spotify.refresh(); });

  async function save() {
    saving = true;
    try {
      if (await spotify.saveClientId(clientId)) message = 'Spotify Client ID saved.';
    } finally {
      saving = false;
    }
  }

  async function connect() {
    message = '';
    if (await spotify.connect()) message = 'Spotify account connected.';
  }

  async function disconnect() {
    if (await spotify.disconnect()) message = 'Spotify account disconnected.';
  }

  async function start() {
    if (await spotify.start()) message = 'Spotify is starting in the background. Luma will connect when a media session appears.';
  }
</script>

<section class="settings-group">
  <h3>Spotify</h3>
  <p class="panel-description">Uses the installed Windows Spotify app for playback. The optional Spotify account connection adds playlists, saved music, search, recent listening, queue, volume, and device controls. Playback audio still runs in Spotify.</p>

  <p class="remote-status">{native ? connection.processRunning ? 'Spotify desktop is running.' : connection.installed ? 'Spotify desktop is installed but not running.' : 'Spotify desktop was not found on this PC.' : 'Available in the desktop application.'}</p>
  <p class="remote-status">{connection.authenticated ? `Account connected${connection.displayName ? ` as ${connection.displayName}` : ''}.` : 'Spotify account not connected.'}</p>
  {#if connection.error}<p class="error">{connection.error}</p>{/if}

  {#if connection.installed && !connection.processRunning}
    <button class="panel-secondary" disabled={!native || spotify.busy || connection.starting} onclick={start}>{connection.starting ? 'Starting Spotify…' : 'Start Spotify in background'}</button>
  {/if}

  <div class="spotify-setup">
    <h4>Account connection</h4>
    <ol>
      <li>Create an app in the Spotify Developer Dashboard.</li>
      <li>Add the redirect URI <code>http://127.0.0.1:8888/callback</code> exactly. Spotify does not allow <code>localhost</code>.</li>
      <li>Paste the app Client ID below and save it.</li>
      <li>Press Connect. Luma opens your browser for Spotify sign-in and catches the loopback callback locally.</li>
    </ol>
    <p class="panel-description">Development Mode apps require the app owner to have Spotify Premium, are limited to five authorized users, and support a smaller set of Web API endpoints. Playback control through the Web API requires Spotify Premium. Some library endpoints may be unavailable for playlists you do not own.</p>
  </div>

  <label class="field">Spotify Client ID<input bind:value={clientId} placeholder="Paste the Client ID" spellcheck="false" autocomplete="off" /></label>
  <div class="spotify-actions">
    <button class="panel-secondary" disabled={!native || saving || !clientId.trim()} onclick={save}>{saving ? 'Saving…' : 'Save Client ID'}</button>
    <button class="panel-secondary" disabled={!native || spotify.connecting || !connection.clientIdConfigured} onclick={connect}>{spotify.connecting ? 'Waiting for browser…' : connection.authenticated ? 'Reconnect Spotify' : 'Connect Spotify'}</button>
    {#if connection.authenticated}<button class="text-link" disabled={!native} onclick={disconnect}>Disconnect account</button>{/if}
  </div>
  {#if message}<p class="panel-description" role="status">{message}</p>{/if}
  {#if spotify.error}<p class="error" role="status">{spotify.error}</p>{/if}
</section>
