<script lang="ts">
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
  let query = $state('');
  let adding = $state(false);
  let note = $state('');

  const all = $derived(widgetInstances(settings.widgets));
  const filtered = $derived(all.length <= 4 ? all : all.filter(item => {
    const needle = query.trim().toLowerCase();
    return !needle || item.meta?.name.toLowerCase().includes(needle) || item.config.type.toLowerCase().includes(needle);
  }));
  const selectedConfig = $derived(selected ? settings.widgets[selected] ?? null : null);
  const selectedDefinition = $derived(selectedConfig ? widgetMeta(selectedConfig.type) : undefined);
  const selectedIndex = $derived(selected ? all.findIndex(item => item.id === selected) : -1);
  const gallery = $derived(widgetCatalog.map(meta => ({
    meta,
    count: all.filter(item => item.config.type === meta.type).length,
  })));
  const contexts: { id: WidgetContext; label: string }[] = [
    { id: 'home', label: 'Home' },
    { id: 'music', label: 'Player' },
    { id: 'idle', label: 'Idle' },
  ];

  $effect(() => {
    const ids = all.map(item => item.id);
    if (selected && ids.includes(selected)) return;
    selected = ids[0] ?? null;
    if (selected) ensureContext(selected);
  });

  $effect(() => {
    if (all.length <= 4 && query) query = '';
  });

  function ensureContext(id: string) {
    const config = settings.widgets[id];
    if (!config || visibleInContext(config, context)) return;
    if (config.presence.includes('home')) context = 'home';
    else if (config.presence.includes('music')) context = 'music';
    else if (config.presence.includes('idle')) context = 'idle';
  }

  function select(id: string) {
    selected = id;
    adding = false;
    note = '';
    ensureContext(id);
  }

  function add(type: string) {
    const definition = widgetMeta(type);
    const id = addWidget(settings.widgets, type);
    if (!id || !definition) { note = 'That widget is already at its limit.'; return; }
    select(id);
    note = `${definition.name} added. Choose where and when it appears.`;
  }

  function remove(id: string) {
    const name = widgetMeta(settings.widgets[id]?.type ?? '')?.name ?? 'Widget';
    removeWidget(settings.widgets, id);
    note = `${name} removed.`;
  }

  function togglePresence(config: WidgetConfig, presence: WidgetPresence) {
    if (config.presence.includes(presence)) {
      if (config.presence.length === 1) { note = 'Keep at least one context selected, or turn the widget off.'; return; }
      config.presence = config.presence.filter(item => item !== presence);
    } else {
      config.presence = [...config.presence, presence];
    }
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
    note = `${widgetMeta(items[index].config.type)?.name ?? 'Widget'} moved ${direction < 0 ? 'earlier' : 'later'} in the stack.`;
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
    note = 'Widget reset to its starting settings.';
  }

  function resetAll() {
    settings.widgets = defaultWidgets();
    selected = 'spotify';
    context = 'home';
    note = 'Widgets reset to the default now-playing bar.';
  }
</script>

