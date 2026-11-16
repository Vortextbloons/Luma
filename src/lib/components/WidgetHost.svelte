<script lang="ts">
  import { tick } from 'svelte';
  import { widgetDefinitions } from '../services/widgets';
  import type { Settings, WidgetConfig, WidgetId } from '../types';
  let { widgets, onopen, onposition }: {
    widgets: Settings['widgets'];
    onopen: () => void;
    onposition: (id: WidgetId, position: WidgetConfig['position']) => void;
  } = $props();
  let moving = $state<WidgetId | null>(null);
  let draft = $state({ x: 1000, y: 1000 });
  let dragging = $state(false);
  let trigger: HTMLButtonElement | null = null;
  let suppressClick = false;
  let drag: { id: number; x: number; y: number; left: number; top: number; width: number; height: number; moved: boolean } | null = null;
  const active = $derived(widgetDefinitions.filter(widget => widgets[widget.id]?.enabled && (moving === widget.id || widget.isVisible(widgets[widget.id].visibility))));
  function position(config: WidgetConfig) {
    return config.position ?? { x: config.placement.endsWith('left') ? 0 : 1000, y: config.placement.startsWith('top') ? 0 : 1000 };
  }
  function clamp(value: number) { return Math.round(Math.max(0, Math.min(1000, value))); }
  function begin(id: WidgetId, button: HTMLButtonElement) {
    trigger = button;
    moving = id;
    draft = { ...position(widgets[id]) };
  }
  async function finish(save: boolean) {
    if (moving && save) onposition(moving, { ...draft });
    moving = null;
    dragging = false;
    drag = null;
    await tick();
    trigger?.focus({ preventScroll: true });
  }
  export function cancelMove() {
    if (!moving) return false;
    void finish(false);
    return true;
  }
  async function startDrag(event: PointerEvent, id: WidgetId) {
    if (event.button !== 0) return;
    const button = event.currentTarget as HTMLButtonElement;
    begin(id, button);
    dragging = true;
    suppressClick = false;
    button.setPointerCapture(event.pointerId);
    await tick();
    const frame = button.closest('.widget-frame')!.getBoundingClientRect();
    const area = button.closest('.widget-layer')!.getBoundingClientRect();
    drag = { id: event.pointerId, x: event.clientX, y: event.clientY, left: frame.left - area.left, top: frame.top - area.top, width: Math.max(1, area.width - frame.width), height: Math.max(1, area.height - frame.height), moved: false };
  }
  function moveDrag(event: PointerEvent) {
    if (!drag || drag.id !== event.pointerId) return;
    const dx = event.clientX - drag.x;
    const dy = event.clientY - drag.y;
    if (Math.abs(dx) + Math.abs(dy) > 4) drag.moved = true;
    if (drag.moved) draft = { x: clamp((drag.left + dx) / drag.width * 1000), y: clamp((drag.top + dy) / drag.height * 1000) };
  }
  function endDrag(event: PointerEvent) {
    if (!drag || drag.id !== event.pointerId) return;
    const moved = drag.moved;
    suppressClick = moved;
    if (moved) draft = { x: draft.x < 40 ? 0 : draft.x > 960 ? 1000 : draft.x, y: draft.y < 40 ? 0 : draft.y > 960 ? 1000 : draft.y };
    drag = null;
    dragging = false;
    if (moved) void finish(true);
  }
  async function openMovement(id: WidgetId, button: HTMLButtonElement) {
    if (suppressClick) { suppressClick = false; return; }
    if (!moving) begin(id, button);
    await tick();
    button.closest('.widget-frame')?.querySelector<HTMLButtonElement>('.widget-directions button')?.focus();
  }
  function moveKey(event: KeyboardEvent) {
    if (event.key === 'Escape' || event.key === 'Backspace') {
      event.preventDefault();
      event.stopPropagation();
      void finish(false);
    }
  }
</script>

