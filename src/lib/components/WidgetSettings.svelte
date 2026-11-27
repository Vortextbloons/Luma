<script lang="ts">
  import { tick } from 'svelte';
  import WidgetStage from './WidgetStage.svelte';
  import type { Settings, WidgetConfig, WidgetContext, WidgetPlacement, WidgetPresence, WidgetSize, WidgetVisibility } from '../types';
  import {
    addWidget, applyPlacementPreset, applyStylePreset, defaultWidgets, placementLabels, placementPresets,
    presenceLabels, removeWidget, resetWidget, sizeLabels, variantLabels, visibleInContext, widgetCatalog,
    widgetInstances, widgetMeta, widgetSummary, type PlacementPreset,
  } from '../services/widgetModel';
  let { settings }: { settings: Settings } = $props();
  let selected = $state<string | null>(null);
  let context = $state<WidgetContext>('home');
  let view = $state<'list' | 'gallery' | 'edit'>('list');
  let editorTab = $state<'position' | 'style' | 'visibility'>('position');
  let confirmation = $state<'remove' | 'reset-all' | null>(null);
  let studio: HTMLElement;
  let listFocus = '[data-add-widget]';
  let note = $state('');

  const all = $derived(widgetInstances(settings.widgets));
  const selectedConfig = $derived(selected ? settings.widgets[selected] ?? null : null);
  const selectedDefinition = $derived(selectedConfig ? widgetMeta(selectedConfig.type) : undefined);
  const selectedIndex = $derived(selected ? all.findIndex(item => item.id === selected) : -1);
  const gallery = $derived(widgetCatalog.map(meta => ({
    meta,
    count: all.filter(item => item.config.type === meta.type).length,
  })));
  const shown = $derived(all.filter(item => item.meta && item.config.enabled && visibleInContext(item.config, context)).length);
  const contexts: { id: WidgetContext; label: string }[] = [
    { id: 'home', label: 'Home' },
    { id: 'music', label: 'Music player' },
    { id: 'idle', label: 'Idle' },
  ];

  async function focusControl(selector: string, fallback = '[data-add-widget]') {
    await tick();
    const target = studio?.querySelector<HTMLElement>(selector) ?? studio?.querySelector<HTMLElement>(fallback);
    target?.focus();
  }

  function showGallery() {
    view = 'gallery';
    note = '';
    listFocus = '[data-add-widget]';
    void focusControl('[data-gallery-choice]:enabled', '[data-widget-back]');
  }

  function showList() {
    view = 'list';
    selected = null;
    confirmation = null;
    void focusControl(listFocus);
  }

  export function consumeBack() {
    if (confirmation) {
      cancelConfirmation();
      return true;
    }
    if (view === 'list') return false;
    note = '';
    showList();
    return true;
  }

  function cancelConfirmation() {
    const target = confirmation;
    confirmation = null;
    void focusControl(target === 'remove' ? '[data-remove-widget]' : '[data-reset-widgets]');
  }

  function ensureContext(id: string) {
    const config = settings.widgets[id];
    if (!config || visibleInContext(config, context)) return;
    if (config.presence.includes('home')) context = 'home';
    else if (config.presence.includes('music')) context = 'music';
    else if (config.presence.includes('idle')) context = 'idle';
  }

  function select(id: string) {
    if (view === 'edit' && selected === id) return;
    selected = id;
    view = 'edit';
    confirmation = null;
    note = '';
    listFocus = `[data-widget-edit="${CSS.escape(id)}"]`;
    ensureContext(id);
    openEditorTab('position');
    void focusControl(selectedDefinition ? '[data-widget-tab="position"]' : '[data-widget-back]');
  }

  function add(type: string) {
    const definition = widgetMeta(type);
    const id = addWidget(settings.widgets, type);
    if (!id || !definition) { note = 'That widget is already at its limit.'; return; }
    select(id);
    note = `${definition.name} added.`;
  }

  function remove(id: string) {
    const name = widgetMeta(settings.widgets[id]?.type ?? '')?.name ?? 'Widget';
    const next = all[selectedIndex + 1] ?? all[selectedIndex - 1];
    removeWidget(settings.widgets, id);
    listFocus = next ? `[data-widget-edit="${CSS.escape(next.id)}"]` : '[data-add-widget]';
    showList();
    note = `${name} removed.`;
  }

  function togglePresence(config: WidgetConfig, presence: WidgetPresence) {
    if (config.presence.includes(presence)) {
      if (config.presence.length === 1) { note = 'Choose another screen first, or turn the widget off.'; return; }
      config.presence = config.presence.filter(item => item !== presence);
    } else {
      config.presence = [...config.presence, presence];
      context = presence;
    }
    if (selected) ensureContext(selected);
    note = '';
  }

  function place(config: WidgetConfig, placement: WidgetPlacement) {
    const definition = widgetMeta(config.type);
    if (!definition?.placements.includes(placement)) return;
    config.placement = placement;
    config.position = null;
    note = '';
  }

  function resize(config: WidgetConfig, size: WidgetSize) {
    const definition = widgetMeta(config.type);
    if (!definition?.sizes.includes(size)) return;
    config.size = size;
    note = '';
  }

  function restyle(config: WidgetConfig, variant: string) {
    const definition = widgetMeta(config.type);
    if (!definition?.variants.includes(variant)) return;
    config.variant = variant;
    note = '';
  }

  function setSetting(config: WidgetConfig, key: string, value: unknown) {
    config.settings = { ...config.settings, [key]: value };
    note = '';
  }

  function moveOrder(id: string, direction: -1 | 1) {
    const items = widgetInstances(settings.widgets);
    items.forEach((item, index) => { item.config.order = index; });
    const index = items.findIndex(item => item.id === id);
    const target = index + direction;
    if (index < 0 || target < 0 || target >= items.length) return;
    const order = items[index].config.order;
    items[index].config.order = items[target].config.order;
    items[target].config.order = order;
    note = `Position ${target + 1} of ${items.length}.`;
    if (target === 0 || target === items.length - 1) {
      void focusControl(target === 0 ? '[data-move-later]' : '[data-move-earlier]');
    }
  }

  function placementPreset(preset: PlacementPreset) {
    applyPlacementPreset(settings.widgets, preset);
    note = `Placement set to ${placementPresets.find(item => item.id === preset)?.label ?? preset}.`;
  }

  function stylePreset(variant: string) {
    applyStylePreset(settings.widgets, variant);
    note = `Look set to ${variantLabels[variant] ?? variant}.`;
  }

  function resetOne(id: string) {
    resetWidget(settings.widgets, id);
    ensureContext(id);
    note = 'Widget reset to defaults.';
  }

  function resetAll() {
    settings.widgets = defaultWidgets();
    context = 'home';
    listFocus = '[data-widget-edit="spotify"]';
    showList();
    note = 'Widgets reset. Now playing is on Home.';
  }
  function openEditorTab(tab: typeof editorTab) {
    editorTab = tab;
    if (tab === 'position' && context === 'music' && selectedConfig) {
      if (selectedConfig.presence.includes('home')) context = 'home';
      else if (selectedConfig.presence.includes('idle')) context = 'idle';
    }
  }

  const editorTabs = [
    { id: 'position', label: 'Position' },
    { id: 'style', label: 'Style' },
    { id: 'visibility', label: 'Visibility' },
  ] as const;
