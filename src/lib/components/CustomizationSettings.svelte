<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { defaultThemeId, themeById } from '../services/theme';
  import type { CustomizationConfig, CustomizationDocument } from '../services/customization';

  type HistoryEntry = { revision: number; savedAt: string | null; config: CustomizationConfig };
  type McpStatus = { enabled: boolean; endpoint: string };
  type McpGrant = McpStatus & { token: string };

  let { revision, native, section, onchanged }: { revision: number; native: boolean; section: string; onchanged?: () => void } = $props();

  const setupPrompt = `Connect to the Luma MCP server that the user has already added to your MCP client.

Luma is a Windows TV launcher. You can inspect its customization capabilities,
stage changes for the user to review, and read the current configuration.

Rules:
1. If the luma_* tools are not available, stop and tell the user to open
   Luma > Settings > Customization > AI access and add the server shown there.
   Do not guess connection details.
2. Start with luma_capabilities_get and luma_config_get. Do not invent
   components, properties, themes, screens, or extension permissions.
3. Prefer the smallest change that satisfies the request. Ask the user when the
   request is ambiguous, and never change app order, screens, or install an
   extension without saying so first.
4. Validate a complete candidate, then stage it. Tell the user that the change
   is waiting for review and that they must press Keep in Luma.
5. Never claim a change is applied, saved, or live until the user confirms they
   kept it, or until a subsequent read shows a new revision.
6. If validation fails, report the exact error. Do not retry an unchanged
   candidate.
7. Do not ask for, print, or store tokens, passwords, or account credentials.
   Luma never returns them through these tools.`;

  const defaultThemeName = themeById(defaultThemeId).name;
  const savedDateFormat = new Intl.DateTimeFormat(undefined, { dateStyle: 'medium', timeStyle: 'short' });

  let entries = $state.raw<HistoryEntry[]>([]);
  let historyLoading = $state(true);
  let historyError = $state('');
  let historyNote = $state('');
  let restoreTarget = $state<number | null>(null);
  let restoreBusy = $state(false);
  let restoreError = $state('');
  let resetConfirming = $state(false);
  let resetBusy = $state(false);
  let resetError = $state('');
  let resetNote = $state('');

  let mcp = $state.raw<McpStatus | null>(null);
  let mcpLoading = $state(true);
  let mcpBusy = $state(false);
  let mcpError = $state('');
  let mcpNote = $state('');
  let token = $state('');

  let root: HTMLElement | undefined;
  let disposed = false;
  let historySeq = 0;
  let loadedRevision = -1;

  const documentBusy = $derived(restoreBusy || resetBusy);
  const mcpTone = $derived(mcp?.enabled ? 'is-good' : 'is-muted');

  function messageOf(error: unknown) {
    return error && typeof error === 'object' && 'message' in error ? String(error.message) : String(error);
  }

  function savedLabel(savedAt: string | null) {
    if (!savedAt) return 'Saved version';
    const date = new Date(savedAt);
    return Number.isNaN(date.getTime()) ? 'Saved version' : savedDateFormat.format(date);
  }

  function describe(config: CustomizationConfig) {
    const theme = themeById(config.theme);
    const rows = config.home.filter(row => row.visible).length;
    return `${theme.name} · ${config.homeColumns === 2 ? 'two columns' : 'one column'} · ${rows} ${rows === 1 ? 'row' : 'rows'} · ${config.widgets.spotify.enabled ? 'Spotify widget on' : 'Spotify widget off'}`;
  }

  function focusActiveSectionTab() {
    requestAnimationFrame(() => {
      if (disposed) return;
      document.querySelector<HTMLElement>('.settings-page .settings-section-tab.selected')?.focus();
    });
  }

  function focusRestoreButton(target: number) {
    requestAnimationFrame(() => {
      if (disposed) return;
      root?.querySelector<HTMLButtonElement>(`[data-restore="${target}"]`)?.focus();
    });
  }

  function focusMcpPrimary() {
    requestAnimationFrame(() => {
      if (disposed) return;
      (root?.querySelector<HTMLElement>('[data-mcp-copy]') ?? root?.querySelector<HTMLElement>('[data-mcp-primary]'))?.focus();
    });
  }

  function focusCancel(node: HTMLElement) {
    requestAnimationFrame(() => {
      if (!disposed && node.isConnected) node.focus();
    });
  }

  async function loadHistory(current = revision, refocus = false) {
    if (!native) {
      historyLoading = false;
      return;
    }
    const request = ++historySeq;
    loadedRevision = current;
    historyLoading = true;
    historyError = '';
    let loaded = false;
    try {
      const history = await invoke<HistoryEntry[]>('customization_history');
      if (disposed || request !== historySeq) return;
      entries = history;
      if (restoreTarget !== null && !history.some(entry => entry.revision === restoreTarget)) restoreTarget = null;
      loaded = true;
    } catch (error) {
      if (disposed || request !== historySeq) return;
      entries = [];
      restoreTarget = null;
      historyError = messageOf(error);
    } finally {
      if (!disposed && request === historySeq) {
        historyLoading = false;
        if (loaded && refocus) focusActiveSectionTab();
      }
    }
  }

  async function loadMcp(refocus = false) {
    if (!native) {
      mcpLoading = false;
      return;
    }
    mcpLoading = true;
    mcpError = '';
    let loaded = false;
    try {
      const status = await invoke<McpStatus>('customization_mcp_status');
      if (disposed) return;
      mcp = status;
      if (!status.enabled) token = '';
      loaded = true;
    } catch (error) {
      if (!disposed) mcpError = messageOf(error);
    } finally {
      if (!disposed) {
        mcpLoading = false;
        if (loaded && refocus) focusMcpPrimary();
      }
    }
  }

  async function runMcp(action: 'enable' | 'rotate' | 'disable') {
    if (!native || mcpBusy) return;
    mcpBusy = true;
    mcpError = '';
    mcpNote = '';
    let focusAfter = false;
    try {
      const result = await invoke<McpGrant>(`customization_mcp_${action}`);
      if (disposed) return;
      mcp = { enabled: result.enabled, endpoint: result.endpoint };
      if (!result.enabled) {
        token = '';
        mcpNote = 'AI access is off. The old token no longer works, and access stays off after a restart.';
      } else if (action === 'disable') {
        token = '';
        mcpNote = 'AI access is still on. Try turning it off again.';
      } else {
        token = result.token ?? '';
        mcpNote = action === 'rotate'
          ? 'A new token replaced the old one. Copy the connection details and update your MCP client.'
          : 'AI access is on for this Luma session. Copy the connection details now; the token is only available in this panel.';
      }
      focusAfter = true;
    } catch (error) {
      if (!disposed) mcpError = messageOf(error);
    } finally {
      if (!disposed) {
        mcpBusy = false;
        if (focusAfter) focusMcpPrimary();
      }
    }
  }

  async function copySetupPrompt() {
    mcpError = '';
    mcpNote = '';
    try {
      await navigator.clipboard.writeText(setupPrompt);
      if (!disposed) mcpNote = 'Setup prompt copied. It contains no connection details and is safe to share.';
    } catch {
      if (!disposed) mcpError = 'Could not copy the setup prompt. Select the text under Read the setup prompt and copy it manually.';
    }
  }

  async function copyConnection() {
    if (!mcp?.enabled || !token) return;
    mcpError = '';
    mcpNote = '';
    try {
      await navigator.clipboard.writeText(`Endpoint: ${mcp.endpoint}\nAuthorization: Bearer ${token}`);
      if (!disposed) mcpNote = 'Connection details copied. Treat them as a credential and do not paste them into chats, issues, or repositories.';
    } catch {
      if (!disposed) mcpError = 'Could not copy the connection details. Rotate the token and try again.';
    }
  }

  function askRestore(target: number) {
    if (documentBusy) return;
    restoreTarget = target;
    restoreError = '';
    historyNote = '';
  }

  function cancelRestore() {
    const target = restoreTarget;
    restoreTarget = null;
    restoreError = '';
    if (target !== null) focusRestoreButton(target);
  }

  async function restoreVersion(entry: HistoryEntry) {
    if (!native || documentBusy || entry.revision === revision) return;
    restoreBusy = true;
    restoreError = '';
    historyNote = '';
    try {
      const document = await invoke<CustomizationDocument>('customization_restore_version', { expectedRevision: revision, revision: entry.revision });
      if (disposed) return;
      restoreTarget = null;
      onchanged?.();
      historyNote = `Restored the saved version from ${savedLabel(entry.savedAt)}.`;
      focusActiveSectionTab();
      if (document.revision !== loadedRevision) await loadHistory(document.revision);
    } catch (error) {
      if (!disposed) restoreError = messageOf(error);
    } finally {
      if (!disposed) restoreBusy = false;
    }
  }

  function askReset() {
    if (documentBusy) return;
    resetConfirming = true;
    resetError = '';
    resetNote = '';
  }

  function cancelReset() {
    resetConfirming = false;
    resetError = '';
    requestAnimationFrame(() => {
      if (disposed) return;
      root?.querySelector<HTMLButtonElement>('[data-reset-trigger]')?.focus();
    });
  }

  async function resetDefaults() {
    if (!native || documentBusy) return;
    resetBusy = true;
    resetError = '';
    resetNote = '';
    try {
      const document = await invoke<CustomizationDocument>('customization_reset_appearance', { expectedRevision: revision });
      if (disposed) return;
      resetConfirming = false;
      onchanged?.();
      resetNote = 'Appearance and Home reset to defaults. App order, sounds, accounts, app locations, and saved versions were kept.';
      focusActiveSectionTab();
      if (document.revision !== loadedRevision) await loadHistory(document.revision);
    } catch (error) {
      if (!disposed) resetError = messageOf(error);
    } finally {
      if (!disposed) resetBusy = false;
    }
  }

  function mountRoot(element: HTMLElement) {
    root = element;
    return () => { if (root === element) root = undefined; };
  }

  onMount(() => {
    if (native) void loadMcp();
    else mcpLoading = false;
  });

  $effect(() => {
    const current = revision;
    if (!native || current === loadedRevision) return;
    void loadHistory(current);
  });

  onDestroy(() => {
    disposed = true;
    historySeq++;
  });
