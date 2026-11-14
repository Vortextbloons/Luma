<script lang="ts">
  import { onMount } from 'svelte';
  import type { StremioStatus } from '../types';
  import { stremioDisconnect, stremioLinkCancel, stremioLinkPoll, stremioLinkStart, stremioOpenLink, stremioStatus, stremioSync } from '../services/tauri';

  let { native }: { native: boolean } = $props();
  let status = $state<StremioStatus | null>(null);
  let busy = $state(false);
  let error = $state('');
  let note = $state('');
  let timer: number | undefined;

  const waiting = $derived(status?.linking ?? null);

  function stopPolling() {
    if (timer !== undefined) { window.clearInterval(timer); timer = undefined; }
  }

  function startPolling() {
    if (timer !== undefined) return;
    timer = window.setInterval(() => void poll(), 3000);
  }

  function when(value: number | null) {
    return value ? new Date(value).toLocaleString([], { dateStyle: 'medium', timeStyle: 'short' }) : 'Never';
  }

  async function refresh() {
    if (!native) return;
    try {
      status = await stremioStatus();
      if (status.linking) startPolling();
    } catch (cause) { error = String(cause); }
  }

  async function connect() {
    if (busy) return;
    busy = true; error = ''; note = '';
    try {
      status = await stremioLinkStart();
      startPolling();
    } catch (cause) { error = String(cause); }
    finally { busy = false; }
  }

  async function poll() {
    if (busy || !status?.linking) return;
    busy = true;
    try {
      status = await stremioLinkPoll();
      error = '';
      if (status.connected) {
        stopPolling();
        note = 'Stremio connected and library synchronized.';
      }
    } catch (cause) {
      error = String(cause);
      stopPolling();
      try { status = await stremioStatus(); } catch { /* keep the last known state */ }
      if (status?.linking) startPolling();
    } finally { busy = false; }
  }

  async function cancel() {
    if (busy) return;
    busy = true; error = '';
    try {
      status = await stremioLinkCancel();
      stopPolling();
    } catch (cause) { error = String(cause); }
    finally { busy = false; }
  }

  async function openLink() {
    error = '';
    try { await stremioOpenLink(); } catch (cause) { error = String(cause); }
  }

  async function sync() {
    if (busy) return;
    busy = true; error = ''; note = '';
    try {
      status = await stremioSync();
      note = 'Stremio library synchronized.';
    } catch (cause) {
      error = String(cause);
      try { status = await stremioStatus(); } catch { /* keep the last known state */ }
    } finally { busy = false; }
  }

  async function disconnect() {
    if (busy) return;
    busy = true; error = ''; note = '';
    try {
      status = await stremioDisconnect();
      stopPolling();
      note = 'Stremio disconnected. Your account was not changed.';
    } catch (cause) { error = String(cause); }
    finally { busy = false; }
  }

  onMount(() => { void refresh(); return stopPolling; });
</script>

<section class="settings-group">
  <h3>Stremio</h3>
  {#if !native}
    <p class="panel-description">Stremio account connection is available in the desktop application.</p>
  {:else if !status}
    <p class="panel-description" role="status">Checking Stremio connection…</p>
  {:else if status.connected}
    <div class="stremio-account"><span class="stremio-dot" aria-hidden="true"></span><div><strong>{status.email || 'Stremio account'}</strong><p>Connected</p></div></div>
    <p class="panel-description">Last synchronized {when(status.lastSync)}. {status.items.length} titles in your library, {status.continueWatching.length} in progress. Playback progress comes from Stremio's synchronized library.</p>
    <button class="panel-secondary" disabled={busy} onclick={sync}>{busy ? 'Synchronizing…' : 'Sync Now'}</button>
    <button class="text-link" disabled={busy} onclick={disconnect}>Disconnect account</button>
    <p class="panel-description">Disconnecting removes only Luma's saved credential. Your Stremio account and its library are not modified.</p>
  {:else if waiting}
    <div class="stremio-link">
      <p class="panel-description">On your phone or another device, open the link below and sign in to Stremio, or scan the QR code. Luma will connect automatically. The code expires after five minutes.</p>
      <span class="stremio-code" aria-label={`Stremio link code ${waiting.code}`}>{waiting.code}</span>
      <img class="stremio-qr" src={waiting.qr} alt="QR code for the Stremio device link" />
      <p class="stremio-link-url">{waiting.link}</p>
      <button class="panel-secondary" disabled={busy} onclick={openLink}>Open link.stremio.com</button>
      <button class="text-link" disabled={busy} onclick={cancel}>Cancel</button>
      <p class="panel-description" role="status">Waiting for approval…</p>
    </div>
  {:else}
    <p class="panel-description">Connect your Stremio account to see Continue Watching and your library in Luma. Luma uses Stremio's device-link service, so your password is never requested or stored.</p>
    <button class="panel-secondary" disabled={busy} onclick={connect}>{busy ? 'Requesting code…' : 'Connect account'}</button>
  {/if}
  {#if error}<p class="stremio-error" role="alert">{error}</p>{/if}
  {#if note}<p class="stremio-note" role="status">{note}</p>{/if}
</section>
