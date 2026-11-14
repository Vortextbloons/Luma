<script lang="ts">
  import { onMount } from 'svelte';
  import { updates, initializeUpdates, checkUpdates, downloadUpdate, installUpdate } from '../state/updates.svelte';
  let { native }: { native: boolean } = $props();
  const busy = $derived(['checking', 'downloading', 'installing'].includes(updates.phase));
  const progress = $derived(updates.total ? Math.min(100, Math.round(updates.downloaded / updates.total * 100)) : undefined);
  onMount(() => { void initializeUpdates(); });
</script>

<section class="settings-group">
  <h3>Luma {updates.version}</h3>
  <div role="status" aria-live="polite">
    {#if !native}<p class="panel-description">Updates are available in the desktop app.</p>
    {:else if updates.phase === 'unconfigured'}<p class="panel-description">Updates will be available in published releases.</p>
    {:else if updates.phase === 'checking'}<p class="panel-description">Checking for updates…</p>
    {:else if updates.phase === 'current'}<p class="panel-description">You’re up to date.</p>
    {:else if updates.phase === 'available'}<p class="panel-description">Luma {updates.nextVersion} is available. Download it now and restart when you’re ready.</p>
    {:else if updates.phase === 'downloading'}<p class="panel-description">Downloading Luma {updates.nextVersion}{progress === undefined ? '…' : ` — ${progress}%`}</p>
    {:else if updates.phase === 'ready'}<p class="panel-description">Your update is ready. Restart to install Luma {updates.nextVersion}.</p>
    {:else if updates.phase === 'installing'}<p class="panel-description">Restarting to install your update…</p>
    {:else}<p class="panel-description">Check for the latest version of Luma.</p>{/if}
  </div>
  {#if updates.phase === 'downloading'}<progress aria-label="Update download" value={progress} max="100"></progress>{/if}
  {#if updates.error}<p class="error" role="alert">{updates.error}</p>{/if}
  {#if native && updates.phase !== 'unconfigured'}
    <div class="settings-actions">
      <button class="panel-secondary is-primary" aria-disabled={busy} onclick={() => {
        if (busy) return;
        if (updates.phase === 'ready') void installUpdate();
        else if (updates.phase === 'available') void downloadUpdate();
        else void checkUpdates();
      }}>{updates.phase === 'ready' ? 'Restart to update' : updates.phase === 'installing' ? 'Restarting…' : updates.phase === 'downloading' ? 'Downloading…' : updates.phase === 'available' ? 'Update now' : updates.phase === 'checking' ? 'Checking…' : 'Check for updates'}</button>
    </div>
  {/if}
  {#if updates.notes}<h3>What’s new in {updates.nextVersion}</h3><p class="release-notes">{updates.notes}</p>{/if}
</section>

<style>
  progress { width: 100%; height: 14px; accent-color: var(--accent); margin-bottom: 20px; }
  .release-notes { white-space: pre-wrap; overflow-wrap: anywhere; line-height: 1.6; color: var(--text); }
  .settings-actions + h3 { margin-top: 28px; }
  button[aria-disabled='true'] { opacity: .6; cursor: wait; }
</style>
