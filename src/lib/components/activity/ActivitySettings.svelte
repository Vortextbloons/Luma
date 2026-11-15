<script lang="ts">
  import type { ProviderStatus } from '../../types';
  import type { ActivityState } from '../../state/activity.svelte';
  import ProviderIcon from './ProviderIcon.svelte';
  import { openSetupLink } from '../../services/tauri';

  let { activity, native, view = 'all' }: { activity: ActivityState; native: boolean; view?: 'all' | 'accounts' | 'history' | 'clear' } = $props();

  type Form = { serverUrl: string; userId: string; clientId: string; token: string };
  const blank = (): Form => ({ serverUrl: '', userId: '', clientId: '', token: '' });
  let forms = $state<Record<string, Form>>({ plex: blank(), jellyfin: blank(), trakt: blank() });
  let busy = $state('');
  let error = $state('');
  let note = $state('');
  let confirming = $state(false);

  const stateLabel: Record<ProviderStatus['state'], string> = {
    available: 'Ready', needsSetup: 'Setup needed', notValidated: 'Needs check',
    connected: 'Connected', disabled: 'Off', error: 'Needs attention',
  };
  const capabilityName: Record<string, string> = {
    appLaunches: 'Opened apps', playbackTracking: 'Playback history', progress: 'Playback progress',
    currentMediaSession: 'Now playing', onDeck: 'Up next', resume: 'Resume',
    contentLinks: 'Links to titles', watchHistory: 'Watch history',
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
      else note = result.updated > 0 ? `${provider.name} is connected. ${result.updated} item${result.updated === 1 ? '' : 's'} updated.` : `${provider.name} is connected. Nothing new to show.`;
    });
  }

  async function save(provider: ProviderStatus) {
    const form = forms[provider.id];
    const values = {
      serverUrl: form.serverUrl.trim() || undefined,
      userId: form.userId.trim() || undefined,
      clientId: form.clientId.trim() || undefined,
      token: form.token.trim() || undefined,
    };
    if (!values.serverUrl && !values.userId && !values.clientId && !values.token) {
      note = 'Fill in the connection details before saving.';
      return;
    }
    await run(provider.id, async () => {
      await activity.saveConfig(provider.id, values);
      form.token = '';
      const result = await activity.refresh(provider.id);
      if (result.error) {
        error = result.error;
        note = `${provider.name} settings saved, but Luma could not connect yet.`;
      } else {
        note = result.updated > 0 ? `${provider.name} is connected. ${result.updated} item${result.updated === 1 ? '' : 's'} updated.` : `${provider.name} is connected.`;
      }
    });
  }

  async function clearProvider(provider: ProviderStatus) {
    await run(provider.id, async () => {
      await activity.clear(provider.id);
      note = `${provider.name} history was removed from Luma on this PC.`;
    });
  }

  async function clearAll() {
    if (!confirming) { confirming = true; note = 'Choose Clear watch history again to confirm.'; return; }
    await run('all', async () => {
      await activity.clear();
      confirming = false;
      note = 'Watch history was cleared from this PC. Your connected services and accounts were not changed.';
    });
  }
</script>

