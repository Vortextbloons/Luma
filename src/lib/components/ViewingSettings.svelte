<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { native as desktop } from '../services/tauri';
  let { native = desktop }: { native?: boolean } = $props();
  type Mode = 'single' | 'split' | 'displays' | 'floating';
  interface ViewingState { mode: Mode; selected: string | null; players: string[]; }
  let viewing = $state<ViewingState>({ mode: 'single', selected: null, players: [] });
  let loading = $state(true);
  let busy = $state(false);
  let error = $state('');
  let message = $state('');
  let disposed = false;
  const modes: { id: Mode; name: string; description: string }[] = [
    { id: 'single', name: 'Single player', description: 'Watch one player fullscreen.' },
    { id: 'split', name: 'Side by side', description: 'Arrange two players on the current display.' },
    { id: 'displays', name: 'Separate displays', description: 'Put each player on a different connected display.' },
    { id: 'floating', name: 'Floating video', description: 'Keep the selected video above Luma while browsing. Move and resize its window.' },
  ];
  function name(label: string) { return ({ youtube: 'YouTube', stremio: 'Stremio', sports: 'Sports' } as Record<string, string>)[label] ?? label; }
  async function refresh() {
    if (!native || disposed) { loading = false; return; }
    try {
      const next = await invoke<ViewingState>('viewing_state');
      if (!disposed) { viewing = next; error = ''; }
    } catch (cause) { if (!disposed) error = String(cause); }
    finally { if (!disposed) loading = false; }
  }
  async function action(command: string, args: Record<string, unknown>, feedback: string) {
    if (busy || !native) return;
    busy = true;
    error = '';
    message = '';
    try {
      await invoke(command, args);
      message = feedback;
      await refresh();
    } catch (cause) { error = String(cause); }
    finally { busy = false; }
  }
  onMount(() => {
    void refresh();
    let unlisten: (() => void) | undefined;
    if (native) void listen('viewing-changed', () => void refresh()).then(stop => { if (disposed) stop(); else unlisten = stop; }).catch(cause => { if (!disposed) error = String(cause); });
    return () => { disposed = true; unlisten?.(); };
  });
</script>

<svelte:window onfocus={() => void refresh()} />

<section class="settings-group">
  <h3>Multi Watch</h3>
  <p class="panel-description">Choose how to watch, then open a second source. Supports up to two players from YouTube, Stremio, and Sports, with one player per source. Select a player to direct playback and remote controls to it.</p>
  {#if !native}<p class="panel-description">Available in the desktop application.</p>{/if}
  <div class="viewing-modes" role="group" aria-label="Viewing layout">
    {#each modes as mode (mode.id)}
      <button class={['viewing-mode', { selected: viewing.mode === mode.id }]} aria-pressed={viewing.mode === mode.id} disabled={!native || busy || loading} onclick={() => void action('viewing_configure', { mode: mode.id, selected: viewing.selected }, `${mode.name} selected.`)}>
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
        </div>
      {/each}
    </div>
  {:else}<p class="panel-description">No players open. Open a source below, or choose something from Watch or Sports.</p>{/if}
  <div class="settings-actions">
    <button class="panel-secondary" disabled={!native || busy || loading} onclick={() => void action('open_youtube', { fullscreen: true }, 'YouTube opened.')}>Open YouTube</button>
    <button class="panel-secondary" disabled={!native || busy || loading} onclick={() => void action('stremio_open', {}, 'Stremio opened.')}>Open Stremio</button>
  </div>
  {#if error}<p class="panel-description" role="alert">{error}</p><button class="panel-secondary" disabled={busy} onclick={() => void refresh()}>Refresh players</button>{/if}
  {#if message}<p class="panel-description" role="status">{message}</p>{/if}
</section>
<section class="settings-group">
  <h3>Switch display</h3>
  <p class="panel-description">Press Ctrl+Alt+M anywhere on Windows, or use the button to move Luma and its current viewing layout to the next connected monitor. Your saved TV display remains the startup default.</p>
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
  .viewing-player { display: flex; gap: 10px; flex-wrap: wrap; }
  @media (max-width: 720px) { .viewing-modes { grid-template-columns: 1fr; } }
</style>

