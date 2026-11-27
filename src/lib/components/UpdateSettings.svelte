<script lang="ts">
  import { onMount } from 'svelte';
  import { version as buildVersion } from '../../../package.json';
  import { updates, initializeUpdates, checkUpdates } from '../state/updates.svelte';

  let { native, onnotes }: { native: boolean; onnotes: (kind: 'installed' | 'update') => void } = $props();
  const hasUpdate = $derived(['available', 'downloading', 'ready', 'installing'].includes(updates.phase));
  const checking = $derived(updates.phase === 'checking');
  onMount(() => { void initializeUpdates(); });
</script>

<section class="settings-group">
  <div class="settings-row">
    <div class="settings-row-copy">
      <strong>Luma {updates.version || buildVersion}</strong>
      <span>Installed version</span>
    </div>
    <button class="panel-secondary" onclick={() => onnotes('installed')}>View changelog</button>
  </div>
</section>

<section class="settings-group">
  <h3>Software updates</h3>
  <div role="status" aria-live="polite">
    {#if !native}<p class="panel-description">Updates are available in the desktop app.</p>
    {:else if updates.phase === 'unconfigured'}<p class="panel-description">Updates will be available in published releases.</p>
    {:else if checking}<p class="panel-description">Checking for updates…</p>
    {:else if updates.phase === 'current'}<p class="panel-description">You’re up to date.</p>
    {:else if updates.phase === 'downloading'}<p class="panel-description">Downloading Luma {updates.nextVersion}…</p>
    {:else if updates.phase === 'ready'}<p class="panel-description">Luma {updates.nextVersion} is ready to install.</p>
    {:else if updates.phase === 'installing'}<p class="panel-description">Restarting to install your update…</p>
    {:else if hasUpdate}<p class="panel-description">Luma {updates.nextVersion} is available.</p>
    {/if}
  </div>
  {#if updates.error}<p class="error" role="alert">{updates.error}</p>{/if}
  {#if native && updates.phase !== 'unconfigured'}
    <div class="settings-actions">
      <button class="panel-secondary is-primary" aria-disabled={checking} onclick={() => {
        if (hasUpdate) onnotes('update');
        else if (!checking) void checkUpdates();
      }}>{hasUpdate ? 'View update' : checking ? 'Checking…' : 'Check for updates'}</button>
    </div>
  {/if}
</section>