</script>
<section class="settings-group widget-studio" bind:this={studio} aria-label="Widgets">
  <header class="studio-head">
    <div>
      <h3>{view === 'gallery' ? 'Add a widget' : 'Widgets'}</h3>
      {#if view === 'list'}<p class="panel-description">Choose what appears on your screen.</p>{/if}
    </div>
    {#if view === 'list'}
      <button class="panel-secondary is-primary" data-add-widget onclick={showGallery}>Add widget</button>
    {:else}
      <button class="panel-secondary" data-widget-back onclick={() => consumeBack()}>All widgets</button>
    {/if}
  </header>

  {#if note}<p class="studio-status" role="status">{note}</p>{/if}

  {#if view === 'gallery'}
    <div class="gallery">
      {#each gallery as { meta, count } (meta.type)}
        {@const limit = meta.maxInstances != null && count >= meta.maxInstances}
        <article class="gallery-card">
          <span class="widget-glyph" aria-hidden="true">{meta.glyph}</span>
          <div class="widget-copy"><h4>{meta.name}</h4><p>{meta.description}</p></div>
          <button class="panel-secondary" data-gallery-choice disabled={limit} aria-label={limit ? undefined : `Add ${meta.name}`} onclick={() => add(meta.type)}>{limit ? (meta.maxInstances === 1 ? 'Already added' : 'Limit reached') : 'Add'}</button>
        </article>
      {/each}
    </div>
  {:else}
    <div class="studio-grid">
      <section class="studio-preview" aria-label="Screen preview">
        <div class="preview-toolbar">
          <h4>Preview</h4>
          <span>{shown === 0 ? 'No widgets on this screen' : `${shown} shown`}</span>
        </div>
        <div class="studio-tabs" role="group" aria-label="Preview screen">
          {#each contexts as item (item.id)}
            <button aria-pressed={context === item.id} onclick={() => context = item.id}>{item.label}</button>
          {/each}
        </div>
        <WidgetStage widgets={settings.widgets} {context} {selected} onselect={select} />
        {#if selectedConfig && (!selectedConfig.enabled || !visibleInContext(selectedConfig, context))}
          <p class="preview-caption">{selectedConfig.enabled ? 'This widget is hidden on this screen.' : 'This widget is turned off.'}</p>
        {/if}
      </section>

      <div class="studio-panel">
        {#if view === 'list'}
          <h4>Your widgets</h4>
          {#if all.length}
            <div class="widget-list">
              {#each all as item (item.id)}
                {@const name = item.meta?.name ?? item.config.type}
                <button class="widget-list-item settings-row" data-widget-edit={item.id} onclick={() => select(item.id)}>
                  <span class="widget-glyph" aria-hidden="true">{item.meta?.glyph ?? '◇'}</span>
                  <span class="settings-row-copy">
                    <strong>{name}</strong>
                    <span>{item.meta ? (item.config.position && item.config.enabled ? 'Custom position' : widgetSummary(item.config)) : 'Unavailable'}</span>
                  </span>
                  <span class="widget-chevron" aria-hidden="true">›</span>
                </button>
              {/each}
            </div>
          {:else}
            <p class="panel-description">No widgets yet. Add one to get started.</p>
          {/if}
          <details class="widget-options">
            <summary>Options for all widgets</summary>
            {#if all.length}
              <div class="options-field">
                <h5>Placement presets</h5>
                <div class="studio-tabs" role="group" aria-label="Placement presets">
                  {#each placementPresets as preset (preset.id)}
                    <button title={preset.description} onclick={() => placementPreset(preset.id)}>{preset.label}</button>
                  {/each}
                </div>
              </div>
              <div class="options-field">
                <h5>Style presets</h5>
                <div class="studio-tabs" role="group" aria-label="Style presets">
                  {#each ['glass', 'cinematic', 'ticker', 'minimal'] as variant (variant)}
                    <button onclick={() => stylePreset(variant)}>{variantLabels[variant]}</button>
                  {/each}
                </div>
              </div>
            {/if}
            {#if confirmation === 'reset-all'}
              <p class="panel-description">Replace your widgets with the default Now playing bar?</p>
              <div class="studio-actions">
                <button class="panel-secondary" data-confirm-cancel onclick={cancelConfirmation}>Cancel</button>
                <button class="text-link is-danger" onclick={resetAll}>Reset all widgets</button>
              </div>
            {:else}
              <button class="text-link" data-reset-widgets onclick={() => { confirmation = 'reset-all'; void focusControl('[data-confirm-cancel]'); }}>Reset all widgets</button>
            {/if}
          </details>
        {:else if selectedConfig && selected}
          {@const id = selected}
          {@const config = selectedConfig}
          <div class="editor-head">
            <span class="widget-glyph" aria-hidden="true">{selectedDefinition?.glyph ?? '◇'}</span>
            <div class="widget-copy">
              <h4>{selectedDefinition?.name ?? config.type}</h4>
              <p>{selectedDefinition ? 'Changes save as you edit.' : 'Enable its add-on to use this widget.'}</p>
            </div>
          </div>
          {#if selectedDefinition}
            {@const definition = selectedDefinition}
            <div class="studio-tabs editor-tabs" role="group" aria-label="Widget settings" data-settings-grid>
              {#each editorTabs as tab (tab.id)}
                <button data-widget-tab={tab.id} aria-pressed={editorTab === tab.id} onclick={() => openEditorTab(tab.id)}>{tab.label}</button>
              {/each}
            </div>

            {#if editorTab === 'position'}
              <div class="options-field">
                <h5>Screen position</h5>
                {#if context === 'music'}<p class="preview-caption">These positions apply to Home and Idle. The music player places widgets automatically.</p>{/if}
                {#if config.position}<p class="preview-caption">Custom position. Choose a spot below to move it.</p>{/if}
                <div class="placement-grid" role="group" aria-label="Screen position" data-settings-grid>
                  {#each definition.placements as placement (placement)}
                    <button class="placement-choice" aria-pressed={config.placement === placement && !config.position} onclick={() => place(config, placement)}>
                      <span class={`placement-screen is-${placement}`} aria-hidden="true"><i></i></span>
                      {placementLabels[placement]}
                    </button>
                  {/each}
                </div>
              </div>
              <div class="options-field">
                <h5>Size</h5>
                <div class="studio-tabs" role="group" aria-label="Widget size">
                  {#each definition.sizes as size (size)}
                    <button aria-pressed={config.size === size} onclick={() => resize(config, size)}>{sizeLabels[size]}</button>
                  {/each}
                </div>
              </div>
              {#if all.length > 1}
                <div class="options-field">
                  <h5>Widget order</h5>
                  <div class="studio-actions">
                    <button class="panel-secondary" data-move-earlier disabled={selectedIndex <= 0} onclick={() => moveOrder(id, -1)}>Move earlier</button>
                    <button class="panel-secondary" data-move-later disabled={selectedIndex >= all.length - 1} onclick={() => moveOrder(id, 1)}>Move later</button>
                    <span class="preview-caption">{selectedIndex + 1} of {all.length}</span>
                  </div>
                </div>
              {/if}
            {:else if editorTab === 'style'}
              <div class="options-field">
                <h5>Look</h5>
                <div class="studio-tabs" role="group" aria-label="Widget look">
                  {#each definition.variants as variant (variant)}
                    <button aria-pressed={config.variant === variant} onclick={() => restyle(config, variant)}>{variantLabels[variant] ?? variant}</button>
                  {/each}
                </div>
              </div>
              {#each definition.settings as field (field.key)}
                {#if field.type === 'boolean'}
                  <label class="setting-toggle">
                    <span><span>{field.label}</span>{#if field.description}<small>{field.description}</small>{/if}</span>
                    <input type="checkbox" checked={config.settings[field.key] === true} onchange={event => setSetting(config, field.key, event.currentTarget.checked)} />
                  </label>
                {:else}
                  <label class="field">{field.label}
                    <select value={String(config.settings[field.key] ?? '')} onchange={event => setSetting(config, field.key, event.currentTarget.value)}>
                      {#each field.options ?? [] as option (option.value)}
                        <option value={option.value}>{option.label}</option>
                      {/each}
                    </select>
                  </label>
                {/if}
              {/each}
            {:else}
              <label class="setting-toggle">
                <span>Show this widget</span>
                <input type="checkbox" bind:checked={config.enabled} onchange={() => { note = ''; if (config.enabled) ensureContext(id); }} />
              </label>
              <div class="options-field">
                <h5>Show on</h5>
                <p class="preview-caption">Home widgets also appear on the idle screen.</p>
                <div class="studio-tabs" role="group" aria-label="Show on">
                  {#each definition.contexts as presence (presence)}
                    <button aria-pressed={config.presence.includes(presence)} onclick={() => togglePresence(config, presence)}>{presenceLabels[presence]}</button>
                  {/each}
                </div>
              </div>
              {#if definition.conditions.length > 1}
                <label class="field">Show when
                  <select value={config.visibility} onchange={event => config.visibility = event.currentTarget.value as WidgetVisibility}>
                    {#each definition.conditions as condition (condition.value)}
                      <option value={condition.value}>{condition.label}</option>
                    {/each}
                  </select>
                </label>
              {/if}
            {/if}
          {/if}

          <details class="widget-options">
            <summary>Manage widget</summary>
            {#if confirmation === 'remove'}
              <p class="panel-description">Remove {selectedDefinition?.name ?? config.type}?</p>
              <div class="studio-actions">
                <button class="panel-secondary" data-confirm-cancel onclick={cancelConfirmation}>Cancel</button>
                <button class="text-link is-danger" onclick={() => remove(id)}>Remove widget</button>
              </div>
            {:else}
              <div class="studio-actions">
                {#if selectedDefinition}<button class="panel-secondary" onclick={() => resetOne(id)}>Reset widget</button>{/if}
                <button class="text-link is-danger" data-remove-widget onclick={() => { confirmation = 'remove'; void focusControl('[data-confirm-cancel]'); }}>Remove widget</button>
              </div>
            {/if}
          </details>
        {:else}
          <p class="panel-description">This widget is no longer available.</p>
        {/if}
      </div>
    </div>
  {/if}
</section>

<style>
  .widget-studio { container-type: inline-size; }
  .studio-head, .preview-toolbar, .studio-actions { display: flex; align-items: center; gap: 12px; flex-wrap: wrap; }
  .studio-head, .preview-toolbar { justify-content: space-between; }
  .studio-head .panel-description { margin: 0; }
  .studio-head { margin-bottom: 20px; }
  .studio-grid { display: grid; grid-template-columns: minmax(0, 1.15fr) minmax(0, 1fr); gap: clamp(18px, 2vw, 32px); align-items: start; }
  .studio-panel, .studio-preview, .widget-list, .options-field, .gallery { display: grid; gap: 10px; min-width: 0; }
  .studio-panel { gap: 16px; }
  .studio-preview { position: sticky; top: 16px; }
  h4, h5 { margin: 0; font-weight: 650; }
  h4 { font-size: clamp(17px, 1.2vw, 23px); }
  h5 { font-size: clamp(14px, .95vw, 18px); }
  .preview-toolbar span, .preview-caption, .widget-copy p { margin: 0; color: var(--text-dim); font-size: clamp(13px, .82vw, 16px); line-height: 1.45; }
  .studio-tabs { display: flex; flex-wrap: wrap; gap: 8px; }
  .studio-tabs button, .placement-choice { min-height: 46px; padding: 8px 12px; color: var(--text); background: var(--surface-2); border: 1px solid var(--line); border-radius: 9px; font-size: clamp(13px, .87vw, 16px); font-weight: 600; }
  .studio-tabs button:hover, .placement-choice:hover, .widget-list-item:hover { background: var(--surface-3); border-color: var(--line-strong); }
  button[aria-pressed='true'] { color: var(--accent-ink); background: var(--accent); border-color: var(--accent); }
  button[aria-pressed='true']:hover { color: var(--accent-ink); background: var(--accent); border-color: var(--accent); }
  .editor-tabs { display: grid; grid-template-columns: repeat(3, minmax(0, 1fr)); }
  .editor-tabs button { padding-inline: 4px; }
  .widget-list .widget-list-item { justify-content: initial; flex-direction: row; align-items: center; gap: 12px; min-height: 64px; text-align: left; }
  .widget-list-item .settings-row-copy { flex: 1; }
  .widget-chevron { color: var(--accent); font-size: 24px; }
  .widget-glyph { display: grid; place-items: center; width: 42px; height: 42px; flex: none; border-radius: 10px; background: var(--accent-soft); color: var(--text); font-size: 22px; }
  .gallery { grid-template-columns: repeat(auto-fit, minmax(min(100%, 270px), 1fr)); gap: 12px; }
  .gallery-card { display: grid; grid-template-columns: auto 1fr; align-items: start; gap: 14px; padding: 16px; border: 1px solid var(--line); border-radius: 12px; background: var(--surface-2); }
  .gallery-card button { grid-column: 1 / -1; justify-self: start; }
  .editor-head { display: flex; align-items: center; gap: 12px; }
  .widget-copy { display: grid; gap: 3px; min-width: 0; overflow-wrap: anywhere; }
  .widget-options { border-top: 1px solid var(--line); }
  .widget-options summary { min-height: 48px; padding: 14px 4px; cursor: pointer; font-weight: 600; }
  .widget-options[open] > :not(summary) { margin: 0 0 12px; }
  .studio-panel .field { margin-top: 0; }
  .studio-status { margin: 0 0 16px; padding: 10px 14px; border-left: 3px solid var(--accent); border-radius: 4px; background: var(--accent-soft); color: var(--text); }
  .placement-grid { display: grid; grid-template-columns: repeat(3, minmax(0, 1fr)); gap: 8px; }
  .placement-choice { display: grid; justify-items: center; gap: 5px; padding: 7px 4px; }
  .placement-screen { position: relative; display: block; width: min(100%, 54px); height: 28px; border: 1px solid var(--line-strong); border-radius: 4px; background: var(--surface); }
  .placement-screen i { position: absolute; border-radius: 2px; background: var(--accent); }
  .placement-screen.is-bar i { left: 8%; right: 8%; bottom: 8%; height: 22%; }
  .placement-screen.is-top-left i { top: 10%; left: 9%; width: 30%; height: 16%; }
  .placement-screen.is-top-center i { top: 10%; left: 35%; width: 30%; height: 16%; }
  .placement-screen.is-top-right i { top: 10%; right: 9%; width: 30%; height: 16%; }
  .placement-screen.is-bottom-left i { left: 9%; bottom: 10%; width: 30%; height: 16%; }
  .placement-screen.is-bottom-center i { left: 35%; bottom: 10%; width: 30%; height: 16%; }
  .placement-screen.is-bottom-right i { right: 9%; bottom: 10%; width: 30%; height: 16%; }
  .placement-screen.is-rail-left i { left: 9%; top: 20%; bottom: 20%; width: 18%; }
  .placement-screen.is-rail-right i { right: 9%; top: 20%; bottom: 20%; width: 18%; }
  @container (max-width: 700px) {
    .studio-grid { grid-template-columns: minmax(0, 1fr); }
    .studio-preview { position: static; }
  }
</style>
