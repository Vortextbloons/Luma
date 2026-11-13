<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  let { native }: { native: boolean } = $props();
  let enabled = $state(false);
  let busy = $state(true);
  let error = $state('');
  onMount(() => {
    if (!native) { busy = false; return; }
    invoke<boolean>('startup_enabled').then(value => enabled = value).catch(cause => error = String(cause)).finally(() => busy = false);
  });
  async function toggle() {
    busy = true;
    error = '';
    try { await invoke('set_startup', { enabled: !enabled }); enabled = !enabled; }
    catch (cause) { error = String(cause); }
    finally { busy = false; }
  }
</script>

<section class="settings-group">
  <h3>Background server</h3>
  <label class="setting-toggle"><span>Start server when Windows starts</span><input type="checkbox" checked={enabled} disabled={!native || busy} onchange={toggle} /></label>
  <p class="panel-description">Starts quietly in the system tray after sign-in. Closing Luma keeps the server running. Use the tray menu to open Luma or quit completely. Enable Display Box in this section to allow your photo frame to connect.</p>
  {#if error}<p role="status" class="error">{error}</p>{/if}
</section>
