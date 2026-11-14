<script lang="ts">
  import { updates, downloadUpdate, installUpdate, dismissUpdate } from '../state/updates.svelte';
  import ReleaseNotes from './ReleaseNotes.svelte';
  let { native }: { native: boolean } = $props();
  const active = $derived(native && (updates.phase === 'ready' || (['available', 'downloading'].includes(updates.phase) && updates.dismissedVersion !== updates.nextVersion)));
  const progress = $derived(updates.total ? Math.min(100, Math.round(updates.downloaded / updates.total * 100)) : undefined);
</script>

{#if active}
  <aside class="update-notice" aria-label="Luma update">
    <div class="update-summary">
      <div class="update-copy" role="status" aria-live="polite">
        {#if updates.phase === 'ready'}<b>Luma {updates.nextVersion} is ready to install.</b>
        {:else if updates.phase === 'downloading'}<b>Downloading Luma {updates.nextVersion}{progress === undefined ? '…' : ` — ${progress}%`}</b>
        {:else}<b>Luma {updates.nextVersion} is available.</b>{/if}
        {#if updates.error}<small class="update-error">{updates.error}</small>
        {:else if updates.phase === 'available'}<small>Download it now and restart when you’re ready.</small>
        {:else if updates.phase === 'ready'}<small>Restart Luma to finish installing.</small>{/if}
      </div>
      {#if updates.phase === 'downloading'}<progress aria-label="Update download" value={progress} max="100"></progress>{/if}
      <button class="update-action" aria-disabled={updates.phase === 'downloading'} onclick={() => {
        if (updates.phase === 'ready') void installUpdate();
        else if (updates.phase === 'available') void downloadUpdate();
      }}>{updates.phase === 'ready' ? 'Restart to update' : updates.phase === 'downloading' ? 'Downloading…' : 'Update now'}</button>
      {#if updates.phase === 'available'}<button class="update-dismiss" onclick={dismissUpdate}>Dismiss</button>{/if}
    </div>
    {#if updates.notes}
      <div class="update-notes">
        <h4>What’s new in Luma {updates.nextVersion}</h4>
        <ReleaseNotes notes={updates.notes} />
      </div>
    {/if}
  </aside>
{/if}

<style>
  .update-notice { display: flex; flex-direction: column; gap: 12px; padding: 14px 18px; background: #242633; border: 1px solid #ffffff30; border-radius: 12px; box-shadow: 0 8px 35px #0008; }
  .update-summary { display: flex; align-items: center; gap: 16px; }
  .update-notes { padding-top: 12px; border-top: 1px solid #ffffff1f; }
  .update-notes h4 { margin: 0 0 8px; font-size: 12px; font-weight: 600; color: #c9cedb; }
  .update-copy { display: flex; flex-direction: column; gap: 3px; min-width: 0; }
  .update-copy b { font-size: 14px; font-weight: 600; }
  .update-copy small { color: #a9aab5; font-size: 12px; line-height: 1.4; }
  .update-copy .update-error { color: #ffb4b4; }
  progress { flex: none; width: clamp(80px, 10vw, 140px); height: 8px; accent-color: var(--accent); }
  .update-action { flex: none; min-height: 40px; padding: 0 18px; border-radius: 9px; background: var(--accent); color: #151925; font-size: 13px; font-weight: 700; }
  .update-action[aria-disabled='true'] { opacity: .6; cursor: wait; }
  .update-dismiss { flex: none; min-height: 40px; padding: 0 8px; background: transparent; color: #c9cedb; font-size: 13px; font-weight: 600; }
</style>
