<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import type { PluginRecord, PluginSettingValue, PluginsState } from '../services/plugins';
  import { pluginControlSummary, pluginFieldLabel, pluginSettingValues } from '../services/plugins';

  type ImportResult = { plugin: PluginRecord; alreadyInstalled: boolean; replacedVersion: string | null };

  let { native, revision, plugins, loading, error, reload, onappearance }: {
    native: boolean;
    revision: number;
    plugins: PluginsState | null;
    loading: boolean;
    error: string;
    reload: () => Promise<void>;
    onappearance?: () => void;
  } = $props();

  let busy = $state('');
  let actionError = $state('');
  let note = $state('');
  let removeTarget = $state<string | null>(null);
  let root = $state<HTMLElement>();

  const list = $derived(plugins?.plugins ?? []);
  const enabledCount = $derived(list.filter(plugin => plugin.enabled).length);

  function messageOf(value: unknown) {
    return value && typeof value === 'object' && 'message' in value ? String(value.message) : String(value);
  }

  function bytesLabel(bytes: number) {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${Math.round(bytes / 1024)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  }

  function sourceLabel(source: string) {
    return source === 'mcp' ? 'Added by AI' : 'Imported';
  }

  function focusPlugin(id: string) {
    requestAnimationFrame(() => root?.querySelector<HTMLElement>(`[data-plugin="${id}"]`)?.focus());
  }

  function focusImport() {
    requestAnimationFrame(() => root?.querySelector<HTMLElement>('[data-plugin-import]')?.focus());
  }

  function focusRemoveCancel() {
    requestAnimationFrame(() => root?.querySelector<HTMLElement>('[data-plugin-remove-cancel]')?.focus());
  }

  function attachCancel(node: HTMLElement) {
    requestAnimationFrame(() => { if (node.isConnected) node.focus(); });
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
          ? `${result.plugin.name} updated to ${result.plugin.version}. It is disabled until you review it.`
          : `${result.plugin.name} imported. It is disabled until you preview and enable it.`;
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

  async function disablePlugin(record: PluginRecord) {
    if (!native || busy) return;
    busy = record.id;
    await run(`${record.name} disabled. Its saved settings are kept.`, async () => {
      await invoke('customization_plugin_disable', { pluginId: record.id, expectedRevision: revision });
      await reload();
    });
    busy = '';
  }

  function askRemove(record: PluginRecord) {
    removeTarget = record.id;
    actionError = '';
    note = '';
    focusRemoveCancel();
  }

  function cancelRemove() {
    const target = removeTarget;
    removeTarget = null;
    if (target) focusPlugin(target);
  }

  async function removePlugin(record: PluginRecord) {
    if (!native || busy) return;
    busy = record.id;
    await run(`${record.name} removed. Appearance and Home use your saved settings.`, async () => {
      await invoke('customization_plugin_remove', { pluginId: record.id, expectedRevision: revision });
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

<section class="settings-group plugins-panel" {@attach mountRoot} aria-busy={loading || !!busy}>
  <div class="settings-group-head">
    <h3>Installed plugins</h3>
    {#if native && list.length}<span class="settings-badge is-muted">{enabledCount} of {list.length} on</span>{/if}
  </div>
  <p class="panel-description">Plugins add appearance, Home layout, and declared settings. Appearance and Home are the built-in plugin, available under Theme, Home layout, and Widgets. Imported plugins start disabled; preview one and press Keep to enable it.</p>
  {#if onappearance}<div class="settings-actions"><button class="panel-secondary" onclick={onappearance}>Open built-in appearance</button></div>{/if}
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
      <p class="panel-description">No plugins yet. Import a .luma-extension.zip, or ask an AI to stage one through Luma MCP. Plugins never run scripts or HTML.</p>
    {:else}
      <div class="plugin-list">
        {#each list as record (record.id)}
          <article class={['plugin-card', { 'is-disabled': !record.enabled }]} data-plugin={record.id} tabindex="-1">
            <header class="plugin-head">
              <div class="plugin-title">
                <strong>{record.name}</strong>
                <span class="plugin-version">v{record.version}</span>
                {#if record.enabled}<span class="settings-badge is-good">Enabled</span>{:else}<span class="settings-badge is-muted">Disabled</span>{/if}
                {#if record.executable}<span class="settings-badge is-warn">Runtime not supported</span>{/if}
              </div>
              <span class="plugin-origin">{sourceLabel(record.source)}</span>
            </header>
            {#if record.description}<p class="plugin-description">{record.description}</p>{/if}
            <p class="plugin-status">{pluginControlSummary(record)}</p>
            {#if record.executionReason}<p class="plugin-warning" role="status">{record.executionReason}</p>{/if}
            {#if record.disabledReason}<p class="plugin-warning" role="status">{record.disabledReason}</p>{/if}
            {#if record.detachedFields.length}<p class="plugin-warning" role="status">You changed {record.detachedFields.map(pluginFieldLabel).join(', ')} in Luma, so this plugin no longer controls {record.detachedFields.length === 1 ? 'that setting' : 'those settings'}.</p>{/if}
            {#if record.conflicts.length}<p class="plugin-warning" role="status">Conflicts with another plugin: {record.conflicts.join(', ')}.</p>{/if}
            {#if record.enableError && record.enableError !== record.executionReason}<p class="plugin-warning" role="status">{record.enableError}</p>{/if}
            <div class="settings-actions plugin-actions">
              {#if record.enabled}
                {#if record.detachedFields.length}
                  <button class="panel-secondary is-primary" disabled={!!busy} onclick={() => previewPlugin(record)}>{busy === record.id ? 'Opening…' : 'Preview and restore controls'}</button>
                {/if}
                <button class="panel-secondary" disabled={!!busy} onclick={() => disablePlugin(record)}>{busy === record.id ? 'Working…' : 'Disable'}</button>
              {:else}
                <button class="panel-secondary is-primary" disabled={!!busy || !record.canEnable} onclick={() => previewPlugin(record)}>{busy === record.id ? 'Opening…' : 'Preview and enable'}</button>
              {/if}
              <button class="panel-secondary" disabled={!!busy} onclick={() => exportPlugin(record)}>Export</button>
              <button class="text-link is-danger" disabled={!!busy} onclick={() => askRemove(record)}>Remove</button>
            </div>
            {#if removeTarget === record.id}
              <div class="plugin-confirm" role="group" aria-label={`Confirm removing ${record.name}`}>
                <p>Remove {record.name}? This deletes its files and saved settings. Appearance and Home return to your saved settings. You can import it again later.</p>
                {#if actionError}<p class="error" role="alert">{actionError}</p>{/if}
                <div class="settings-actions">
                  <button class="panel-secondary" data-plugin-remove-cancel disabled={!!busy} {@attach attachCancel} onclick={cancelRemove}>Cancel</button>
                  <button class="panel-secondary is-primary" disabled={!!busy} onclick={() => removePlugin(record)}>{busy === record.id ? 'Removing…' : 'Remove plugin'}</button>
                </div>
              </div>
            {/if}
            {#if record.settings.length}
              <div class="plugin-settings">
                <h4>Settings</h4>
                {#each record.settings as setting (setting.key)}
                  <div class="plugin-setting">
                    <label for={`plugin-${record.id}-${setting.key}`}>
                      <strong>{setting.label}</strong>
                      {#if setting.description}<span>{setting.description}</span>{/if}
                    </label>
                    {#if setting.kind === 'boolean'}
                      <input id={`plugin-${record.id}-${setting.key}`} type="checkbox" checked={setting.value === true} disabled={!!busy} onchange={event => booleanSetting(record, setting.key, event)} />
                    {:else if setting.kind === 'enum'}
                      <select id={`plugin-${record.id}-${setting.key}`} value={String(setting.value)} disabled={!!busy} onchange={event => selectSetting(record, setting.key, event)}>
                        {#each setting.options as option (option.value)}
                          <option value={option.value}>{option.label}</option>
                        {/each}
                      </select>
                    {:else if setting.kind === 'number'}
                      <input id={`plugin-${record.id}-${setting.key}`} type="number" value={Number(setting.value)} min={setting.min ?? undefined} max={setting.max ?? undefined} step={setting.integer ? 1 : 'any'} disabled={!!busy} onchange={event => numberSetting(record, setting.key, event)} />
                    {:else}
                      <input id={`plugin-${record.id}-${setting.key}`} type="text" value={String(setting.value)} maxlength={setting.maxLength} disabled={!!busy} onchange={event => textSetting(record, setting.key, event)} />
                    {/if}
                    {#if !setting.valid}<span class="plugin-warning">The stored value is invalid, so the default is in use. Choose a value and save to repair it.</span>{/if}
                  </div>
                {/each}
              </div>
            {/if}
            <details class="plugin-details">
              <summary>Package details</summary>
              <p>{record.id} · API {record.apiVersion} · {bytesLabel(record.sizeBytes)} · {record.assets.length + record.sourceFiles.length} declared files</p>
              {#if record.author}<p>By {record.author}</p>{/if}
              <p class="plugin-hash">{record.contentHash.slice(0, 16)}…</p>
            </details>
          </article>
        {/each}
      </div>
    {/if}
  {/if}
</section>

<style>
  .plugin-list { display: grid; gap: 14px; margin-top: 16px; }
  .plugin-card { padding: clamp(16px, 1.4vw, 22px); border: 1px solid var(--line); border-radius: 14px; background: color-mix(in srgb, var(--surface) 58%, transparent); }
  .plugin-card:focus-visible { outline: 3px solid var(--accent); outline-offset: 2px; }
  .plugin-card.is-disabled { border-style: dashed; }
  .plugin-head { display: flex; align-items: flex-start; justify-content: space-between; gap: 14px; }
  .plugin-title { display: flex; align-items: center; gap: 10px; flex-wrap: wrap; }
  .plugin-title strong { color: var(--text); font-size: clamp(17px, 1.2vw, 22px); font-weight: 650; letter-spacing: -.015em; }
  .plugin-version { color: var(--muted); font-size: clamp(12px, .82vw, 15px); font-variant-numeric: tabular-nums; }
  .plugin-origin { color: var(--muted); font-size: clamp(12px, .82vw, 15px); white-space: nowrap; }
  .plugin-description { margin: 8px 0 0; color: var(--text-dim); font-size: clamp(13px, .9vw, 16px); line-height: 1.55; }
  .plugin-status { margin: 10px 0 0; color: var(--text-dim); font-size: clamp(13px, .88vw, 16px); }
  .plugin-warning { margin: 8px 0 0; color: var(--warn); font-size: clamp(12px, .82vw, 15px); line-height: 1.5; }
  .plugin-actions { margin-top: 14px; }
  .plugin-confirm { margin-top: 12px; padding: 16px 18px; border: 1px solid color-mix(in srgb, var(--warn) 35%, transparent); border-radius: 13px; background: color-mix(in srgb, var(--warn) 6%, transparent); }
  .plugin-confirm p { margin: 0; color: var(--text-dim); font-size: clamp(13px, .88vw, 16px); line-height: 1.55; }
  .plugin-confirm p.error { margin-top: 8px; color: var(--danger); }
  .plugin-settings { margin-top: 18px; padding-top: 16px; border-top: 1px solid var(--line); }
  .plugin-settings h4 { margin: 0 0 12px; color: var(--text); font-size: clamp(14px, .95vw, 17px); font-weight: 650; }
  .plugin-setting { display: grid; grid-template-columns: minmax(0, 1fr) minmax(160px, 0.7fr); align-items: center; gap: 14px; padding: 8px 0; }
  .plugin-setting label { display: grid; gap: 3px; color: var(--text); font-size: clamp(13px, .9vw, 16px); }
  .plugin-setting label span { color: var(--muted); font-size: clamp(12px, .8vw, 14px); line-height: 1.45; }
  .plugin-setting input[type="text"], .plugin-setting input[type="number"], .plugin-setting select { min-height: 42px; padding: 8px 12px; border: 1px solid var(--line); border-radius: 9px; background: var(--surface-2); color: var(--text); font: inherit; }
  .plugin-setting input[type="checkbox"] { justify-self: start; width: 24px; height: 24px; accent-color: var(--accent); }
  .plugin-setting .plugin-warning { grid-column: 1 / -1; }
  .plugin-details { margin-top: 14px; border-top: 1px solid var(--line); padding-top: 10px; color: var(--muted); font-size: clamp(12px, .8vw, 14px); }
  .plugin-details summary { cursor: pointer; color: var(--text-dim); font-weight: 600; }
  .plugin-details p { margin: 6px 0 0; }
  .plugin-hash { font-family: ui-monospace, 'Cascadia Code', Consolas, monospace; overflow-wrap: anywhere; }
  @media (max-width: 640px) { .plugin-setting { grid-template-columns: 1fr; } }
</style>
