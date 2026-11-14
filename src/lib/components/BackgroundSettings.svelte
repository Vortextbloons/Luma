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
  async function toggle(event: Event) {
    const input = event.currentTarget as HTMLInputElement;
    const next = input.checked;
    busy = true;
    error = '';
    try { await invoke('set_startup', { enabled: next }); enabled = next; }
    catch (cause) { error = String(cause); input.checked = enabled; }
    finally { busy = false; }
  }
</script>

<section class="settings-group">
  <div class="settings-group-head">
    <h3>Startup</h3>
    <span class={enabled ? 'settings-badge is-good' : 'settings-badge is-muted'}>{busy ? 'Checking' : enabled ? 'Starts at sign-in' : 'Starts manually'}</span>
  </div>
  <p class="panel-description">Closing the Luma window asks whether to keep running in the notification area, so the Display Box and web remote stay available, or quit completely. The remote’s Stop command only closes the window.</p>
  <label class="setting-toggle"><span class="setting-toggle-copy"><span>Start Luma when I sign in</span><small>Luma opens quietly in the notification area.</small></span><input type="checkbox" checked={enabled} disabled={!native || busy} onchange={toggle} /></label>
  {#if error}<p role="status" class="error">{error}</p>{/if}
</section>