<div class="widget-layer">
  {#each active as widget (widget.id)}
    {@const config = widgets[widget.id]}
    {@const point = moving === widget.id ? draft : position(config)}
    {@const bar = config.placement === 'bar' && !config.position && moving !== widget.id}
    {@const Component = widget.component}
    <section class={['widget-frame', { 'is-bar': bar, 'is-moving': moving === widget.id }]} style:--x={`${point.x / 10}%`} style:--y={`${point.y / 10}%`} aria-label={`${widget.name} widget`}>
      <Component placement={bar ? 'bar' : config.placement === 'bar' ? 'bottom-right' : config.placement} {onopen} />
      <button class="widget-move" aria-label={`Move ${widget.name} widget`} title="Drag to move, or select for movement controls" onpointerdown={event => startDrag(event, widget.id)} onpointermove={moveDrag} onpointerup={endDrag} onpointercancel={() => void finish(false)} onclick={event => void openMovement(widget.id, event.currentTarget)}>Move</button>
      {#if moving === widget.id && !dragging}
        <div class="widget-movement" role="group" aria-label="Move widget">
          <span>Move widget</span>
          <div class="widget-directions">
            <button onkeydown={moveKey} aria-label="Move left" onclick={() => draft.x = clamp(draft.x - 50)}>&larr;</button>
            <button onkeydown={moveKey} aria-label="Move up" onclick={() => draft.y = clamp(draft.y - 50)}>&uarr;</button>
            <button onkeydown={moveKey} aria-label="Move down" onclick={() => draft.y = clamp(draft.y + 50)}>&darr;</button>
            <button onkeydown={moveKey} aria-label="Move right" onclick={() => draft.x = clamp(draft.x + 50)}>&rarr;</button>
          </div>
          <div class="widget-directions"><button onkeydown={moveKey} onclick={() => void finish(true)}>Done</button><button onkeydown={moveKey} onclick={() => void finish(false)}>Cancel</button></div>
        </div>
      {/if}
    </section>
  {/each}
</div>
{#if active.some(widget => widgets[widget.id].placement === 'bar' && !widgets[widget.id].position)}<div class="mini-player-spacer" aria-hidden="true"></div>{/if}

<style>
  .widget-layer { position: fixed; inset: calc(var(--topbar-space) + 12px) 12px 12px 62px; z-index: 80; pointer-events: none; }
  .widget-frame { position: absolute; left: var(--x); top: var(--y); transform: translate(calc(-1 * var(--x)), calc(-1 * var(--y))); max-width: 100%; pointer-events: auto; }
  .widget-frame.is-bar { position: fixed; inset: auto 0 0; transform: none; max-width: none; }
  .widget-frame :global(.mini-player) { position: relative; inset: auto; max-width: 100%; }
  .widget-frame :global(.mini-player.is-docked) { position: relative; inset: auto; }
  .widget-frame :global(.mini-player-spacer) { display: none; }
  .widget-move { position: absolute; right: 0; bottom: 100%; min-height: 28px; padding: 4px 10px; border: 1px solid var(--line-strong); border-radius: 8px 8px 0 0; background: var(--surface-2); color: var(--text); font-size: 12px; touch-action: none; cursor: grab; opacity: 0; }
  .widget-frame:hover .widget-move, .widget-frame:focus-within .widget-move, .is-moving .widget-move { opacity: 1; }
  .widget-frame:not(.is-bar) .widget-move { bottom: auto; top: 0; right: 100%; border-radius: 8px 0 0 8px; }
  .widget-movement { display: grid; gap: 8px; padding: 12px; border: 1px solid var(--accent); border-radius: 10px; background: var(--surface-2); color: var(--text); font-size: 14px; }
  .is-bar .widget-movement { position: absolute; bottom: 100%; right: 0; }
  .widget-directions { display: flex; gap: 6px; }
  .widget-directions button { min-width: 44px; min-height: 40px; padding: 6px 10px; border-radius: 8px; background: var(--surface-3); color: var(--text); }
  .widget-frame button:focus-visible { outline: 3px solid var(--accent); outline-offset: 2px; }
  .is-moving { z-index: 1; }
</style>