</script>

{#if section === 'ai'}
  <section class="settings-group" {@attach mountRoot} aria-busy={native && (mcpLoading || mcpBusy)}>
    <div class="settings-group-head">
      <h3>AI access</h3>
      {#if native && mcp}<span class={`settings-badge ${mcpTone}`}>{mcp.enabled ? 'On' : 'Off'}</span>{/if}
    </div>
    <p class="panel-description">Let an MCP client on this PC inspect Luma and stage changes for your review. Nothing an agent stages is applied until you press Keep in Luma.</p>
    {#if !native}
      <p class="panel-description">Available in the desktop application.</p>
    {:else if mcpLoading}
      <p class="panel-description" role="status">Checking AI access…</p>
    {:else if mcpError && !mcp}
      <p class="error" role="alert">{mcpError}</p>
      <div class="settings-actions">
        <button class="panel-secondary" onclick={() => void loadMcp(true)}>Try again</button>
      </div>
    {:else if mcp}
      <div class="settings-row">
        <div class="settings-row-copy">
          <strong>{mcp.enabled ? 'AI access is on' : 'AI access is off'}</strong>
          <span>{mcp.enabled ? 'An MCP client can connect while Luma is running.' : 'Luma keeps agent access off and will not start it until you turn it on.'}</span>
        </div>
      </div>
      {#if mcp.enabled}
        <div class="settings-row">
          <div class="settings-row-copy">
            <strong>Local endpoint</strong>
            {#if mcp.endpoint}
              <span><code class="mcp-endpoint">{mcp.endpoint}</code></span>
            {:else}
              <span>No endpoint was reported. Try turning access off and on again.</span>
            {/if}
          </div>
        </div>
      {/if}
      <div class="settings-actions">
        {#if mcp.enabled}
          <button class="panel-secondary" data-mcp-primary disabled={mcpBusy} onclick={() => runMcp('rotate')}>Rotate token</button>
          <button class="text-link is-danger" disabled={mcpBusy} onclick={() => runMcp('disable')}>Turn off AI access</button>
        {:else}
          <button class="panel-secondary is-primary" data-mcp-primary disabled={mcpBusy} onclick={() => runMcp('enable')}>Turn on AI access</button>
        {/if}
      </div>
      {#if mcpNote}<p class="panel-description" role="status">{mcpNote}</p>{/if}
      {#if mcpError}<p class="error" role="alert">{mcpError}</p>{/if}
      <p class="panel-description">{mcp.enabled ? 'Access is on for this session. It turns off when Luma quits, so turn it on again next time.' : 'Access is off by default and stays off after Luma restarts. Turn it on when you are ready to connect an MCP client.'}</p>
      {#if mcp.enabled}
        {#if token}
          <div class="mcp-credential">
            <h4>Connection details ready</h4>
            <p>Copy these now. The token is not saved; it is only available until you leave Customization settings or Luma quits.</p>
            <button class="panel-secondary is-primary" data-mcp-copy disabled={mcpBusy || !mcp.endpoint} onclick={copyConnection}>Copy connection details</button>
            <p class="mcp-warning">This text is a credential. Paste it only into your MCP client configuration, never into a chat, issue, or repository.</p>
          </div>
        {:else}
          <p class="panel-description">The token is not shown again after you leave Customization settings. Rotate it to create new connection details. Luma also needs access turned on again after it restarts.</p>
        {/if}
        <p class="mcp-subhead">Set up your MCP client</p>
        <p class="panel-description">Your MCP client must already support connecting to a local server on this PC. Luma cannot configure the client for you, and not every client can reach a local address.</p>
        <div class="settings-actions">
          <button class="panel-secondary" onclick={copySetupPrompt}>Copy setup prompt</button>
        </div>
        <p class="panel-description">The setup prompt contains no endpoint or token and is safe to share.</p>
        <details class="prompt-preview">
          <summary>Read the setup prompt</summary>
          <pre>{setupPrompt}</pre>
        </details>
      {/if}
    {/if}
  </section>
{:else if section === 'versions'}
  <section class="settings-group" {@attach mountRoot} aria-busy={native && (historyLoading || documentBusy)}>
    <div class="settings-group-head">
      <h3>Saved versions</h3>
      {#if native && entries.length}<span class="settings-badge is-muted">{entries.length} {entries.length === 1 ? 'version' : 'versions'}</span>{/if}
    </div>
    <p class="panel-description">Luma keeps the last ten saved customization versions here. Restoring one creates a new revision, so you can return to what you have now.</p>
    {#if historyNote}<p class="panel-description" role="status">{historyNote}</p>{/if}
    {#if !native}
      <p class="panel-description">Available in the desktop application.</p>
    {:else if historyLoading}
      <p class="panel-description" role="status">Loading saved versions…</p>
    {:else if historyError}
      <p class="error" role="alert">{historyError}</p>
      <div class="settings-actions">
        <button class="panel-secondary" onclick={() => void loadHistory(revision, true)}>Try again</button>
      </div>
    {:else if !entries.length}
      <p class="panel-description">No saved versions yet. Change the theme or Home layout, and the new version will appear here.</p>
    {:else}
      <div class="history-list">
        {#each entries as entry (entry.revision)}
          <div class="settings-row">
            <div class="settings-row-copy">
              <strong>{savedLabel(entry.savedAt)}</strong>
              <span>{describe(entry.config)} · Revision {entry.revision}</span>
            </div>
            {#if entry.revision === revision}
              <span class="settings-badge is-good">Current</span>
            {:else}
              <button class="panel-secondary" data-restore={entry.revision} disabled={documentBusy} aria-label={`Restore version from ${savedLabel(entry.savedAt)}`} onclick={() => askRestore(entry.revision)}>Restore</button>
            {/if}
          </div>
          {#if restoreTarget === entry.revision}
            <div class="history-confirm" role="group" aria-label="Confirm restoring a saved version">
              <p>Restore the version from {savedLabel(entry.savedAt)}? This replaces the current theme, layout, widgets, app order, and sound. Accounts and app locations stay. A new revision is created, so this version stays in history.</p>
              {#if restoreError}<p class="error" role="alert">{restoreError}</p>{/if}
              <div class="settings-actions">
                <button class="panel-secondary" disabled={documentBusy} {@attach focusCancel} onclick={cancelRestore}>Cancel</button>
                <button class="panel-secondary is-primary" disabled={documentBusy} onclick={() => restoreVersion(entry)}>{restoreBusy ? 'Restoring…' : 'Restore version'}</button>
              </div>
            </div>
          {/if}
        {/each}
      </div>
    {/if}
  </section>
{:else if section === 'reset'}
  <section class="settings-group" {@attach mountRoot} aria-busy={resetBusy}>
    <h3>Reset appearance and Home</h3>
    <p class="panel-description">Return the visual style, Home layout, and widgets to their defaults. This creates a new saved version, so you can restore the previous setup from Saved versions.</p>
    {#if !native}
      <p class="panel-description">Available in the desktop application.</p>
    {:else}
      <ul class="reset-scope">
        <li>Theme returns to {defaultThemeName}.</li>
        <li>Density, tile shape, text size, and motion return to their defaults.</li>
        <li>Home rows, columns, and widgets return to their defaults.</li>
      </ul>
      {#if !resetConfirming}
        <div class="settings-actions">
          <button class="panel-secondary" data-reset-trigger disabled={documentBusy} onclick={askReset}>Reset to defaults</button>
        </div>
      {:else}
        <div class="reset-confirm" role="group" aria-label="Confirm resetting appearance and Home">
          <p>Reset appearance and Home to defaults? App order, sounds, accounts, app locations, and saved versions stay.</p>
          {#if resetError}<p class="error" role="alert">{resetError}</p>{/if}
          <div class="settings-actions">
            <button class="panel-secondary" disabled={documentBusy} {@attach focusCancel} onclick={cancelReset}>Cancel</button>
            <button class="panel-secondary is-primary" disabled={documentBusy} onclick={resetDefaults}>{resetBusy ? 'Resetting…' : 'Reset to defaults'}</button>
          </div>
        </div>
      {/if}
      {#if resetNote}<p class="panel-description" role="status">{resetNote}</p>{/if}
    {/if}
  </section>
{/if}

<style>
  .mcp-endpoint { display: inline-block; max-width: 100%; padding: 2px 6px; border: 1px solid var(--line); border-radius: 6px; background: color-mix(in srgb, var(--surface-3) 70%, transparent); color: var(--text); font-family: ui-monospace, 'Cascadia Code', Consolas, monospace; font-size: .95em; overflow-wrap: anywhere; }
  .mcp-credential { margin: 16px 0; padding: clamp(16px, 1.5vw, 22px); border: 1px solid color-mix(in srgb, var(--accent) 35%, transparent); border-radius: 14px; background: color-mix(in srgb, var(--accent) 8%, transparent); }
  .mcp-credential h4 { margin: 0 0 8px; font-size: clamp(16px, 1.1vw, 20px); font-weight: 650; letter-spacing: -.015em; }
  .mcp-credential p { margin: 0 0 14px; color: var(--text-dim); font-size: clamp(13px, .88vw, 16px); line-height: 1.55; }
  .mcp-credential .panel-secondary { margin: 0; }
  .mcp-credential .mcp-warning { margin: 12px 0 0; color: var(--warn); font-size: clamp(12px, .8vw, 14px); line-height: 1.5; }
  .mcp-subhead { margin: 24px 0 8px; color: var(--text); font-size: clamp(16px, 1.1vw, 20px); font-weight: 650; letter-spacing: -.015em; }
  .prompt-preview { margin-top: 10px; border: 1px solid var(--line); border-radius: 12px; background: color-mix(in srgb, var(--surface) 55%, transparent); }
  .prompt-preview summary { padding: 14px 16px; color: var(--text-dim); font-size: clamp(13px, .9vw, 16px); font-weight: 600; cursor: pointer; }
  .prompt-preview summary:focus-visible { outline: 3px solid var(--accent); outline-offset: -3px; border-radius: 12px; }
  .prompt-preview[open] summary { border-bottom: 1px solid var(--line); margin-bottom: 14px; }
  .prompt-preview pre { margin: 0; padding: 0 16px 16px; max-height: 280px; overflow: auto; color: var(--text-dim); font-family: ui-monospace, 'Cascadia Code', Consolas, monospace; font-size: clamp(12px, .8vw, 14px); line-height: 1.55; white-space: pre-wrap; overflow-wrap: anywhere; }
  .history-list { display: grid; gap: 10px; }
  .history-confirm, .reset-confirm { margin-top: 12px; padding: 16px 18px; border: 1px solid color-mix(in srgb, var(--warn) 35%, transparent); border-radius: 13px; background: color-mix(in srgb, var(--warn) 6%, transparent); }
  .history-confirm p, .reset-confirm p { margin: 0; color: var(--text-dim); font-size: clamp(13px, .88vw, 16px); line-height: 1.55; }
  .history-confirm p.error, .reset-confirm p.error { margin-top: 10px; color: var(--danger); }
  .history-confirm .settings-actions, .reset-confirm .settings-actions { margin-top: 14px; }
  .reset-scope { display: grid; gap: 6px; margin: 0 0 18px; padding-left: 20px; color: var(--text-dim); font-size: clamp(13px, .88vw, 16px); line-height: 1.55; }
  .reset-scope li::marker { color: var(--muted); }
</style>