{#if view === 'all' || view === 'history'}
<section class="settings-group">
  <div class="settings-group-head">
    <h3>Watch history</h3>
    {#if native}<span class={activity.trackingEnabled ? 'settings-badge is-good' : 'settings-badge is-muted'}>{activity.trackingEnabled ? 'On' : 'Paused'}</span>{/if}
  </div>
  <p class="panel-description">Luma can remember what you watch and play so you can pick up where you left off. History stays on this PC and is never uploaded to a Luma account.</p>
  {#if !native}
    <p class="panel-description">Watch history is available in the desktop application.</p>
  {:else}
    <label class="setting-toggle">
      <span class="setting-toggle-copy"><span>Remember what I watch and play</span><small>Adds titles you open and playback progress to history on this PC.</small></span>
      <input type="checkbox" checked={activity.trackingEnabled} disabled={busy !== ''} onchange={event => { const enabled = event.currentTarget.checked; run('tracking', () => activity.setTracking(enabled)); }} />
    </label>
  {/if}
</section>
{/if}

{#if view === 'history'}
<section class="settings-group">
  <h3>Providers on Home</h3>
  <p class="panel-description">Choose which connected services can add rows and cards to Home. Hidden services still keep their history and can be shown again at any time.</p>
  {#if !native}
    <p class="panel-description">Provider visibility is available in the desktop application.</p>
  {:else}
    <div class="provider-list">
      {#each activity.providers as provider (provider.id)}
        <article class="provider-row provider-row-compact">
          <div class="provider-head">
            <ProviderIcon provider={provider.id} />
            <div class="provider-title"><strong>{provider.name}</strong><span>{provider.detail}</span></div>
            <span class={`provider-state state-${provider.state}`}>{stateLabel[provider.state]}</span>
          </div>
          <label class="setting-toggle compact"><span>Show on Home</span><input type="checkbox" checked={!provider.hidden} disabled={busy !== ''} onchange={event => { const shown = event.currentTarget.checked; run(provider.id, () => activity.setHidden(provider.id, !shown)); }} /></label>
        </article>
      {/each}
    </div>
  {/if}
</section>
{/if}

{#if view === 'all' || view === 'clear'}
<section class="settings-group">
  <h3>Clear history</h3>
  <p class="panel-description">Removing history deletes it from Luma on this PC. It does not sign you out, disconnect a service, or change anything in your accounts.</p>
  {#if !native}
    <p class="panel-description">Clearing history is available in the desktop application.</p>
  {:else}
    <div class="activity-danger">
      <button class="text-link is-danger" disabled={busy !== ''} onclick={clearAll}>{confirming ? 'Clear watch history now' : 'Clear watch history'}</button>
      {#if confirming}<button class="text-link" onclick={() => { confirming = false; note = ''; }}>Cancel</button>{/if}
    </div>
    {#if view === 'clear'}
      <div class="provider-list clear-list">
        {#each activity.providers as provider (provider.id)}
          <article class="provider-row provider-row-compact">
            <div class="provider-head">
              <ProviderIcon provider={provider.id} />
              <div class="provider-title"><strong>{provider.name}</strong><span>{provider.detail}</span></div>
              <button class="text-link is-danger" disabled={busy !== ''} onclick={() => clearProvider(provider)}>Clear {provider.name} history</button>
            </div>
          </article>
        {/each}
      </div>
    {/if}
  {/if}
</section>
{/if}

{#if view === 'all' || view === 'accounts'}
{#if native}
<section class="settings-group">
  <h3>Connected services</h3>
  <p class="panel-description">Connect the services you use to see what you're watching in Luma. Each service is optional, and one that is offline will not stop the rest of Luma. Visibility is under History &amp; Privacy.</p>
  <div class="provider-list">
    {#each activity.providers as provider (provider.id)}
      <article class="provider-row">
        <div class="provider-head">
          <ProviderIcon provider={provider.id} />
          <div class="provider-title"><strong>{provider.name}</strong><span>{provider.detail}</span></div>
          <span class={`provider-state state-${provider.state}`}>{stateLabel[provider.state]}</span>
        </div>
        <div class="provider-flags">
          <div class="provider-toggles">
            <label class="setting-toggle compact"><span>Use this service</span><input type="checkbox" checked={provider.enabled} disabled={busy !== ''} onchange={event => { const enabled = event.currentTarget.checked; run(provider.id, () => activity.setEnabled(provider.id, enabled)); }} /></label>
            {#if view === 'all'}
              <label class="setting-toggle compact"><span>Show on Home</span><input type="checkbox" checked={!provider.hidden} disabled={busy !== ''} onchange={event => { const shown = event.currentTarget.checked; run(provider.id, () => activity.setHidden(provider.id, !shown)); }} /></label>
            {/if}
          </div>
        </div>
        <details class="provider-advanced" open={provider.requiresConfig && !provider.configured}>
          <summary tabindex="0">Advanced</summary>
          <div class="provider-advanced-body">
            {#if provider.capabilities.length > 0}<p class="provider-caps"><span>Can show</span>{provider.capabilities.map(id => capabilityName[id] ?? id).join(' · ')}</p>{/if}
            {#if provider.requiresConfig}
              <p class="provider-advanced-note">Enter the connection details from {provider.name}. Luma stores them securely on this PC.</p>
              {#if fields[provider.id]}
                <ol class="provider-steps">
                  {#if provider.id === 'plex'}
                    <li>Enter the address of your Plex Media Server, including its port, such as http://192.168.1.20:32400.</li>
                    <li>Follow the Plex guide to find your account token, then paste it below.</li>
                  {:else if provider.id === 'jellyfin'}
                    <li>Enter the address you use to open your Jellyfin server.</li>
                    <li>Ask your server administrator for an API key. Leave User ID blank if the server has one user.</li>
                  {:else if provider.id === 'trakt'}
                    <li>Follow Trakt's developer guide to create an application and authorize your account.</li>
                    <li>Paste the Client ID and your OAuth access token below. This advanced connection does not use your Trakt password.</li>
                  {/if}
                  <li>Choose Save and connect. Luma checks the details and loads your history.</li>
                </ol>
                <button class="text-link" disabled={busy !== ''} onclick={() => run(provider.id, () => openSetupLink(provider.id))}>Open {provider.name} setup guide</button>
              {/if}
              <div class="provider-config">
                {#if fields[provider.id]}
                  {#if fields[provider.id].url}<label class="field">{provider.id === 'jellyfin' ? 'Jellyfin server URL' : 'Plex server URL'}<input bind:value={forms[provider.id].serverUrl} placeholder="http://192.168.1.20:8096" spellcheck="false" /></label>{/if}
                  {#if fields[provider.id].user}<label class="field">Jellyfin user ID (optional if there is one user)<input bind:value={forms[provider.id].userId} placeholder="User ID from Jellyfin" spellcheck="false" /></label>{/if}
                  {#if fields[provider.id].client}<label class="field">Trakt client ID<input bind:value={forms[provider.id].clientId} placeholder="Trakt API client ID" spellcheck="false" /></label>{/if}
                  <label class="field">{fields[provider.id].token}<input type="password" bind:value={forms[provider.id].token} placeholder="Paste your token" autocomplete="off" spellcheck="false" /></label>
                {/if}
                <div class="provider-actions">
                  <button class="panel-secondary" disabled={busy !== ''} onclick={() => save(provider)}>Save and connect</button>
                  {#if provider.configured}<button class="text-link" disabled={busy !== ''} onclick={() => refresh(provider)}>Check connection</button><button class="text-link" disabled={busy !== ''} onclick={() => run(provider.id, () => activity.disconnect(provider.id))}>Disconnect</button>{/if}
                  {#if view === 'all'}<button class="text-link is-danger" disabled={busy !== ''} onclick={() => clearProvider(provider)}>Clear history</button>{/if}
                </div>
              </div>
            {:else}
              {#if view === 'all'}<div class="provider-actions"><button class="text-link is-danger" disabled={busy !== ''} onclick={() => clearProvider(provider)}>Clear history</button></div>{/if}
            {/if}
            {#if provider.lastError}<p class="provider-advanced-note">{provider.lastError}</p>{/if}
          </div>
        </details>
      </article>
    {/each}
  </div>
</section>
{:else}
<section class="settings-group">
  <h3>Connected services</h3>
  <p class="panel-description">Account connections are available in the desktop application.</p>
</section>
{/if}
{/if}

{#if error}<p class="error" role="alert">{error}</p>{/if}
{#if note}<p class="panel-description" role="status">{note}</p>{/if}

<style>
  .provider-steps { margin: 0; padding-left: 24px; color: var(--text-dim); font-size: clamp(14px, 1vw, 18px); line-height: 1.65; }
  .provider-steps li + li { margin-top: 8px; }
  .activity-danger { display: flex; flex-wrap: wrap; gap: 10px; align-items: center; margin-top: 18px; padding-top: 18px; border-top: 1px solid #ffffff0d; }
  .provider-list { display: grid; gap: 14px; }
  .provider-row { border: 1px solid #ffffff11; background: var(--surface-2); border-radius: 16px; padding: clamp(16px, 1.5vw, 24px); display: grid; gap: 14px; }
  .provider-row-compact { gap: 10px; padding: 14px 16px; }
  .provider-head { display: grid; grid-template-columns: auto minmax(0, 1fr) auto; gap: 14px; align-items: center; }
  .provider-title { display: grid; gap: 4px; min-width: 0; }
  .provider-title strong { font-size: clamp(15px, 1vw, 18px); overflow-wrap: anywhere; }
  .provider-title span { color: var(--muted); font-size: clamp(12px, .78vw, 14px); line-height: 1.5; }
  .provider-state { border: 1px solid #ffffff25; border-radius: 99px; padding: 5px 10px; font-size: clamp(10px, .66vw, 12px); font-weight: 650; white-space: nowrap; }
  .state-connected, .state-available { color: var(--good); border-color: #8fe3bd55; background: #8fe3bd0e; }
  .state-notValidated, .state-needsSetup { color: var(--warn); border-color: #ffd28d55; background: #ffd28d0c; }
  .state-error { color: var(--danger); border-color: #ff9e9e55; background: #ff9e9e0c; }
  .state-disabled { color: var(--muted); background: #ffffff05; }
  .provider-flags { display: grid; gap: 12px; }
  .provider-advanced { padding-top: 12px; border-top: 1px solid #ffffff0d; }
  .provider-advanced summary { width: fit-content; cursor: pointer; color: var(--text-dim); font-size: clamp(12px, .78vw, 14px); font-weight: 600; }
  .provider-advanced summary:hover { color: #fff; }
  .provider-advanced summary:focus-visible { outline: 3px solid var(--accent); outline-offset: 4px; }
  .provider-advanced-body { display: grid; gap: 12px; margin-top: 12px; }
  .provider-advanced-note { margin: 0; color: var(--muted); font-size: clamp(11px, .72vw, 13px); line-height: 1.5; }
  .provider-caps { display: flex; flex-wrap: wrap; gap: 4px 10px; margin: 0; color: var(--muted); font-size: clamp(11px, .72vw, 13px); line-height: 1.5; }
  .provider-caps span { color: var(--dim); font-weight: 600; }
  .provider-toggles { display: flex; flex-wrap: wrap; gap: 8px; }
  .setting-toggle.compact { width: auto; min-height: 44px; padding: 8px 13px; gap: 14px; border: 1px solid var(--line); border-radius: 10px; background: #ffffff05; font-size: clamp(12px, .76vw, 14px); }
  .provider-config { display: grid; gap: 0; padding: clamp(14px, 1.2vw, 20px); border: 1px solid #ffffff0a; border-radius: 12px; background: var(--bg); }
  .provider-config .field { margin-top: 14px; }
  .provider-actions { display: flex; flex-wrap: wrap; gap: 8px; align-items: center; margin-top: 16px; }
  .provider-actions .panel-secondary, .provider-actions .text-link { margin-top: 0; }
  .clear-list { margin-top: 18px; }
  .clear-list .provider-head { grid-template-columns: auto minmax(0, 1fr) auto; }
</style>
