<script lang="ts">
  import MusicIcon from './MusicIcon.svelte';
  import { onMount } from 'svelte';
  import { spotify } from '../../state/spotify.svelte';
  import ProviderIcon from '../activity/ProviderIcon.svelte';
  import { openSetupLink } from '../../services/tauri';

  let { native }: { native: boolean } = $props();
  let clientId = $derived(spotify.clientId);
  let saving = $state(false);
  let message = $state('');

  const connection = $derived(spotify.connection);
  const stateLabel = $derived(!native ? 'Desktop only' : connection.authenticated ? 'Account connected' : connection.installed ? 'Ready' : 'Not found');
  const stateTone = $derived(!native ? 'is-muted' : connection.authenticated ? 'is-good' : connection.installed ? 'is-muted' : 'is-warn');

  onMount(() => { void spotify.refresh(); });

  async function save() {
    saving = true;
    try {
      if (await spotify.saveClientId(clientId)) message = 'Spotify Client ID saved. You can connect now.';
    } finally {
      saving = false;
    }
  }

  async function connect() {
    message = '';
    if (await spotify.connect()) message = 'Spotify account connected. Luma will keep you signed in until you sign out.';
  }

  async function disconnect() {
    if (await spotify.disconnect()) message = 'Spotify account signed out. Choose Connect Spotify to use another account.';
  }

  async function start() {
    await spotify.start();
  }

  async function openGuide() {
    try { await openSetupLink('spotify'); }
    catch (cause) { message = `Could not open your browser: ${String(cause)}`; }
  }
</script>

<section class="settings-group">
  <div class="settings-service-head">
    <ProviderIcon provider="spotify" />
    <div class="settings-service-title"><h3>Spotify</h3><p>Music controls for the Spotify app on this PC.</p></div>
    <span class={`settings-badge ${stateTone}`}>{stateLabel}</span>
  </div>

  <div class="settings-status-panel">
    <p class="settings-status-line"><span class={`settings-status-dot ${connection.processRunning ? 'is-good' : ''}`}></span>{native ? connection.processRunning ? 'Spotify is running.' : connection.starting ? 'Spotify is starting…' : connection.installed ? 'Spotify is installed but closed.' : 'Spotify is not installed on this PC.' : 'Available in the desktop application.'}</p>
    <p class="settings-status-line"><span class={`settings-status-dot ${connection.authenticated ? 'is-good' : ''}`}></span>{connection.authenticated ? `Account connected${connection.displayName ? ` as ${connection.displayName}` : ''}.` : 'Connecting a Spotify account is optional.'}</p>
  </div>

  {#if connection.installed && !connection.processRunning}
    <button class="panel-secondary is-primary" disabled={!native || spotify.busy || connection.starting} onclick={start}>{connection.starting ? 'Starting Spotify…' : 'Start Spotify'}</button>
  {/if}

  {#if connection.error}<p class="error">{connection.error}</p>{/if}

  <details class="spotify-advanced">
    <summary class="settings-advanced-summary" tabindex="0">
      <span class="settings-advanced-copy"><strong>Connect a Spotify account</strong><small>Optional. Adds playlists, search, saved music, and devices.</small></span>
      <span class="settings-advanced-mark" aria-hidden="true"><MusicIcon name="chevron-right" /></span>
    </summary>
    <div class="settings-advanced-body">
      <p class="panel-description">Link a Spotify developer app to browse your music and playlists in Luma.</p>
      <ol class="spotify-steps">
        <li><button class="text-link" disabled={!native} onclick={openGuide}>Open Spotify Developer Dashboard</button>, sign in, and choose Create app.</li>
        <li>Give the app a name, such as Luma, and select Web API.</li>
        <li>Add the redirect URI <code>http://127.0.0.1:8888/callback</code> exactly. Spotify does not allow <code>localhost</code>.</li>
        <li>Save the app in Spotify, open its Settings, and copy the Client ID. Paste it below and choose Save Client ID. You do not need the Client Secret.</li>
        <li>Choose Connect Spotify. Luma opens your browser for sign-in and finishes the connection locally.</li>
      </ol>
      <p class="panel-description">Spotify Premium is required for developer app owners and account playback controls. Development Mode allows up to five authorized users.</p>

      <label class="field">Spotify Client ID<input bind:value={clientId} placeholder="Paste the Client ID" spellcheck="false" autocomplete="off" /></label>
      <div class="settings-actions">
        <button class="panel-secondary" disabled={!native || saving || !clientId.trim()} onclick={save}>{saving ? 'Saving…' : 'Save Client ID'}</button>
        {#if !connection.authenticated}<button class="panel-secondary is-primary" disabled={!native || spotify.connecting || !connection.clientIdConfigured} onclick={connect}>{spotify.connecting ? 'Waiting for browser…' : 'Connect Spotify'}</button>{/if}
        {#if connection.authenticated}<button class="text-link" disabled={!native} onclick={disconnect}>Sign out</button>{/if}
      </div>
      {#if message}<p class="panel-description" role="status">{message}</p>{/if}
      {#if spotify.error}<p class="error" role="status">{spotify.error}</p>{/if}
    </div>
  </details>
</section>

<style>
  .spotify-advanced { margin-top: 18px; border: 1px solid #ffffff10; border-radius: 14px; background: var(--bg); overflow: hidden; }
  .spotify-advanced .settings-advanced-summary { min-height: 0; padding: 16px 18px; }
  .spotify-advanced[open] .settings-advanced-mark { transform: rotate(90deg); }
  .spotify-advanced .settings-advanced-body { padding: 0 18px 18px; border-top: 0; }
  .spotify-steps { display: grid; gap: 8px; margin: 0 0 16px; padding-left: 20px; color: var(--muted); font-size: clamp(13px, .88vw, 16px); line-height: 1.6; }
  .spotify-steps code { color: var(--text); overflow-wrap: anywhere; }
</style>
