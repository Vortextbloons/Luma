<script lang="ts">
  import { onMount } from 'svelte';
  import { spotify } from '../../state/spotify.svelte';
  import MusicIcon from './MusicIcon.svelte';
  let switching = $state('');
  let message = $state('');
  onMount(() => { void spotify.loadDevices(); });
  async function transfer(id?: string) {
    if (!id || switching) return;
    switching = id; message = '';
    if (await spotify.transfer(id)) { message = 'Playback device changed.'; await spotify.loadDevices(); }
    else message = spotify.error || 'Could not change the playback device. Try again.';
    switching = '';
  }
</script>
<section class="devices" aria-label="Playback devices">
  <header><div><h2>Listen on</h2><p>Choose where your music plays.</p></div><button disabled={spotify.devicesLoading || !!switching} onclick={() => spotify.loadDevices()}>{spotify.devicesLoading ? 'Refreshing...' : 'Refresh devices'}</button></header>
  {#if spotify.devicesError}<p class="error" role="status">{spotify.devicesError}</p>{/if}
  {#if message}<p role="status">{message}</p>{/if}
  {#if !spotify.devices.length}<p>{spotify.devicesLoading ? 'Finding Spotify devices...' : 'No devices found. Open Spotify on a device, then refresh.'}</p>{/if}
  <div class="list">{#each spotify.devices as device (`${device.id}-${device.name}`)}
    <button class={{ "active": device.active }} disabled={!device.id || device.active || device.restricted || spotify.busy || !!switching} onclick={() => transfer(device.id)}>
      <MusicIcon name="speaker" size={32} /><span><b>{device.name}</b><small>{device.kind} - {device.active ? 'Current device' : device.restricted ? 'Control unavailable' : switching === device.id ? 'Connecting...' : 'Use this device'}</small></span>
    </button>
  {/each}</div>
</section>
<style>
  .devices { padding: 28px; border: 1px solid #ffffff1a; border-radius: 20px; background: var(--surface); }
  header { display: flex; align-items: center; justify-content: space-between; gap: 20px; margin-bottom: 20px; }
  h2 { margin: 0; font-size: 30px; } p { font-size: 18px; color: var(--muted); line-height: 1.5; }
  header p { margin: 8px 0 0; } button { min-height: 56px; padding: 14px 20px; border-radius: 12px; background: #ffffff0e; font-size: 18px; }
  button:focus-visible { outline: 3px solid var(--accent); outline-offset: 4px; }
  button:disabled { opacity: .5; } button.active { opacity: 1; border: 1px solid var(--accent); }
  .list { display: grid; grid-template-columns: repeat(auto-fit,minmax(280px,1fr)); gap: 14px; }
  .list button { display: flex; align-items: center; gap: 20px; text-align: left; } span { display: grid; gap: 6px; min-width: 0; } b { overflow-wrap: anywhere; } small { font-size: 16px; color: var(--muted); }
</style>
