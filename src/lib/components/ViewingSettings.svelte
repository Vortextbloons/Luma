<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { native as desktop } from '../services/tauri';
  let { native = desktop }: { native?: boolean } = $props();
  type Mode = 'single' | 'split' | 'displays' | 'floating';
  interface ViewingState { mode: Mode; selected: string | null; players: string[]; names: Record<string, string>; external: string[]; }
  interface Source { id: string; name: string; }
  let viewing = $state.raw<ViewingState>({ mode: 'single', selected: null, players: [], names: {}, external: [] });
  let sources = $state.raw<Source[]>([]);
  let windows = $state.raw<Source[]>([]);
  let sourceId = $state('');
  let windowId = $state('');
  let windowsLoading = $state(false);
  let sourcesLoading = $state(false);
  let windowError = $state('');
  let sourceError = $state('');
  let refreshRequest = 0;
  const availableWindows = $derived(windows.filter(window => !viewing.players.includes(window.id)));
  const full = $derived(viewing.mode !== 'single' && viewing.players.length >= 2);
  let loading = $state(true);
  let busy = $state(false);
  let error = $state('');
  let message = $state('');
  let disposed = false;
  let openingPlayers: string[] | null = null;
  const modes: { id: Mode; name: string; description: string }[] = [
    { id: 'single', name: 'Single player', description: 'Watch one player fullscreen.' },
    { id: 'split', name: 'Side by side', description: 'Arrange two players on the current display.' },
    { id: 'displays', name: 'Separate displays', description: 'Put each player on a different connected display.' },
    { id: 'floating', name: 'Floating video', description: 'Keep the selected video above Luma while browsing. Move and resize its window.' },
  ];
  function name(label: string) { return viewing.names[label] ?? ({ youtube: 'YouTube', stremio: 'Stremio', sports: 'Sports' } as Record<string, string>)[label] ?? label; }
  async function refresh() {
    if (!native || disposed) { loading = false; return; }
    const request = ++refreshRequest;
    try {
      const next = await invoke<ViewingState>('viewing_state');
      if (!disposed && request === refreshRequest) viewing = next;
    } catch (cause) { if (!disposed && request === refreshRequest) error = String(cause); }
    finally { if (!disposed && request === refreshRequest) loading = false; }
  }
  async function refreshWindows() {
    if (!native || disposed || windowsLoading) return;
    windowsLoading = true;
    windowError = '';
    try {
      const next = await invoke<Source[]>('viewing_windows');
      if (!disposed) {
        windows = next;
        if (!windowId && next.length) windowId = next[0].id;
      }
    } catch (cause) { if (!disposed) windowError = String(cause); }
    finally { if (!disposed) windowsLoading = false; }
  }
  async function refreshSources() {
    if (!native || disposed || sourcesLoading) return;
    sourcesLoading = true;
    sourceError = '';
    try {
      const next = await invoke<Source[]>('viewing_sources');
      if (!disposed) { sources = next; if (!sourceId && next.length) sourceId = next[0].id; }
    } catch (cause) { if (!disposed) sourceError = String(cause); }
    finally { if (!disposed) sourcesLoading = false; }
  }
  async function action(command: string, args: Record<string, unknown>, feedback: string) {
    if (busy || !native) return;
    busy = true;
    error = '';
    message = '';
    if (command === 'viewing_open_source') { openingPlayers = [...viewing.players]; message = feedback; }
    try {
      await invoke(command, args);
      if (!disposed && !error && command === 'viewing_open_source' && openingPlayers) {
        if (['NBA', 'NFL', 'MLB', 'NHL'].includes(String(args.id))) {
          openingPlayers = null;
          message = 'Choose a game in Sports to start playback.';
        } else {
          message = 'Source launch requested. Its window will appear here when ready.';
          if (viewing.mode === 'single') { openingPlayers = null; message = 'Source launch requested.'; }
        }
      }
      if (!disposed && !error && command !== 'viewing_open_source') message = feedback;
      await refresh();
    } catch (cause) { if (!disposed) { openingPlayers = null; message = ''; error = String(cause); } }
    finally { if (!disposed) busy = false; }
  }
  onMount(() => {
    void refresh();
    void refreshSources();
    void refreshWindows();
    const listeners: (() => void)[] = [];
    const keepListener = (stop: () => void) => { if (disposed) stop(); else listeners.push(stop); };
    const listenerError = (cause: unknown) => { if (!disposed) error = String(cause); };
    if (native) {
      void listen<ViewingState>('viewing-changed', event => {
        if (disposed) return;
        if (openingPlayers && event.payload.players.some(player => !openingPlayers!.includes(player))) {
          openingPlayers = null;
          message = 'Player added to Multi Watch.';
        }
        void refresh();
      }).then(keepListener).catch(listenerError);
      void listen<string>('viewing-error', event => {
        if (!disposed) { openingPlayers = null; message = ''; error = event.payload; }
      }).then(keepListener).catch(listenerError);
    }
    return () => { disposed = true; for (const stop of listeners) stop(); };
  });
