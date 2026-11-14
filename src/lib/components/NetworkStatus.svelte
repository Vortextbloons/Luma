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
  <span
    class={['vpn-status', error ? 'is-error' : !status ? 'is-checking' : status.vpnDetected ? 'is-active' : 'is-clear']}
    role="status"
    title={error ? 'Windows network status is unavailable.' : status?.vpnDetected ? `${status.vpnNames.join(', ')}. Some apps may use a different route. For web remote access, allow local network access in your VPN settings.` : 'No active VPN adapter was detected. Some VPN apps may not be identifiable.'}
  >
    <span class="vpn-dot" aria-hidden="true"></span>
    <span class="vpn-label">{error ? 'VPN status unavailable' : !status ? 'Checking VPN…' : status.vpnDetected ? 'VPN detected' : 'No VPN detected'}</span>
  </span>
{/if}

<style>
  .vpn-status { display: inline-flex; align-items: center; flex-shrink: 0; }
  .vpn-dot { width: 9px; height: 9px; border-radius: 50%; background: #777a86; box-shadow: 0 0 0 1px #ffffff1a; }
  .vpn-status.is-clear .vpn-dot { background: #7fe0ae; box-shadow: 0 0 10px #7fe0ae66; }
  .vpn-status.is-active .vpn-dot { background: #ffd28d; box-shadow: 0 0 10px #ffd28d66; }
  .vpn-status.is-error .vpn-dot { background: #ff9e9e; }
  .vpn-label { position: absolute; width: 1px; height: 1px; padding: 0; margin: -1px; overflow: hidden; clip: rect(0 0 0 0); white-space: nowrap; border: 0; }
</style>
