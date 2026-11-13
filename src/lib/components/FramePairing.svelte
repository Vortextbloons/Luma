<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  let { native }: { native: boolean } = $props();
  type Status = { enabled: boolean; paired: boolean; addresses: string[]; pcName: string; code: string | null };
  let status = $state<Status | null>(null);
  let busy = $state(false);
  let error = $state('');
  async function run(command: string, args?: Record<string, unknown>) {
    busy = true;
    error = '';
    try { status = await invoke<Status>(command, args); }
    catch (reason) { error = String(reason); }
    finally { busy = false; }
  }
  onMount(() => {
    if (native) void run('frame_status');
    let checking = false;
    const timer = setInterval(async () => {
      if (!native || busy || checking) return;
      checking = true;
      try { status = await invoke<Status>('frame_status'); }
      catch (reason) { error = String(reason); }
      finally { checking = false; }
    }, 3000);
    return () => clearInterval(timer);
  });
</script>

<section class="settings-group">
  <h3>Display Box</h3>
  <p class="panel-description">Use your frame as a remote for Luma. Spotify audio plays on this PC. Pairing and this setting survive restarts.</p>
  {#if error}<p role="alert" class="error">{error}</p>{/if}
  {#if status}
    <p class="remote-status">{status.enabled ? status.paired ? 'Enabled · Frame paired' : 'Enabled · Ready to pair' : 'Disabled'}</p>
    <p class="panel-description">PC name: {status.pcName}. Spotify Connect must use this name for Music controls.</p>
    {#if status.enabled}
      <p class="panel-description">On the frame, open Settings → Luma mode. Enter this PC address:</p>
      {#each status.addresses as address (address)}<p class="address">{address}</p>{/each}
      <button class="panel-secondary" disabled={busy} onclick={() => run('frame_pairing')}>{status.code ? 'Create new code' : 'Pair a frame'}</button>
      {#if status.code}<p class="pair-code" aria-label="Pairing code">{status.code}</p><p class="panel-description">Valid for five minutes. Pairing a frame replaces the previous pairing.</p>{/if}
    {/if}
    <button class="text-link" disabled={busy} onclick={() => run('configure_frame', { enabled: !status?.enabled })}>{status.enabled ? 'Disable Display Box access' : 'Enable Display Box access'}</button>
    <p class="panel-description">Use a trusted private Wi-Fi network. Local remote traffic uses HTTP on port 9877. Allow Luma through Windows Firewall on private networks if the frame cannot connect.</p>
  {:else if !native}<p class="panel-description">Available in the desktop application.</p>{/if}
</section>

<style>
  .pair-code { font-family: monospace; font-size: 2.5rem; font-weight: 600; letter-spacing: .22em; margin: 1rem 0; color: var(--accent, #a59bff); }
  .address { font-family: monospace; font-size: 1.25rem; margin: .75rem 0; }
</style>
