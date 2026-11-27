<script lang="ts">
  import { widgetAvailable, widgetDefinition } from '../services/widgets';
  import { widgetInstances } from '../services/widgetModel';
  import type { Settings } from '../types';
  let { widgets, onopen, onselect, selected = null, preview = false }: {
    widgets: Settings['widgets'];
    onopen: () => void;
    onselect?: (id: string) => void;
    selected?: string | null;
    preview?: boolean;
  } = $props();
  const items = $derived(widgetInstances(widgets)
    .map(item => ({ id: item.id, config: item.config, definition: widgetDefinition(item.config.type) }))
    .filter(item => item.definition && item.config.enabled && item.config.presence.includes('music') && (preview || widgetAvailable(item.config))));
</script>

{#if items.length}
  <div class="widget-hud" role="group" aria-label="Player widgets">
    {#each items as item (item.id)}
      {@const Component = item.definition!.component}
      <div class="hud-widget" class:is-selected={selected === item.id} data-widget-id={item.id}>
        <div inert={preview}>
          <Component config={item.config} context="music" compact {onopen} />
        </div>
        {#if preview && onselect}
          <button class="hud-select" aria-label={`Select ${item.definition!.name} widget`} onclick={() => onselect(item.id)}></button>
        {/if}
      </div>
    {/each}
  </div>
{/if}

<style>
  .widget-hud { display: flex; align-items: center; justify-content: center; gap: 10px; flex-wrap: wrap; margin-top: clamp(22px, 3vw, 40px); }
  .hud-widget { position: relative; display: flex; align-items: center; min-width: 0; border-radius: 99px; }
  .hud-widget.is-selected { outline: 3px solid var(--accent); outline-offset: 4px; }
  .hud-select { position: absolute; inset: 0; z-index: 2; border: 0; border-radius: inherit; background: transparent; cursor: pointer; }
  .hud-select:focus-visible { outline: 3px solid var(--accent); outline-offset: 4px; }
</style>