<section class="settings-group widget-studio">
  <header class="studio-head">
    <div>
      <h3>Widgets</h3>
      <p class="panel-description">Place widgets around the screen, choose how they look, and decide which moments they belong to. The preview is live; select a widget to edit it.</p>
    </div>
    <button class="panel-secondary" onclick={() => { adding = !adding; note = ''; }}>{adding ? 'Close gallery' : 'Add widget'}</button>
  </header>

  <div class="widget-presets">
    <div class="preset-row">
      <span class="preset-label">Placement</span>
      <div class="preset-options">
        {#each placementPresets as preset (preset.id)}
          <button class="preset-button" title={preset.description} onclick={() => placementPreset(preset.id)}>{preset.label}</button>
        {/each}
      </div>
    </div>
    <div class="preset-row">
      <span class="preset-label">Look</span>
      <div class="preset-options">
        {#each ['glass', 'cinematic', 'ticker', 'minimal'] as variant (variant)}
          <button class="preset-button" onclick={() => stylePreset(variant)}>{variantLabels[variant]}</button>
        {/each}
      </div>
    </div>
  </div>

  <div class="studio-grid">
    <div class="studio-preview">
      <div class="preview-toolbar">
        <div class="context-switch" role="group" aria-label="Preview context">
          {#each contexts as item (item.id)}
            <button class:is-active={context === item.id} aria-pressed={context === item.id} onclick={() => context = item.id}>{item.label}</button>
          {/each}
        </div>
        <span class="preview-count">{all.filter(item => item.config.enabled).length} active</span>
      </div>
      <WidgetStage widgets={settings.widgets} {context} {selected} onselect={select} />
      <p class="preview-note">Select a widget in the preview or the list. Home shows everyday widgets; Player shows what stays visible during music; Idle shows the calmer, hands-off layout.</p>
    </div>

    <div class="studio-panel">
      <div class="widget-list-head"><strong>Your widgets</strong><span>{all.length}</span></div>
      {#if all.length > 4}
        <label class="widget-search"><span class="sr-only">Search widgets</span><input type="search" bind:value={query} placeholder="Search widgets" /></label>
      {/if}
      {#if filtered.length}
        <div class="widget-list">
          {#each filtered as item (item.id)}
            <div class="widget-row" class:is-selected={selected === item.id} class:is-off={!item.config.enabled}>
              <button class="widget-row-main" onclick={() => select(item.id)}>
                <span class="widget-glyph" aria-hidden="true">{item.meta?.glyph ?? '◇'}</span>
                <span class="widget-row-copy"><strong>{item.meta?.name ?? item.config.type}</strong><small>{widgetSummary(item.config)}</small></span>
              </button>
              <label class="widget-switch"><span class="sr-only">Show {item.meta?.name ?? item.config.type}</span><input type="checkbox" bind:checked={item.config.enabled} onchange={() => note = ''} /></label>
            </div>
          {/each}
        </div>
      {:else if all.length === 0}
        <p class="panel-description">No widgets yet. Add one to start.</p>
      {:else}
        <p class="panel-description">No widgets match “{query}”.</p>
      {/if}

      {#if adding}
        <div class="widget-gallery">
          <div class="gallery-head"><strong>Add a widget</strong><button class="text-link" onclick={() => adding = false}>Close</button></div>
          {#each gallery as entry (entry.meta.type)}
            {@const limit = entry.meta.maxInstances != null && entry.count >= entry.meta.maxInstances}
            <div class="gallery-row">
              <span class="widget-glyph" aria-hidden="true">{entry.meta.glyph}</span>
              <span class="gallery-copy"><strong>{entry.meta.name}</strong><small>{entry.meta.description}</small></span>
              <button class="panel-secondary" disabled={limit} onclick={() => add(entry.meta.type)}>{limit ? 'Added' : 'Add'}</button>
            </div>
          {/each}
        </div>
      {/if}

      {#if selectedConfig && selectedDefinition}
        <article class="widget-editor" aria-label={`${selectedDefinition.name} settings`}>
          <header class="editor-head">
            <span class="widget-glyph" aria-hidden="true">{selectedDefinition.glyph}</span>
            <div class="editor-title"><strong>{selectedDefinition.name}</strong><small>{selectedDefinition.description}</small></div>
            <label class="widget-switch"><span>Show</span><input type="checkbox" bind:checked={selectedConfig.enabled} onchange={() => note = ''} /></label>
          </header>

          <div class="editor-section">
            <span class="editor-label">Where</span>
            <div class="placement-grid" role="group" aria-label="Widget placement">
              {#each selectedDefinition.placements as placement (placement)}
                <button class="placement-choice" class:is-active={selectedConfig.placement === placement && !selectedConfig.position} aria-pressed={selectedConfig.placement === placement && !selectedConfig.position} onclick={() => place(selectedConfig, placement)}>
                  <span class={`placement-screen is-${placement}`} aria-hidden="true"><i></i></span>
                  <span>{placementLabels[placement]}</span>
                </button>
              {/each}
            </div>
            {#if selectedConfig.position}
              <div class="editor-note"><span>Custom position from Move on Home.</span><button class="text-link" onclick={() => { selectedConfig.position = null; note = 'Widget returned to its placement preset.'; }}>Use the preset</button></div>
            {/if}
          </div>

          <div class="editor-section">
            <span class="editor-label">Size</span>
            <div class="choice-row" role="group" aria-label="Widget size">
              {#each selectedDefinition.sizes as size (size)}
                <button class="choice-button" class:is-active={selectedConfig.size === size} aria-pressed={selectedConfig.size === size} onclick={() => resize(selectedConfig, size)}>{sizeLabels[size]}</button>
              {/each}
            </div>
          </div>

          <div class="editor-section">
            <span class="editor-label">Look</span>
            <div class="choice-row" role="group" aria-label="Widget look">
              {#each selectedDefinition.variants as variant (variant)}
                <button class="choice-button" class:is-active={selectedConfig.variant === variant} aria-pressed={selectedConfig.variant === variant} onclick={() => restyle(selectedConfig, variant)}>{variantLabels[variant] ?? variant}</button>
              {/each}
            </div>
          </div>

          <div class="editor-section">
            <span class="editor-label">When</span>
            <div class="context-chips" role="group" aria-label="When the widget is visible">
              {#each selectedDefinition.contexts as presence (presence)}
                <button class:is-active={selectedConfig.presence.includes(presence)} aria-pressed={selectedConfig.presence.includes(presence)} onclick={() => togglePresence(selectedConfig, presence)}>{presenceLabels[presence]}</button>
              {/each}
            </div>
          </div>

          <div class="editor-section">
            <span class="editor-label">Stack order</span>
            <div class="choice-row">
              <button class="choice-button" disabled={selectedIndex <= 0} onclick={() => selected && moveOrder(selected, -1)}>Earlier</button>
              <button class="choice-button" disabled={selectedIndex < 0 || selectedIndex >= all.length - 1} onclick={() => selected && moveOrder(selected, 1)}>Later</button>
              <span class="order-note">{selectedIndex >= 0 ? `Position ${selectedIndex + 1} of ${all.length}` : ''}</span>
            </div>
          </div>

          {#if selectedDefinition.conditions.length > 1}
            <label class="editor-field">Availability
              <select value={selectedConfig.visibility} onchange={event => selectedConfig.visibility = event.currentTarget.value as WidgetVisibility}>
                {#each selectedDefinition.conditions as condition (condition.value)}
                  <option value={condition.value}>{condition.label}</option>
                {/each}
              </select>
            </label>
          {/if}

          {#each selectedDefinition.settings as field (field.key)}
            {#if field.type === 'boolean'}
              <label class="editor-toggle">
                <span class="editor-toggle-copy"><strong>{field.label}</strong>{#if field.description}<small>{field.description}</small>{/if}</span>
                <input type="checkbox" checked={selectedConfig.settings[field.key] === true} onchange={event => setSetting(selectedConfig, field.key, event.currentTarget.checked)} />
              </label>
            {:else}
              <label class="editor-field">{field.label}
                <select value={String(selectedConfig.settings[field.key] ?? '')} onchange={event => setSetting(selectedConfig, field.key, event.currentTarget.value)}>
                  {#each field.options ?? [] as option (option.value)}
                    <option value={option.value}>{option.label}</option>
                  {/each}
                </select>
              </label>
            {/if}
          {/each}

          <div class="editor-actions">
            <button class="panel-secondary" onclick={() => selected && resetOne(selected)}>Reset this widget</button>
            <button class="panel-secondary" onclick={() => selected && remove(selected)}>Remove</button>
          </div>
        </article>
      {:else}
        <p class="panel-description">Choose a widget to edit it, or add one from the gallery.</p>
      {/if}
      {#if selectedConfig && !selectedDefinition}
        <article class="widget-editor" aria-label={`${selectedConfig.type} settings`}>
          <header class="editor-head">
            <span class="widget-glyph" aria-hidden="true">◇</span>
            <div class="editor-title"><strong>{selectedConfig.type}</strong><small>This widget type is unavailable. Enable its plugin or remove the instance.</small></div>
          </header>
          <div class="editor-actions"><button class="panel-secondary" onclick={() => selected && remove(selected)}>Remove</button></div>
        </article>
      {/if}
    </div>
  </div>

  <div class="settings-actions"><button class="panel-secondary" onclick={resetAll}>Reset all widgets</button></div>
  {#if note}<p class="panel-description" role="status">{note}</p>{/if}
</section>

<style>
  .studio-head { display: flex; align-items: flex-start; justify-content: space-between; gap: 20px; }
  .studio-head .panel-description { margin-bottom: 0; }
  .widget-presets { display: grid; gap: 10px; margin: 18px 0 16px; padding: 12px 14px; border: 1px solid var(--line); border-radius: 14px; background: color-mix(in srgb, var(--surface) 42%, transparent); }
  .preset-row { display: flex; align-items: center; gap: 14px; flex-wrap: wrap; }
  .preset-label { min-width: 84px; color: var(--muted); font-size: 11.5px; font-weight: 700; letter-spacing: .08em; text-transform: uppercase; }
  .preset-options { display: flex; gap: 8px; flex-wrap: wrap; }
  .preset-button { min-height: 44px; padding: 8px 14px; border: 1px solid var(--line); border-radius: 99px; background: var(--surface-2); color: var(--text-dim); font-size: 12.5px; font-weight: 600; }
  .preset-button:hover { border-color: var(--line-strong); color: var(--text); background: var(--surface-3); }
  .studio-grid { display: grid; grid-template-columns: minmax(0, 1.15fr) minmax(340px, .85fr); gap: 18px; align-items: start; }
  .studio-preview { position: sticky; top: 12px; display: grid; gap: 10px; }
  .preview-toolbar { display: flex; align-items: center; justify-content: space-between; gap: 12px; flex-wrap: wrap; }
  .context-switch { display: flex; gap: 4px; padding: 4px; border: 1px solid var(--line); border-radius: 99px; background: var(--surface-2); }
  .context-switch button { min-height: 36px; padding: 6px 14px; border-radius: 99px; background: transparent; color: var(--text-dim); font-size: 12.5px; font-weight: 600; }
  .context-switch button.is-active { background: var(--accent-soft); color: var(--text); box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--accent) 45%, transparent); }
  .preview-count { color: var(--muted); font-size: 12.5px; font-weight: 600; }
  .preview-note { color: var(--muted); font-size: 12px; line-height: 1.5; margin: 0; }
  .studio-panel { display: grid; gap: 12px; min-width: 0; }
  .widget-list-head { display: flex; align-items: center; justify-content: space-between; color: var(--text-dim); font-size: 13px; }
  .widget-list-head span { min-width: 26px; padding: 2px 8px; border-radius: 99px; background: var(--surface-3); color: var(--muted); text-align: center; font-size: 11px; font-weight: 700; }
  .widget-search { display: block; }
  .widget-search input { width: 100%; min-height: 44px; padding: 10px 14px; border: 1px solid var(--line); border-radius: 10px; background: var(--surface-2); color: var(--text); }
  .widget-list { display: grid; gap: 6px; max-height: 260px; overflow-y: auto; padding-right: 4px; }
  .widget-row { display: flex; align-items: center; gap: 8px; padding: 6px 8px 6px 6px; border: 1px solid var(--line); border-radius: 12px; background: color-mix(in srgb, var(--surface) 42%, transparent); }
  .widget-row.is-selected { border-color: var(--accent); background: var(--accent-soft); }
  .widget-row.is-off { opacity: .6; }
  .widget-row-main { display: flex; align-items: center; gap: 10px; flex: 1; min-width: 0; padding: 6px; border-radius: 9px; background: transparent; text-align: left; }
  .widget-glyph { display: grid; place-items: center; width: 34px; height: 34px; flex: none; border-radius: 9px; background: var(--surface-3); color: var(--text-dim); font-size: 16px; }
  .widget-row-copy { display: grid; gap: 2px; min-width: 0; }
  .widget-row-copy strong { font-size: 13px; }
  .widget-row-copy small { overflow: hidden; color: var(--muted); font-size: 11.5px; text-overflow: ellipsis; white-space: nowrap; }
  .widget-switch { display: flex; align-items: center; gap: 8px; color: var(--text-dim); font-size: 12.5px; font-weight: 600; }
  .widget-switch input { width: 20px; height: 20px; accent-color: var(--accent); }
  .widget-gallery { display: grid; gap: 8px; padding: 12px; border: 1px solid var(--line); border-radius: 14px; background: color-mix(in srgb, var(--surface) 55%, transparent); }
  .gallery-head { display: flex; align-items: center; justify-content: space-between; font-size: 13px; }
  .gallery-row { display: flex; align-items: center; gap: 10px; }
  .gallery-copy { display: grid; gap: 2px; flex: 1; min-width: 0; }
  .gallery-copy strong { font-size: 13px; }
  .gallery-copy small { color: var(--muted); font-size: 11.5px; line-height: 1.4; }
  .widget-editor { display: grid; gap: 16px; padding: 16px; border: 1px solid var(--line-strong); border-radius: 16px; background: color-mix(in srgb, var(--surface-2) 62%, transparent); }
  .editor-head { display: flex; align-items: center; gap: 10px; }
  .editor-title { display: grid; gap: 2px; flex: 1; min-width: 0; }
  .editor-title strong { font-size: 15px; }
  .editor-title small { color: var(--muted); font-size: 11.5px; line-height: 1.4; }
  .editor-section { display: grid; gap: 8px; }
  .editor-label { color: var(--muted); font-size: 11px; font-weight: 700; letter-spacing: .08em; text-transform: uppercase; }
  .placement-grid { display: grid; grid-template-columns: repeat(3, minmax(0, 1fr)); gap: 8px; }
  .placement-choice { display: grid; gap: 5px; justify-items: center; padding: 7px; border: 1px solid var(--line); border-radius: 10px; background: var(--surface-2); color: var(--text-dim); font-size: 11.5px; font-weight: 600; }
  .placement-choice:hover { border-color: var(--line-strong); background: var(--surface-3); }
  .placement-choice.is-active { border-color: var(--accent); background: var(--accent-soft); color: var(--text); }
  .placement-screen { position: relative; display: block; width: 100%; aspect-ratio: 16 / 9; border: 1px solid var(--line); border-radius: 5px; background: var(--surface-2); }
  .placement-screen i { position: absolute; display: block; border-radius: 3px; background: var(--accent); }
  .placement-screen.is-bar i { left: 8%; right: 8%; bottom: 8%; height: 22%; border-radius: 4px; }
  .placement-screen.is-top-left i { top: 10%; left: 9%; width: 30%; height: 16%; }
  .placement-screen.is-top-center i { top: 10%; left: 35%; width: 30%; height: 16%; }
  .placement-screen.is-top-right i { top: 10%; right: 9%; width: 30%; height: 16%; }
  .placement-screen.is-bottom-left i { left: 9%; bottom: 10%; width: 30%; height: 16%; }
  .placement-screen.is-bottom-center i { left: 35%; bottom: 10%; width: 30%; height: 16%; }
  .placement-screen.is-bottom-right i { right: 9%; bottom: 10%; width: 30%; height: 16%; }
  .placement-screen.is-rail-left i { left: 9%; top: 20%; bottom: 20%; width: 18%; }
  .placement-screen.is-rail-right i { right: 9%; top: 20%; bottom: 20%; width: 18%; }
  .choice-row { display: flex; gap: 6px; flex-wrap: wrap; }
  .choice-button { min-height: 40px; padding: 8px 14px; border: 1px solid var(--line); border-radius: 9px; background: var(--surface-2); color: var(--text-dim); font-size: 12.5px; font-weight: 600; }
  .choice-button.is-active { border-color: var(--accent); background: var(--accent-soft); color: var(--text); }
  .choice-button:disabled { opacity: .45; }
  .order-note { align-self: center; color: var(--muted); font-size: 11.5px; }
  .context-chips { display: flex; gap: 6px; flex-wrap: wrap; }
  .context-chips button { min-height: 40px; padding: 8px 14px; border: 1px solid var(--line); border-radius: 99px; background: var(--surface-2); color: var(--text-dim); font-size: 12.5px; font-weight: 600; }
  .context-chips button.is-active { border-color: var(--accent); background: var(--accent-soft); color: var(--text); }
  .editor-field { display: grid; gap: 6px; color: var(--text-dim); font-size: 12.5px; font-weight: 600; }
  .editor-field select { min-height: 42px; padding: 8px 12px; border: 1px solid var(--line); border-radius: 9px; background: var(--surface-2); color: var(--text); }
  .editor-toggle { display: flex; align-items: center; justify-content: space-between; gap: 16px; }
  .editor-toggle-copy { display: grid; gap: 3px; color: var(--text-dim); font-size: 12.5px; }
  .editor-toggle-copy small { color: var(--muted); font-size: 11.5px; line-height: 1.4; }
  .editor-toggle input { width: 20px; height: 20px; accent-color: var(--accent); }
  .editor-note { display: flex; align-items: center; gap: 10px; flex-wrap: wrap; color: var(--muted); font-size: 11.5px; }
  .editor-actions { display: flex; gap: 8px; flex-wrap: wrap; justify-content: flex-end; }
  .placement-choice:focus-visible, .choice-button:focus-visible, .context-chips button:focus-visible, .preset-button:focus-visible, .context-switch button:focus-visible { outline: 3px solid var(--accent); outline-offset: 3px; }
  .sr-only { position: absolute; width: 1px; height: 1px; padding: 0; margin: -1px; overflow: hidden; clip: rect(0, 0, 0, 0); white-space: nowrap; border: 0; }
  @media (max-width: 1100px) {
    .studio-grid { grid-template-columns: 1fr; }
    .studio-preview { position: static; }
  }
  @media (max-width: 720px) {
    .studio-head { flex-direction: column; }
    .placement-grid { grid-template-columns: repeat(2, minmax(0, 1fr)); }
  }
</style>
