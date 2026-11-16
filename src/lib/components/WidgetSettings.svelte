<script lang="ts">
  import { widgetDefinitions } from '../services/widgets';
  import type { Settings, WidgetPlacement } from '../types';
  let { settings }: { settings: Settings } = $props();
  let note = $state('');
  function reset() {
    settings.widgets.spotify = { enabled: true, placement: 'bar', visibility: 'available', position: null };
    note = 'Widget settings reset. Home rows and theme were not changed.';
  }
  const placementLabel: Record<WidgetPlacement, string> = {
    bar: 'Bottom bar',
    'top-left': 'Top left',
    'top-right': 'Top right',
    'bottom-right': 'Bottom right',
    'bottom-left': 'Bottom left',
  };
</script>

<section class="settings-group">
  <h3>Widgets</h3>
  <p class="panel-description">Choose a preset or use Move on the widget to drag it or position it with your remote. Custom positions stay within the screen when its size changes.</p>
  <div class="widget-list">
    {#each widgetDefinitions as widget (widget.id)}
      {@const config = settings.widgets[widget.id]}
      <article class="widget-card">
        <div class="widget-head">
          <div class="widget-title"><strong>{widget.name}</strong><span>{widget.description}</span></div>
          <label class="widget-switch"><span>Show</span><input type="checkbox" bind:checked={config.enabled} onchange={() => note = ''} /></label>
        </div>
        {#if config.enabled}
          <label class="widget-visibility">Show widget
            <select bind:value={config.visibility} onchange={() => note = ''}>
              <option value="always">Always</option>
              <option value="available">When Spotify is available</option>
              <option value="playing">Only during playback</option>
            </select>
          </label>
          {#if config.position}<p class="panel-description">Custom position. Choose a preset below to reset it.</p>{/if}
          <div class="widget-placement" role="group" aria-label={`${widget.name} placement`}>
            {#each widget.placements as placement (placement)}
              <button class={['placement-choice', { 'is-active': config.placement === placement && !config.position }]} aria-pressed={config.placement === placement && !config.position} onclick={() => { config.placement = placement; config.position = null; note = ''; }}>
                <span class={`placement-screen is-${placement}`} aria-hidden="true"><i></i></span>
                <span>{placementLabel[placement]}</span>
              </button>
            {/each}
          </div>
        {/if}
      </article>
    {/each}
  </div>
  <div class="settings-actions"><button class="panel-secondary" onclick={reset}>Reset widgets</button></div>
  {#if note}<p class="panel-description" role="status">{note}</p>{/if}
</section>

<style>
  .widget-visibility { display: flex; align-items: center; justify-content: space-between; gap: 16px; margin-top: 16px; color: var(--text-dim); font-size: 14px; }
  .widget-visibility select { padding: 10px 12px; border: 1px solid var(--line); border-radius: 8px; background: var(--surface-2); color: var(--text); }
  .widget-list { display: grid; gap: 12px; }
  .widget-card { padding: 16px 18px; border: 1px solid var(--line); border-radius: 14px; background: color-mix(in srgb, var(--surface) 45%, transparent); }
  .widget-head { display: flex; align-items: center; justify-content: space-between; gap: 20px; }
  .widget-title { display: grid; gap: 3px; min-width: 0; }
  .widget-title strong { font-size: 15px; }
  .widget-title span { color: var(--muted); font-size: 12px; line-height: 1.4; }
  .widget-switch { display: flex; align-items: center; gap: 10px; color: var(--text-dim); font-size: 12px; font-weight: 600; }
  .widget-switch input { width: 20px; height: 20px; accent-color: var(--accent); }
  .widget-placement { display: grid; grid-template-columns: repeat(4, minmax(0, 1fr)); gap: 10px; margin-top: 16px; }
  .placement-choice { display: grid; gap: 8px; justify-items: center; padding: 10px; border: 1px solid var(--line); border-radius: 12px; background: var(--surface-2); color: var(--text-dim); font-size: 11.5px; font-weight: 600; transition: border-color .18s, background-color .18s, color .18s; }
  .placement-choice:hover { border-color: var(--line-strong); background: var(--surface-3); }
  .placement-choice.is-active { border-color: var(--accent); background: var(--accent-soft); color: var(--text); }
  .placement-choice:focus-visible { outline: 3px solid var(--accent); outline-offset: 3px; }
  .placement-screen { position: relative; display: block; width: 100%; aspect-ratio: 16 / 9; border: 1px solid var(--line); border-radius: 6px; background: var(--surface-2); }
  .placement-screen i { position: absolute; display: block; border-radius: 3px; background: var(--accent); }
  .placement-screen.is-bar i { left: 8%; right: 8%; bottom: 8%; height: 22%; border-radius: 4px; }
  .placement-screen.is-top-right i { top: 10%; right: 9%; width: 30%; height: 16%; border-radius: 4px; }
  .placement-screen.is-top-left i { top: 10%; left: 9%; width: 30%; height: 16%; border-radius: 4px; }
  .placement-screen.is-bottom-right i { right: 9%; bottom: 10%; width: 30%; height: 16%; border-radius: 4px; }
  .placement-screen.is-bottom-left i { left: 9%; bottom: 10%; width: 30%; height: 16%; border-radius: 4px; }
  @media (max-width: 720px) { .widget-placement { grid-template-columns: repeat(2, minmax(0, 1fr)); } }
</style>
