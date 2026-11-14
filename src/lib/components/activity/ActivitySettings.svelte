<script lang="ts">
  import type { ProviderStatus } from '../../types';
  import type { ActivityState } from '../../state/activity.svelte';
  import ProviderIcon from './ProviderIcon.svelte';

  let { activity, native }: { activity: ActivityState; native: boolean } = $props();

  type Form = { serverUrl: string; userId: string; clientId: string; token: string };
  const blank = (): Form => ({ serverUrl: '', userId: '', clientId: '', token: '' });
  let forms = $state<Record<string, Form>>({ plex: blank(), jellyfin: blank(), trakt: blank() });
  let busy = $state('');
  let error = $state('');
  let note = $state('');
  let confirming = $state(false);

  const stateLabel: Record<ProviderStatus['state'], string> = {
    available: 'Ready', needsSetup: 'Setup needed', notValidated: 'Not validated',
    connected: 'Connected', disabled: 'Disabled', error: 'Error',
  };
  const capabilityName: Record<string, string> = {
    appLaunches: 'App launches', playbackTracking: 'Playback tracking', progress: 'Verified progress',
    currentMediaSession: 'Current media session', onDeck: 'On-deck', resume: 'Resume items',
    contentLinks: 'Content links', watchHistory: 'Watch history',
  };
  const fields: Record<string, { url?: boolean; user?: boolean; client?: boolean; token: string }> = {
    plex: { url: true, token: 'Plex token' },
    jellyfin: { url: true, user: true, token: 'API key' },
    trakt: { client: true, token: 'Access token' },
  };

  async function run(key: string, action: () => Promise<unknown>) {
    if (busy) return;
    busy = key; error = ''; note = '';
    try { await action(); }
    catch (cause) { error = String(cause); }
    finally { busy = ''; }
  }

  async function refresh(provider: ProviderStatus) {
    await run(provider.id, async () => {
      const result = await activity.refresh(provider.id);
      if (result.error) error = result.error;
      else note = result.updated > 0 ? `${provider.name}: ${result.updated} item${result.updated === 1 ? '' : 's'} updated.` : `${provider.name} is connected. No new items.`;
    });
  }

  async function save(provider: ProviderStatus) {
    const form = forms[provider.id];
    await run(provider.id, async () => {
      await activity.saveConfig(provider.id, {
        serverUrl: form.serverUrl.trim() || undefined,
        userId: form.userId.trim() || undefined,
        clientId: form.clientId.trim() || undefined,
        token: form.token.trim() || undefined,
      });
      form.token = '';
      note = `${provider.name} credentials saved to Windows Credential Manager. Run Refresh to validate.`;
    });
  }

  async function clearProvider(provider: ProviderStatus) {
    await run(provider.id, async () => {
      await activity.clear(provider.id);
      note = `${provider.name} history cleared from this PC.`;
    });
  }

  async function clearAll() {
    if (!confirming) { confirming = true; note = 'Select Clear all history again to confirm.'; return; }
    await run('all', async () => {
      await activity.clear();
      confirming = false;
      note = 'All local activity history was cleared.';
    });
  }
</script>

