<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { pluginLogs } from '../services/tauri';
  import type { PluginRecord, PluginSettingValue, PluginTrigger, PluginsState } from '../services/plugins';
  import {
    pluginControlSummary, pluginDefaultTrigger, pluginFieldLabel, pluginPermissionDescription,
    pluginRuntimeKind, pluginRuntimeLabel, pluginRuntimeStateLabel, pluginRuntimeTone, pluginSettingValues,
    pluginTriggerDescription, pluginTriggerLabel, pluginTriggers, pluginTriggerSummary,
  } from '../services/plugins';

  type ImportResult = { plugin: PluginRecord; alreadyInstalled: boolean; replacedVersion: string | null };

  let { native, revision, plugins, loading, error, reload }: {
    native: boolean;
    revision: number;
    plugins: PluginsState | null;
    loading: boolean;
    error: string;
    reload: () => Promise<void>;
  } = $props();

  let busy = $state('');
  let actionError = $state('');
  let note = $state('');
  let removeTarget = $state<string | null>(null);
  let deleteData = $state(false);
  let consentTarget = $state<string | null>(null);
  let consentTrigger = $state<PluginTrigger>('launcher-open');
  let modalPluginId = $state<string | null>(null);
  let modalTab = $state<string>('General');
  let logsById = $state<Record<string, string[]>>({});
  let logsLoading = $state<Record<string, boolean>>({});
  let root = $state<HTMLElement>();

  async function loadLogs(pluginId: string) {
    if (!native || logsLoading[pluginId]) return;
    logsLoading = { ...logsLoading, [pluginId]: true };
    try {
      const entries = await pluginLogs(pluginId);
      logsById = { ...logsById, [pluginId]: entries };
    } catch {
      logsById = { ...logsById, [pluginId]: [] };
    } finally {
      logsLoading = { ...logsLoading, [pluginId]: false };
    }
  }

  const list = $derived(plugins?.plugins ?? []);
  const enabledCount = $derived(list.filter(plugin => plugin.enabled).length);
  const consentRecord = $derived(consentTarget ? list.find(plugin => plugin.id === consentTarget && !plugin.enabled && plugin.canEnable) ?? null : null);
  const modalRecord = $derived(modalPluginId ? list.find(plugin => plugin.id === modalPluginId) ?? null : null);

  function settingTabsFor(record: PluginRecord) {
    const tabs: string[] = [];
    for (const setting of record.settings) {
      const tab = setting.tab?.trim() || 'General';
      if (!tabs.includes(tab)) tabs.push(tab);
    }
    tabs.push('Details');
    return tabs;
  }

  function openPluginModal(record: PluginRecord) {
    const tabs = settingTabsFor(record);
    modalPluginId = record.id;
    modalTab = tabs[0] ?? 'Details';
    if (modalTab === 'Details' && record.executable) void loadLogs(record.id);
  }

  function closePluginModal() {
    const id = modalPluginId;
    modalPluginId = null;
    if (id) {
      requestAnimationFrame(() => root?.querySelector<HTMLElement>(`[data-plugin-settings="${id}"]`)?.focus());
    }
  }

  function selectModalTab(record: PluginRecord, tab: string) {
    modalTab = tab;
    if (tab === 'Details' && record.executable) void loadLogs(record.id);
  }

  function messageOf(value: unknown) {
    return value && typeof value === 'object' && 'message' in value ? String(value.message) : String(value);
  }

  function bytesLabel(bytes: number) {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${Math.round(bytes / 1024)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  }

  function sourceLabel(source: string) {
    if (source === 'builtin') return 'Built-in';
    return source === 'mcp' ? 'Added by AI' : 'Imported';
  }

  function focusPlugin(id: string) {
    requestAnimationFrame(() => root?.querySelector<HTMLElement>(`[data-plugin="${id}"]`)?.focus());
  }

  function focusPluginAction(id: string) {
    requestAnimationFrame(() => root?.querySelector<HTMLElement>(`[data-plugin="${id}"] .plugin-actions button:not(:disabled)`)?.focus());
  }

  function focusRemove(id: string) {
    requestAnimationFrame(() => root?.querySelector<HTMLElement>(`[data-plugin-remove="${id}"]`)?.focus());
  }

  function focusImport() {
    requestAnimationFrame(() => root?.querySelector<HTMLElement>('[data-plugin-import]')?.focus());
  }

  function focusRemoveCancel() {
    requestAnimationFrame(() => root?.querySelector<HTMLElement>('[data-plugin-remove-cancel]')?.focus());
  }

  function attachFocus(node: HTMLElement) {
    requestAnimationFrame(() => { if (node.isConnected) node.focus(); });
  }

  export function consumeBack() {
    if (modalRecord) {
      if (!busy) closePluginModal();
      return true;
    }
    if (!consentRecord && !removeTarget) return false;
    if (!busy) {
      if (consentRecord) cancelConsent();
      else cancelRemove();
    }
    return true;
  }

  function interceptBack(event: KeyboardEvent) {
    if (event.key !== 'Escape' && event.key !== 'Backspace') return;
    if (event.key === 'Backspace') {
      const target = event.target;
      if (target instanceof HTMLTextAreaElement || target instanceof HTMLSelectElement) return;
      if (target instanceof HTMLElement && target.isContentEditable) return;
      if (target instanceof HTMLInputElement && target.type !== 'checkbox' && target.type !== 'radio') return;
    }
    if (consumeBack()) {
      event.preventDefault();
      event.stopPropagation();
    }
  }

  async function run(message: string, task: () => Promise<void>) {
    actionError = '';
    note = '';
    try {
      await task();
      if (message) note = message;
    } catch (value) {
      actionError = messageOf(value);
    }
  }

  async function retryPlugins() {
    if (busy) return;
    busy = 'reload';
    await run('', reload);
    busy = '';
  }

  async function importPlugin() {
    if (!native || busy) return;
    busy = 'import';
    await run('', async () => {
      const result = await invoke<ImportResult | null>('customization_plugin_import');
      if (!result) return;
      await reload();
      note = result.alreadyInstalled
        ? `${result.plugin.name} ${result.plugin.version} is already installed.`
        : result.replacedVersion
          ? `${result.plugin.name} updated to ${result.plugin.version}. It is disabled until you ${result.plugin.executable ? 'review its access' : 'preview it'}.`
          : `${result.plugin.name} imported. It is disabled until you ${result.plugin.executable ? 'review its access' : 'preview and enable it'}.`;
      focusPlugin(result.plugin.id);
    });
    busy = '';
  }

  async function previewPlugin(record: PluginRecord) {
    if (!native || busy) return;
    busy = record.id;
    await run('', async () => {
      await invoke('customization_plugin_preview', { pluginId: record.id, expectedRevision: revision });
      note = `Preview opened for ${record.name}. Press Keep in Luma to ${record.enabled ? 'restore its controls' : 'enable it'}.`;
    });
    busy = '';
  }

  async function openSurface(record: PluginRecord) {
    if (!native || busy || !record.enabled || !record.surfaceOpenable) return;
    busy = record.id;
    await run('', async () => {
      await invoke('customization_plugin_open_surface', { pluginId: record.id });
      note = `Opened ${record.name}. Use Back in Luma to return.`;
    });
    busy = '';
  }

  function reviewRuntime(record: PluginRecord) {
    if (!native || busy) return;
    removeTarget = null;
    actionError = '';
    note = '';
    consentTrigger = pluginDefaultTrigger(record);
    consentTarget = record.id;
  }

  function cancelConsent() {
    const target = consentTarget;
    consentTarget = null;
    actionError = '';
    if (target) {
      requestAnimationFrame(() => {
        const review = root?.querySelector<HTMLElement>(`[data-plugin-review="${target}"]`);
        (review ?? root?.querySelector<HTMLElement>(`[data-plugin="${target}"] .plugin-actions button:not(:disabled)`))?.focus();
      });
    }
  }

  function chooseTrigger(event: Event) {
    consentTrigger = (event.currentTarget as HTMLInputElement).value as PluginTrigger;
  }

  async function enableRuntime(record: PluginRecord) {
    if (!native || busy || !record.canEnable) return;
    const trigger = consentTrigger;
    busy = record.id;
    await run('', async () => {
      await invoke('customization_plugin_enable_runtime', {
        pluginId: record.id,
        expectedRevision: revision,
        permissions: record.permissions,
        network: record.network,
        trigger,
      });
      consentTarget = null;
      await reload();
      note = trigger === 'on-demand'
        ? `${record.name} enabled. It starts only when you open it; disabling it revokes access.`
        : trigger === 'launcher-open'
          ? `${record.name} enabled. It runs while Luma is open; disabling it revokes access.`
          : trigger === 'startup'
            ? `${record.name} enabled. It starts with Luma, including in the tray; disabling it revokes access.`
            : `${record.name} enabled. It keeps running in the tray until you disable it.`;
    });
    busy = '';
    if (!actionError) focusPluginAction(record.id);
  }

  async function toggleBuiltin(record: PluginRecord, enabled: boolean) {
    if (!native || busy || !record.canEnable) return;
    busy = record.id;
    await run(enabled ? `${record.name} is on.` : `${record.name} is off. Its saved settings are kept.`, async () => {
      await invoke('customization_plugin_toggle', { pluginId: record.id, expectedRevision: revision, enabled });
      await reload();
    });
    busy = '';
    if (!actionError) focusPluginAction(record.id);
  }

  async function disablePlugin(record: PluginRecord) {
    if (!native || busy) return;
    busy = record.id;
    await run(record.executable
      ? `${record.name} disabled. Its runtime access is revoked and saved settings are kept.`
      : `${record.name} disabled. Its saved settings are kept.`, async () => {
      await invoke('customization_plugin_disable', { pluginId: record.id, expectedRevision: revision });
      if (consentTarget === record.id) consentTarget = null;
      await reload();
    });
    busy = '';
    if (!actionError) focusPluginAction(record.id);
  }

  function askRemove(record: PluginRecord) {
    deleteData = false;
    removeTarget = record.id;
    consentTarget = null;
    actionError = '';
    note = '';
    focusRemoveCancel();
  }

  function cancelRemove() {
    const target = removeTarget;
    removeTarget = null;
    if (target) focusRemove(target);
  }

  async function removePlugin(record: PluginRecord) {
    if (!native || busy) return;
    busy = record.id;
    await run(`${record.name} removed. ${deleteData ? 'Its saved settings and local data were deleted.' : 'Its saved settings and local data are kept for reinstalling.'}`, async () => {
      try {
        await invoke('customization_plugin_remove', { pluginId: record.id, expectedRevision: revision, deleteData });
      } catch (error) {
        if (error && typeof error === 'object' && 'code' in error && error.code === 'plugin_data_cleanup') {
          removeTarget = null;
          await reload();
          focusImport();
        }
        throw error;
      }
      if (removeTarget === record.id) removeTarget = null;
      await reload();
      focusImport();
    });
    busy = '';
  }

  async function exportPlugin(record: PluginRecord) {
    if (!native || busy) return;
    busy = record.id;
    await run('', async () => {
      const name = await invoke<string | null>('customization_plugin_export', { pluginId: record.id });
      if (name) note = `Exported ${name}.`;
    });
    busy = '';
  }

  async function saveSetting(record: PluginRecord, key: string, value: PluginSettingValue) {
    if (!native || busy) return;
    busy = `${record.id}:${key}`;
    await run('Settings saved.', async () => {
      await invoke('customization_plugin_settings_save', {
        pluginId: record.id,
        expectedRevision: revision,
        settings: { ...pluginSettingValues(record), [key]: value },
      });
      await reload();
    });
    busy = '';
  }

  function booleanSetting(record: PluginRecord, key: string, event: Event) {
    const input = event.currentTarget as HTMLInputElement;
    void saveSetting(record, key, input.checked);
  }

  function textSetting(record: PluginRecord, key: string, event: Event) {
    const input = event.currentTarget as HTMLInputElement;
    void saveSetting(record, key, input.value);
  }

  function numberSetting(record: PluginRecord, key: string, event: Event) {
    const input = event.currentTarget as HTMLInputElement;
    const value = Number(input.value);
    if (!Number.isFinite(value)) {
      actionError = 'Enter a number before saving.';
      return;
    }
    void saveSetting(record, key, value);
  }

  function selectSetting(record: PluginRecord, key: string, event: Event) {
    const input = event.currentTarget as HTMLSelectElement;
    void saveSetting(record, key, input.value);
  }

  function mountRoot(element: HTMLElement) {
    root = element;
    return () => { if (root === element) root = undefined; };
  }
</script>

<svelte:window onkeydowncapture={interceptBack} />

<section class="settings-group plugins-panel" {@attach mountRoot} aria-busy={loading || !!busy}>
  <div class="settings-group-head">
    <h3>Installed plugins</h3>
    {#if native && list.length}<span class="settings-badge is-muted">{enabledCount} of {list.length} on</span>{/if}
  </div>
  <p class="panel-description">Add optional features and layouts to Luma. Imported plugins stay off until you turn them on.</p>
  {#if !native}
    <p class="panel-description">Available in the desktop application.</p>
  {:else}
    <div class="settings-actions">
      <button class="panel-secondary is-primary" data-plugin-import disabled={!!busy} onclick={importPlugin}>{busy === 'import' ? 'Choosing a plugin…' : 'Import plugin'}</button>
    </div>
    {#if note}<p class="panel-description" role="status">{note}</p>{/if}
    {#if actionError}<p class="error" role="alert">{actionError}</p>{/if}
    {#if error}
      <p class="error" role="alert">{error}</p>
      <div class="settings-actions"><button class="panel-secondary" disabled={!!busy} onclick={retryPlugins}>{busy === 'reload' ? 'Checking…' : 'Try again'}</button></div>
    {/if}
    {#if loading && !plugins}
      <p class="panel-description" role="status">Checking installed plugins…</p>
    {:else if !list.length}
      <p class="panel-description">No plugins installed. Import a .luma-extension.zip file to get started.</p>
    {:else}
      <div class="plugin-list">
        {#each list as record (record.id)}
          <article class={['plugin-card', { 'is-disabled': !record.enabled }]} data-plugin={record.id} tabindex="-1">
            <header class="plugin-head">
              <div class="plugin-identity">
                <button type="button" class="plugin-icon-btn" aria-label={`Open ${record.name} settings`} onclick={() => openPluginModal(record)}>
                  {#if record.iconUrl}
                    <img class="plugin-icon" src={record.iconUrl} alt="" />
                  {:else}
                    <span class="plugin-icon-fallback" aria-hidden="true">{record.name.slice(0, 1).toUpperCase()}</span>
                  {/if}
                </button>
                <div class="plugin-title">
                  <strong>{record.name}</strong>
                  <span class="plugin-version">v{record.version}</span>
                  {#if record.enabled}<span class="settings-badge is-good">Enabled</span>{:else}<span class="settings-badge is-muted">Disabled</span>{/if}
                  {#if record.executable && record.executionReason}
                    <span class="settings-badge is-warn">Runtime not supported</span>
                  {:else if record.executable}
                    <span class="settings-badge is-muted">{pluginRuntimeLabel(record)}</span>
                  {/if}
                </div>
              </div>
              <span class="plugin-origin">{sourceLabel(record.source)}</span>
            </header>
            {#if record.description}<p class="plugin-description">{record.description}</p>{/if}
            {#if record.controls.length}<p class="plugin-status">{pluginControlSummary(record)}</p>{/if}
            {#if record.executable && (record.runtimeStatus?.state === 'failed' || (record.runtimeStatus?.restarts ?? 0) > 0)}
              <p class="plugin-runtime" role="status">
                {#if record.runtimeStatus}
                  <span class={['settings-badge', pluginRuntimeTone(record.runtimeStatus.state)]}>{pluginRuntimeStateLabel(record.runtimeStatus.state)}</span>
                {/if}
                {#if record.runtimeStatus?.state === 'failed' && !record.enabled}
                  <span>Stopped after repeated failures</span>
                {/if}
                {#if record.runtimeStatus && record.runtimeStatus.restarts > 0}
                  <span>{record.runtimeStatus.restarts === 1 ? 'Restarted once' : `Restarted ${record.runtimeStatus.restarts} times`}</span>
                {/if}
              </p>
              {#if record.runtimeStatus?.reason && record.runtimeStatus.reason !== record.disabledReason && record.runtimeStatus.reason !== record.enableError}
                <p class="plugin-warning" role="status">{record.runtimeStatus.reason}</p>
              {/if}
              {#if !record.enabled && record.runtimeStatus?.state === 'failed' && record.canEnable}
                <p class="plugin-warning" role="status">Turn it back on to try again.</p>
              {/if}
            {/if}
            {#if record.executionReason}<p class="plugin-warning" role="status">{record.executionReason}</p>{/if}
            {#if record.disabledReason}<p class="plugin-warning" role="status">{record.disabledReason}</p>{/if}
            {#if record.detachedFields.length}<p class="plugin-warning" role="status">You changed {record.detachedFields.map(pluginFieldLabel).join(', ')} in Luma, so this plugin no longer controls {record.detachedFields.length === 1 ? 'that setting' : 'those settings'}.</p>{/if}
            {#if record.conflicts.length}<p class="plugin-warning" role="status">Conflicts with another plugin: {record.conflicts.join(', ')}.</p>{/if}
            {#if record.enableError && record.enableError !== record.executionReason}<p class="plugin-warning" role="status">{record.enableError}</p>{/if}
            <div class="settings-actions plugin-actions">
              {#if record.enabled}
                {#if record.executable}
                  {#if record.surfaceOpenable}
                    <button class="panel-secondary is-primary" data-plugin-open={record.id} disabled={!!busy} onclick={() => openSurface(record)}>{busy === record.id ? 'Opening…' : 'Open'}</button>
                  {/if}
                  <button class="panel-secondary" disabled={!!busy} onclick={() => disablePlugin(record)}>{busy === record.id ? 'Working…' : 'Turn off'}</button>
                {:else}
                  {#if record.detachedFields.length}
                    <button class="panel-secondary is-primary" disabled={!!busy} onclick={() => previewPlugin(record)}>{busy === record.id ? 'Opening…' : 'Restore controls'}</button>
                  {/if}
                  <button class="panel-secondary" disabled={!!busy} onclick={() => disablePlugin(record)}>{busy === record.id ? 'Working…' : 'Turn off'}</button>
                {/if}
              {:else if record.executable}
                <button class="panel-secondary is-primary" data-plugin-review={record.id} disabled={!!busy || !record.canEnable || consentRecord?.id === record.id} onclick={() => reviewRuntime(record)}>{busy === record.id ? 'Turning on…' : 'Turn on'}</button>
              {:else if record.builtin}
                <button class="panel-secondary is-primary" disabled={!!busy || !record.canEnable} onclick={() => toggleBuiltin(record, true)}>{busy === record.id ? 'Turning on…' : 'Turn on'}</button>
              {:else}
                <button class="panel-secondary is-primary" disabled={!!busy || !record.canEnable} onclick={() => previewPlugin(record)}>{busy === record.id ? 'Opening…' : 'Preview & turn on'}</button>
              {/if}
              <button class="panel-secondary" data-plugin-settings={record.id} disabled={!!busy} onclick={() => openPluginModal(record)}>
                {record.settings.length ? 'Settings' : 'Details'}
              </button>
              <button class="panel-secondary" disabled={!!busy} onclick={() => exportPlugin(record)}>Export</button>
              {#if !record.builtin}
                <button class="text-link is-danger" data-plugin-remove={record.id} disabled={!!busy} onclick={() => askRemove(record)}>Remove</button>
              {/if}
            </div>
            {#if consentRecord?.id === record.id}
              <div class="plugin-consent" role="group" aria-label={`Review access for ${record.name}`} data-plugin-consent={record.id}>
                <h4>Review access</h4>
                <p class="plugin-consent-identity">{pluginRuntimeLabel(record)} · {record.id} · v{record.version} · {sourceLabel(record.source)}</p>
                <p>{record.executable && pluginRuntimeKind(record) === 'background'
                  ? 'This plugin runs a background service without a window. Enabling it grants the access below and starts the service.'
                  : 'This plugin runs its own interface in a separate Luma window. Enabling it grants the access below and allows it to open while Luma is running.'}</p>
                <div class="plugin-consent-group">
                  <strong>Permissions</strong>
                  {#if record.permissions.length}
                    <ul>
                      {#each record.permissions as permission (permission)}
                        <li><code>{permission}</code><span>{pluginPermissionDescription(permission)}</span></li>
                      {/each}
                    </ul>
                  {:else}
                    <p>No permissions requested.</p>
                  {/if}
                </div>
                <div class="plugin-consent-group">
                  <strong>Network</strong>
                  {#if record.network.length}
                    <ul>
                      {#each record.network as origin (origin)}
                        <li><code>{origin}</code></li>
                      {/each}
                    </ul>
                    <p>Requests go through Luma to these origins only.</p>
                  {:else}
                    <p>No network access requested.</p>
                  {/if}
                </div>
                {#if pluginTriggers(record).length > 1}
                  <fieldset class="plugin-consent-triggers">
                    <legend>When should it run?</legend>
                    {#each pluginTriggers(record) as trigger (trigger)}
                      <label class="plugin-consent-trigger">
                        <input type="radio" name={`plugin-trigger-${record.id}`} value={trigger} checked={consentTrigger === trigger} disabled={!!busy} onchange={chooseTrigger} />
                        <span><strong>{pluginTriggerLabel(trigger)}</strong><small>{pluginTriggerDescription(trigger)}</small></span>
                      </label>
                    {/each}
                  </fieldset>
                {:else}
                  <div class="plugin-consent-group">
                    <strong>{pluginTriggerLabel(consentTrigger)}</strong>
                    <p>{pluginTriggerDescription(consentTrigger)}</p>
                  </div>
                {/if}
                {#if consentTrigger === 'startup' || consentTrigger === 'always'}
                  <p class="plugin-warning" role="status">This service can keep running in the background while Luma is in the tray. It stops when you disable it or exit Luma.</p>
                {/if}
                {#if actionError}<p class="error" role="alert">{actionError}</p>{/if}
                <div class="settings-actions">
                  <button class="panel-secondary" data-plugin-consent-cancel disabled={!!busy} {@attach attachFocus} onclick={cancelConsent}>Cancel</button>
                  <button class="panel-secondary is-primary" disabled={!!busy} onclick={() => enableRuntime(record)}>{busy === record.id ? 'Enabling…' : consentTrigger === 'startup' ? 'Allow and start with Luma' : consentTrigger === 'always' ? 'Allow always-on service' : 'Allow and enable'}</button>
                </div>
              </div>
            {/if}
            {#if removeTarget === record.id}
              <div class="plugin-confirm" role="group" aria-label={`Confirm removing ${record.name}`}>
                <p>Remove {record.name}? Its code and access are removed. Saved settings and local data are kept for reinstalling unless you delete them below.</p>
                <label class="plugin-consent-trigger"><input type="checkbox" bind:checked={deleteData} disabled={!!busy} /><span>Also delete this plugin's saved settings and local data</span></label>
                {#if actionError}<p class="error" role="alert">{actionError}</p>{/if}
                <div class="settings-actions">
                  <button class="panel-secondary" data-plugin-remove-cancel disabled={!!busy} {@attach attachFocus} onclick={cancelRemove}>Cancel</button>
                  <button class="panel-secondary is-primary" disabled={!!busy} onclick={() => removePlugin(record)}>{busy === record.id ? 'Removing…' : 'Remove plugin'}</button>
                </div>
              </div>
            {/if}
          </article>
        {/each}
      </div>
    {/if}
  {/if}

  {#if modalRecord}
    {@const tabs = settingTabsFor(modalRecord)}
    <div class="plugin-modal-backdrop" role="presentation" onclick={event => { if (event.target === event.currentTarget && !busy) closePluginModal(); }}>
      <div class="plugin-modal" role="dialog" aria-modal="true" aria-labelledby="plugin-modal-title">
        <header class="plugin-modal-head">
          <div class="plugin-identity">
            {#if modalRecord.iconUrl}
              <img class="plugin-icon" src={modalRecord.iconUrl} alt="" />
            {:else}
              <span class="plugin-icon-fallback" aria-hidden="true">{modalRecord.name.slice(0, 1).toUpperCase()}</span>
            {/if}
            <div>
              <h4 id="plugin-modal-title">{modalRecord.name}</h4>
              <p class="plugin-modal-sub">v{modalRecord.version} · {sourceLabel(modalRecord.source)}</p>
            </div>
          </div>
          <button type="button" class="panel-secondary" {@attach attachFocus} onclick={closePluginModal}>Close</button>
        </header>

        <div class="plugin-modal-tabs" role="tablist" aria-label={`${modalRecord.name} sections`}>
          {#each tabs as tab (tab)}
            <button
              type="button"
              role="tab"
              aria-selected={modalTab === tab}
              class={['plugin-modal-tab', { 'is-active': modalTab === tab }]}
              onclick={() => selectModalTab(modalRecord, tab)}
            >
              {tab}
            </button>
          {/each}
        </div>

        <div class="plugin-modal-body" role="tabpanel">
          {#if modalTab === 'Details'}
            <div class="plugin-modal-details">
              <p>{modalRecord.id} · API {modalRecord.apiVersion} · {bytesLabel(modalRecord.sizeBytes)} · {modalRecord.assets.length + modalRecord.sourceFiles.length} files{#if modalRecord.executable && modalRecord.enabled} · {pluginTriggerSummary(pluginDefaultTrigger(modalRecord))}{/if}</p>
              {#if modalRecord.executable}
                <p>Runtime: {pluginRuntimeLabel(modalRecord)} · Entry: {modalRecord.entry ?? 'Not declared'}</p>
                <p>Declared permissions: {modalRecord.permissions.length ? modalRecord.permissions.join(', ') : 'None'}</p>
                {#if modalRecord.network.length}<p>Declared network: {modalRecord.network.join(', ')}</p>{/if}
                {#if modalRecord.grantedPermissions?.length || modalRecord.grantedNetwork?.length}
                  <p>Granted permissions: {modalRecord.grantedPermissions?.length ? modalRecord.grantedPermissions.join(', ') : 'None'}</p>
                  {#if modalRecord.grantedNetwork?.length}<p>Granted network: {modalRecord.grantedNetwork.join(', ')}</p>{/if}
                {/if}
                {#if native}
                  <div class="plugin-logs">
                    <div class="plugin-logs-head">
                      <span>Recent runtime logs</span>
                      <button type="button" class="plugin-logs-refresh" disabled={Boolean(logsLoading[modalRecord.id])} onclick={() => void loadLogs(modalRecord.id)}>
                        {logsLoading[modalRecord.id] ? 'Refreshing…' : 'Refresh'}
                      </button>
                    </div>
                    {#if logsById[modalRecord.id]?.length}
                      <pre class="plugin-logs-body">{logsById[modalRecord.id].join('\n')}</pre>
                    {:else}
                      <p>No recent runtime logs.</p>
                    {/if}
                  </div>
                {/if}
              {/if}
              {#if modalRecord.author}<p>By {modalRecord.author}</p>{/if}
              <p class="plugin-hash">{modalRecord.contentHash.slice(0, 16)}…</p>
            </div>
          {:else}
            {@const tabSettings = modalRecord.settings.filter(s => (s.tab?.trim() || 'General') === modalTab)}
            <div class="plugin-modal-settings">
              {#each tabSettings as setting (setting.key)}
                <div class="plugin-setting">
                  <label for={`plugin-modal-${modalRecord.id}-${setting.key}`}>
                    <strong>{setting.label}</strong>
                    {#if setting.description}<span>{setting.description}</span>{/if}
                  </label>
                  {#if setting.kind === 'boolean'}
                    <input id={`plugin-modal-${modalRecord.id}-${setting.key}`} type="checkbox" checked={setting.value === true} disabled={!!busy} onchange={event => booleanSetting(modalRecord, setting.key, event)} />
                  {:else if setting.kind === 'enum'}
                    <select id={`plugin-modal-${modalRecord.id}-${setting.key}`} value={String(setting.value)} disabled={!!busy} onchange={event => selectSetting(modalRecord, setting.key, event)}>
                      {#each setting.options as option (option.value)}
                        <option value={option.value}>{option.label}</option>
                      {/each}
                    </select>
                  {:else if setting.kind === 'number'}
                    <input id={`plugin-modal-${modalRecord.id}-${setting.key}`} type="number" value={Number(setting.value)} min={setting.min ?? undefined} max={setting.max ?? undefined} step={setting.integer ? 1 : 'any'} disabled={!!busy} onchange={event => numberSetting(modalRecord, setting.key, event)} />
                  {:else}
                    <input id={`plugin-modal-${modalRecord.id}-${setting.key}`} type="text" value={String(setting.value)} maxlength={setting.maxLength} disabled={!!busy} onchange={event => textSetting(modalRecord, setting.key, event)} />
                  {/if}
                  {#if !setting.valid}<span class="plugin-warning">The stored value is invalid, so the default is in use. Choose a value and save to repair it.</span>{/if}
                </div>
              {/each}
            </div>
          {/if}
        </div>
      </div>
    </div>
  {/if}
</section>

<style>
  .plugin-list { display: grid; gap: 14px; margin-top: 16px; }
  .plugin-card { padding: clamp(16px, 1.4vw, 22px); border: 1px solid var(--line); border-radius: 14px; background: color-mix(in srgb, var(--surface) 58%, transparent); }
  .plugin-card:focus-visible { outline: 3px solid var(--accent); outline-offset: 2px; }
  .plugin-card.is-disabled { border-style: dashed; }
  .plugin-head { display: flex; align-items: flex-start; justify-content: space-between; gap: 14px; }
  .plugin-identity { display: flex; align-items: center; gap: 12px; min-width: 0; }
  .plugin-icon-btn { flex: none; padding: 0; border: 0; border-radius: 12px; background: transparent; cursor: pointer; display: grid; place-items: center; }
  .plugin-icon-btn:focus-visible { outline: 3px solid var(--accent); outline-offset: 3px; }
  .plugin-icon, .plugin-icon-fallback { width: 44px; height: 44px; border-radius: 12px; object-fit: cover; flex: none; }
  .plugin-icon-fallback { display: grid; place-items: center; background: color-mix(in srgb, var(--accent) 20%, var(--surface-3)); color: var(--text); font-weight: 700; font-size: 18px; border: 1px solid var(--line); }
  .plugin-title { display: flex; align-items: center; gap: 10px; flex-wrap: wrap; }
  .plugin-title strong { color: var(--text); font-size: clamp(17px, 1.2vw, 22px); font-weight: 650; letter-spacing: -.015em; }
  .plugin-version { color: var(--muted); font-size: clamp(12px, .82vw, 15px); font-variant-numeric: tabular-nums; }
  .plugin-origin { color: var(--muted); font-size: clamp(12px, .82vw, 15px); white-space: nowrap; }
  .plugin-description { margin: 8px 0 0; color: var(--text-dim); font-size: clamp(13px, .9vw, 16px); line-height: 1.55; }
  .plugin-status { margin: 10px 0 0; color: var(--text-dim); font-size: clamp(13px, .88vw, 16px); }
  .plugin-warning { margin: 8px 0 0; color: var(--warn); font-size: clamp(12px, .82vw, 15px); line-height: 1.5; }
  .plugin-actions { margin-top: 14px; }
  .plugin-runtime { display: flex; align-items: center; gap: 10px; flex-wrap: wrap; margin: 10px 0 0; color: var(--text-dim); font-size: clamp(13px, .88vw, 16px); }
  .plugin-consent { margin-top: 12px; padding: 16px 18px; border: 1px solid color-mix(in srgb, var(--accent) 38%, transparent); border-radius: 13px; background: color-mix(in srgb, var(--accent) 5%, transparent); }
  .plugin-consent h4 { margin: 0 0 8px; color: var(--text); font-size: clamp(15px, 1vw, 18px); font-weight: 650; }
  .plugin-consent p { margin: 0; color: var(--text-dim); font-size: clamp(13px, .88vw, 16px); line-height: 1.55; }
  .plugin-consent .plugin-consent-identity { color: var(--muted); font-size: clamp(12px, .82vw, 14px); }
  .plugin-consent .plugin-warning { margin-top: 8px; }
  .plugin-consent .error { margin-top: 10px; color: var(--danger); }
  .plugin-consent-group { margin-top: 14px; }
  .plugin-consent-group > strong { display: block; color: var(--text); font-size: clamp(13px, .9vw, 16px); }
  .plugin-consent-group ul { display: grid; gap: 6px; margin: 7px 0 0; padding: 0; list-style: none; }
  .plugin-consent-group li { display: flex; align-items: baseline; gap: 10px; flex-wrap: wrap; }
  .plugin-consent-group code { color: var(--text); font-family: ui-monospace, 'Cascadia Code', Consolas, monospace; font-size: clamp(13px, .88vw, 16px); overflow-wrap: anywhere; }
  .plugin-consent-group li span, .plugin-consent-group p { color: var(--muted); font-size: clamp(13px, .86vw, 16px); line-height: 1.5; }
  .plugin-consent-group > strong + p { margin-top: 5px; }
  .plugin-consent-triggers { margin: 14px 0 0; padding: 0; border: 0; }
  .plugin-consent-triggers legend { padding: 0; color: var(--text); font-size: clamp(13px, .9vw, 16px); font-weight: 650; }
  .plugin-consent-trigger { display: flex; align-items: flex-start; gap: 12px; padding: 9px 0; cursor: pointer; }
  .plugin-consent-trigger input { flex: none; width: 22px; height: 22px; margin: 2px 0 0; accent-color: var(--accent); }
  .plugin-consent-trigger input:focus-visible { outline: 3px solid var(--accent); outline-offset: 3px; }
  .plugin-consent-trigger span { display: grid; gap: 3px; }
  .plugin-consent-trigger strong { color: var(--text); font-size: clamp(14px, .92vw, 17px); }
  .plugin-consent-trigger small { color: var(--muted); font-size: clamp(13px, .86vw, 16px); line-height: 1.5; }
  .plugin-consent .settings-actions { margin-top: 14px; }
  .plugin-confirm { margin-top: 12px; padding: 16px 18px; border: 1px solid color-mix(in srgb, var(--warn) 35%, transparent); border-radius: 13px; background: color-mix(in srgb, var(--warn) 6%, transparent); }
  .plugin-confirm p { margin: 0; color: var(--text-dim); font-size: clamp(13px, .88vw, 16px); line-height: 1.55; }
  .plugin-confirm p.error { margin-top: 8px; color: var(--danger); }
  .plugin-setting { display: grid; grid-template-columns: minmax(0, 1fr) minmax(160px, 0.7fr); align-items: center; gap: 14px; padding: 8px 0; }
  .plugin-setting label { display: grid; gap: 3px; color: var(--text); font-size: clamp(13px, .9vw, 16px); }
  .plugin-setting label span { color: var(--muted); font-size: clamp(12px, .8vw, 14px); line-height: 1.45; }
  .plugin-setting input[type="text"], .plugin-setting input[type="number"], .plugin-setting select { min-height: 42px; padding: 8px 12px; border: 1px solid var(--line); border-radius: 9px; background: var(--surface-2); color: var(--text); font: inherit; }
  .plugin-setting input[type="checkbox"] { appearance: none; -webkit-appearance: none; justify-self: end; flex: none; width: 56px; height: 30px; margin: 0; border: 1px solid #ffffff2b; border-radius: 99px; background: #333947; position: relative; cursor: pointer; transition: background .18s, border-color .18s; }
  .plugin-setting input[type="checkbox"]::after { content: ''; position: absolute; top: 3px; left: 3px; width: 22px; height: 22px; border-radius: 50%; background: #f7f8fc; box-shadow: 0 2px 6px #0006; transition: transform .18s; }
  .plugin-setting input[type="checkbox"]:checked { background: var(--accent); border-color: transparent; }
  .plugin-setting input[type="checkbox"]:checked::after { transform: translateX(26px); }
  .plugin-setting input[type="checkbox"]:focus-visible { outline: 3px solid var(--accent); outline-offset: 4px; }
  .plugin-setting input[type="checkbox"]:disabled { cursor: default; opacity: .45; }
  .plugin-setting .plugin-warning { grid-column: 1 / -1; }
  .plugin-logs { margin-top: 10px; padding-top: 8px; border-top: 1px dashed var(--line); }
  .plugin-logs-head { display: flex; align-items: center; justify-content: space-between; gap: 10px; }
  .plugin-logs-refresh { padding: 4px 10px; border: 1px solid var(--line); border-radius: 7px; background: var(--surface-2); color: var(--text-dim); font: inherit; font-size: 12px; cursor: pointer; }
  .plugin-logs-refresh:disabled { opacity: .5; cursor: default; }
  .plugin-logs-body { margin: 6px 0 0; padding: 8px 10px; border-radius: 8px; background: color-mix(in srgb, var(--surface-2) 85%, transparent); color: var(--text-dim); font-family: ui-monospace, 'Cascadia Code', Consolas, monospace; font-size: 12px; line-height: 1.45; white-space: pre-wrap; overflow-wrap: anywhere; max-height: 160px; overflow-y: auto; }
  .plugin-hash { font-family: ui-monospace, 'Cascadia Code', Consolas, monospace; overflow-wrap: anywhere; }
  .plugin-modal-backdrop { position: fixed; inset: 0; z-index: 200; display: grid; place-items: center; padding: 24px; background: rgba(6, 8, 16, 0.72); backdrop-filter: blur(10px); }
  .plugin-modal { width: min(680px, 100%); max-height: min(82vh, 720px); display: grid; grid-template-rows: auto auto minmax(0, 1fr); gap: 14px; padding: 22px; border-radius: 18px; border: 1px solid var(--line-strong); background: var(--surface-2); color: var(--text); box-shadow: 0 24px 64px rgba(0, 0, 0, 0.55); }
  .plugin-modal-head { display: flex; align-items: center; justify-content: space-between; gap: 14px; }
  .plugin-modal-head h4 { margin: 0; font-size: clamp(18px, 1.3vw, 22px); font-weight: 700; }
  .plugin-modal-sub { margin: 2px 0 0; color: var(--muted); font-size: 13px; }
  .plugin-modal-tabs { display: flex; gap: 8px; flex-wrap: wrap; padding-bottom: 10px; border-bottom: 1px solid var(--line); }
  .plugin-modal-tab { padding: 8px 14px; border-radius: 999px; border: 1px solid var(--line); background: transparent; color: var(--text-dim); font: inherit; font-size: 14px; font-weight: 600; cursor: pointer; }
  .plugin-modal-tab:hover { color: var(--text); border-color: var(--line-strong); }
  .plugin-modal-tab.is-active { background: var(--accent); border-color: transparent; color: #fff; }
  .plugin-modal-tab:focus-visible { outline: 3px solid var(--accent); outline-offset: 2px; }
  .plugin-modal-body { overflow-y: auto; padding-right: 4px; }
  .plugin-modal-settings { display: grid; gap: 8px; }
  .plugin-modal-details { display: grid; gap: 8px; color: var(--text-dim); font-size: 14px; }
  .plugin-modal-details p { margin: 0; }
  @media (max-width: 640px) { .plugin-setting { grid-template-columns: 1fr; } }
</style>
