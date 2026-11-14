<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { native } from '../services/tauri';
  let status = $state<{ vpnDetected: boolean; vpnNames: string[] } | null>(null);
  let error = $state(false);
  onMount(() => {
    if (!native) return;
    let disposed = false;
    let generation = 0;
    let stop: (() => void) | undefined;
    async function refresh() {
      const request = ++generation;
      try {
        const result = await invoke<{ vpnDetected: boolean; vpnNames: string[] }>('network_status');
        if (!disposed && request === generation) { status = result; error = false; }
      } catch { if (!disposed && request === generation) error = true; }
    }
    void listen('network-changed', () => { void refresh(); }).then(unlisten => {
      if (disposed) unlisten();
      else { stop = unlisten; void refresh(); }
    }).catch(() => { if (!disposed) error = true; });
    return () => { disposed = true; stop?.(); };
  });
</script>

{#if native}
  <span class={['vpn-status', { connected: status?.vpnDetected && !error }]} role="status" title={error ? 'Windows network status is unavailable.' : status?.vpnDetected ? `${status.vpnNames.join(', ')}. Some apps may use a different route. For web remote access, allow local network access in your VPN settings.` : 'No active VPN adapter was detected. Some VPN apps may not be identifiable.'}>
    {error ? 'VPN status unavailable' : !status ? 'Checking VPN…' : status.vpnDetected ? 'VPN detected' : 'No VPN detected'}
  </span>
{/if}

<style>
  .vpn-status { font-size: 13px; color: var(--text); opacity: .75; white-space: nowrap; flex-shrink: 0; }
  .connected { color: var(--accent); opacity: 1; }
</style>