<section class="settings-group">
  <h3>Local activity</h3>
  <p class="panel-description">History lives in a local SQLite database on this PC. No Luma account, cloud sync, or viewing data leaves the device. Provider tokens are stored in Windows Credential Manager, never in the activity database.</p>
  {#if !native}
    <p class="panel-description">Activity tracking is available in the desktop application.</p>
  {:else}
    <label class="setting-toggle"><span>Record activity on this PC</span><input type="checkbox" checked={activity.trackingEnabled} disabled={busy !== ''} onchange={event => { const enabled = event.currentTarget.checked; run('tracking', () => activity.setTracking(enabled)); }} /></label>
    <div class="activity-danger">
      <button class="text-link" disabled={busy !== ''} onclick={clearAll}>{confirming ? 'Clear all history now' : 'Clear all activity history'}</button>
      {#if confirming}<button class="text-link" onclick={() => { confirming = false; note = ''; }}>Cancel</button>{/if}
    </div>
    {#if error}<p class="error" role="alert">{error}</p>{/if}
    {#if note}<p class="panel-description" role="status">{note}</p>{/if}
  {/if}
</section>

{#if native}
<section class="settings-group">
  <h3>Providers</h3>
  <p class="panel-description">Each integration is independent. A provider that is offline or unconfigured never blocks the home screen. “Not validated” means credentials are saved but Luma has not completed a successful refresh.</p>
  <div class="provider-list">
    {#each activity.providers as provider (provider.id)}
      <article class="provider-row">
        <div class="provider-head">
          <ProviderIcon provider={provider.id} />
          <div class="provider-title"><strong>{provider.name}</strong><span>{provider.detail}</span></div>
          <span class={`provider-state state-${provider.state}`}>{stateLabel[provider.state]}</span>
        </div>
        <div class="provider-flags">
          {#if provider.capabilities.length > 0}<span class="provider-caps">{provider.capabilities.map(id => capabilityName[id] ?? id).join(' · ')}</span>{/if}
          <label class="setting-toggle compact"><span>Enabled</span><input type="checkbox" checked={provider.enabled} disabled={busy !== ''} onchange={event => { const enabled = event.currentTarget.checked; run(provider.id, () => activity.setEnabled(provider.id, enabled)); }} /></label>
          <label class="setting-toggle compact"><span>Hide on Home</span><input type="checkbox" checked={provider.hidden} disabled={busy !== ''} onchange={event => { const hidden = event.currentTarget.checked; run(provider.id, () => activity.setHidden(provider.id, hidden)); }} /></label>
        </div>
        {#if provider.requiresConfig}
          <div class="provider-config">
            {#if fields[provider.id]}
              {#if fields[provider.id].url}<label class="field">{provider.id === 'jellyfin' ? 'Jellyfin server URL' : 'Plex server URL'}<input bind:value={forms[provider.id].serverUrl} placeholder="http://192.168.1.20:8096" spellcheck="false" /></label>{/if}
              {#if fields[provider.id].user}<label class="field">Jellyfin user ID (optional if there is one user)<input bind:value={forms[provider.id].userId} placeholder="User ID from Jellyfin" spellcheck="false" /></label>{/if}
              {#if fields[provider.id].client}<label class="field">Trakt client ID<input bind:value={forms[provider.id].clientId} placeholder="Trakt API client ID" spellcheck="false" /></label>{/if}
              <label class="field">{fields[provider.id].token}<input type="password" bind:value={forms[provider.id].token} placeholder="Stored in Windows Credential Manager" autocomplete="off" spellcheck="false" /></label>
            {/if}
            <div class="provider-actions">
              <button class="panel-secondary" disabled={busy !== ''} onclick={() => save(provider)}>Save credentials</button>
              {#if provider.configured}<button class="text-link" disabled={busy !== ''} onclick={() => refresh(provider)}>Refresh now</button><button class="text-link" disabled={busy !== ''} onclick={() => run(provider.id, () => activity.disconnect(provider.id))}>Disconnect</button>{/if}
              <button class="text-link" disabled={busy !== ''} onclick={() => clearProvider(provider)}>Clear history</button>
            </div>
          </div>
        {:else}
          <div class="provider-actions"><button class="text-link" disabled={busy !== ''} onclick={() => clearProvider(provider)}>Clear history</button></div>
        {/if}
      </article>
    {/each}
  </div>
</section>
{/if}

<style>
  .activity-danger { display: flex; gap: 16px; align-items: center; margin-top: 8px; }
  .provider-list { display: grid; gap: 14px; }
  .provider-row { border: 1px solid #ffffff16; background: #ffffff05; border-radius: 14px; padding: 18px; display: grid; gap: 14px; }
  .provider-head { display: grid; grid-template-columns: auto minmax(0, 1fr) auto; gap: 13px; align-items: center; }
  .provider-title { display: grid; gap: 3px; min-width: 0; }
  .provider-title strong { font-size: 15px; }
  .provider-title span { color: var(--muted); font-size: 12px; line-height: 1.5; }
  .provider-state { border: 1px solid #ffffff25; border-radius: 99px; padding: 5px 10px; font-size: 10px; font-weight: 700; text-transform: uppercase; letter-spacing: .06em; white-space: nowrap; }
  .state-connected, .state-available { color: #8fe3bd; border-color: #8fe3bd66; }
  .state-notValidated, .state-needsSetup { color: #ffd28d; border-color: #ffd28d66; }
  .state-error { color: #ff9e9e; border-color: #ff9e9e66; }
  .state-disabled { color: #999aa5; }
  .provider-flags { display: flex; flex-wrap: wrap; gap: 8px 20px; align-items: center; }
  .provider-caps { color: #8d8f9c; font-size: 11px; margin-right: auto; }
  .setting-toggle.compact { padding: 0; gap: 8px; font-size: 12px; }
  .setting-toggle.compact input { width: 17px; height: 17px; }
  .provider-config { display: grid; gap: 4px; }
  .provider-config .field { margin-top: 8px; }
  .provider-actions { display: flex; flex-wrap: wrap; gap: 8px 18px; align-items: center; margin-top: 6px; }
</style>