</script>

<svelte:window onfocus={() => void refresh()} />

<section class="settings-group">
  <h3>Multi Watch</h3>
  <p class="panel-description">Open up to two players. Select a player to control it with your remote.</p>
  {#if !native}<p class="panel-description">Available in the desktop application.</p>{/if}
  <div class="viewing-modes" role="group" aria-label="Viewing layout">
    {#each modes as mode (mode.id)}
      <button class={['viewing-mode', { selected: viewing.mode === mode.id }]} aria-pressed={viewing.mode === mode.id} disabled={!native || busy || loading} onclick={() => void action('viewing_configure', { mode: mode.id, selected: null }, `${mode.name} selected.`)}>
        <strong>{mode.name}</strong><span>{mode.description}</span>
      </button>
    {/each}
  </div>
  {#if loading}<p class="panel-description" role="status">Loading players...</p>
  {:else if viewing.players.length}
    <div class="viewing-players" role="group" aria-label="Active players">
      {#each viewing.players as player (player)}
        <div class="viewing-player">
          <button class="panel-secondary" aria-pressed={viewing.selected === player} disabled={busy} onclick={() => void action('viewing_configure', { mode: viewing.mode, selected: player }, `${name(player)} selected.`)}>{name(player)}{viewing.selected === player ? ' - Selected' : ''}</button>
          <button class="panel-secondary" disabled={busy} aria-label={`Remove ${name(player)} player`} onclick={() => void action('viewing_remove', { label: player }, `${name(player)} removed.`)}>Remove</button>
          {#if viewing.external.includes(player)}<span class="viewing-player-note">Use this player's own audio and playback controls.</span>{/if}
        </div>
      {/each}
    </div>
  {:else}<p class="panel-description">No players open. Open a source below, or choose something from Watch or Sports.</p>{/if}
  {#if full}<p class="panel-description" role="status">Two players are open. Remove one before adding another.</p>{/if}
  <div class="viewing-source">
    <label for="viewing-source">Open a Luma source</label>
    <div class="viewing-picker">
      <select id="viewing-source" bind:value={sourceId} disabled={!native || busy || sourcesLoading || !sources.length}>
        <option value="" disabled>{sourcesLoading ? 'Loading sources...' : 'Choose a source'}</option>
        {#each sources as source (source.id)}<option value={source.id}>{source.name}</option>{/each}
      </select>
      <button class="panel-secondary" disabled={!native || busy || loading || sourcesLoading || !sourceId || full} onclick={() => void action('viewing_open_source', { id: sourceId }, `Opening ${sources.find(source => source.id === sourceId)?.name ?? 'source'}...`)}>Open source</button>
    </div>
    {#if sourceError}<p class="panel-description" role="alert">{sourceError}</p><button class="panel-secondary" disabled={busy || sourcesLoading} onclick={() => void refreshSources()}>Reload sources</button>{/if}
  </div>
  <div class="viewing-source">
    <label for="viewing-window">Add an open window</label>
    <p class="panel-description">Open a player on your PC, then refresh this list.</p>
    <div class="viewing-picker">
      <select id="viewing-window" bind:value={windowId} disabled={!native || busy || windowsLoading || !availableWindows.length}>
        <option value="" disabled>{windowsLoading ? 'Loading windows...' : 'Choose a window'}</option>
        {#each availableWindows as window (window.id)}<option value={window.id}>{window.name}</option>{/each}
      </select>
      <button class="panel-secondary" disabled={!native || busy || loading || windowsLoading || full || viewing.mode === 'single' || !availableWindows.some(window => window.id === windowId)} onclick={() => void action('viewing_add_window', { id: windowId }, 'Window added.')}>Add window</button>
      <button class="panel-secondary" disabled={!native || busy || windowsLoading} onclick={() => void refreshWindows()}>{windowsLoading ? 'Refreshing...' : 'Refresh windows'}</button>
    </div>
    {#if viewing.mode === 'single'}<p class="panel-description">Choose Side by side, Separate displays, or Floating video before adding a window.</p>{/if}
    {#if !windowsLoading && native && !availableWindows.length}<p class="panel-description">No other windows are available. Open a player and refresh the list.</p>{/if}
    {#if windowError}<p class="panel-description" role="alert">{windowError}</p>{/if}
    <p class="panel-description">Use the app’s own volume and playback controls.</p>
  </div>
  {#if error}<p class="panel-description" role="alert">{error}</p><button class="panel-secondary" disabled={busy} onclick={() => void refresh()}>Refresh players</button>{/if}
  {#if message}<p class="panel-description" role="status">{message}</p>{/if}
</section>
<section class="settings-group">
  <h3>Switch display</h3>
  <p class="panel-description">Move Luma to the next screen now. You can also press Ctrl+Alt+M.</p>
  <button class="panel-secondary" disabled={!native || busy} onclick={() => void action('switch_display', {}, 'Display switched.')}>Switch display</button>
</section>

<style>
  .viewing-modes { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 10px; margin: 16px 0; }
  .viewing-mode { display: grid; gap: 6px; padding: 16px; text-align: left; border: 1px solid var(--line); border-radius: 12px; background: var(--surface-2); color: var(--text); }
  .viewing-mode strong { font-size: 15px; }
  .viewing-mode span { color: var(--muted); font-size: 13px; line-height: 1.5; }
  .viewing-mode.selected { border-color: var(--accent); background: var(--accent-soft); }
  .viewing-mode:focus-visible { outline: 3px solid var(--accent); outline-offset: 3px; }
  .viewing-mode:disabled { opacity: .5; }
  .viewing-players { display: grid; gap: 10px; margin-bottom: 16px; }
  .viewing-player { display: flex; gap: 10px; flex-wrap: wrap; align-items: center; }
  .viewing-player-note { flex-basis: 100%; color: var(--muted); font-size: 13px; }
  .viewing-source { display: grid; gap: 10px; margin-top: 22px; }
  .viewing-source > label { color: var(--text); font-size: 15px; font-weight: 650; }
  .viewing-picker { display: flex; align-items: center; gap: 10px; flex-wrap: wrap; }
  .viewing-picker select { flex: 1; min-width: min(100%, 220px); max-width: 100%; padding: 12px; border: 1px solid var(--line); border-radius: 10px; background: var(--surface-2); color: var(--text); font-size: 14px; }
  .viewing-picker select:focus-visible { outline: 3px solid var(--accent); outline-offset: 3px; }
  @media (max-width: 720px) { .viewing-modes { grid-template-columns: 1fr; } }
</style>

