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
  let drag: { id: number; widget: WidgetId; button: HTMLButtonElement; x: number; y: number; right: number; offsetY: number; area: DOMRect; width: number; height: number; offsetX: number; moved: boolean } | null = null;
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
  function startDrag(event: PointerEvent, id: WidgetId) {
    if (event.button !== 0) return;
    const button = event.currentTarget as HTMLButtonElement;
    const frame = button.closest('.widget-frame')!.getBoundingClientRect();
    const area = button.closest('.widget-layer')!.getBoundingClientRect();
    suppressClick = false;
    button.setPointerCapture(event.pointerId);
    drag = { id: event.pointerId, widget: id, button, x: event.clientX, y: event.clientY, right: frame.right - event.clientX, offsetY: event.clientY - frame.top, area, width: 1, height: 1, offsetX: 0, moved: false };
  }
  async function moveDrag(event: PointerEvent) {
    const current = drag;
    if (!current || current.id !== event.pointerId) return;
    if (!current.moved) {
      if (Math.abs(event.clientX - current.x) + Math.abs(event.clientY - current.y) <= 4) return;
      current.moved = true;
      begin(current.widget, current.button);
      dragging = true;
      await tick();
      if (drag !== current) return;
      const frame = current.button.closest('.widget-frame')!.getBoundingClientRect();
      current.width = Math.max(1, current.area.width - frame.width);
      current.height = Math.max(1, current.area.height - frame.height);
      current.offsetX = Math.max(0, Math.min(frame.width, frame.width - current.right));
      current.offsetY = Math.max(0, Math.min(frame.height, current.offsetY));
    }
    draft = { x: clamp((event.clientX - current.area.left - current.offsetX) / current.width * 1000), y: clamp((event.clientY - current.area.top - current.offsetY) / current.height * 1000) };
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
      <button class="widget-move" aria-label={`Move ${widget.name} widget`} title="Drag to move, or select for movement controls" onpointerdown={event => startDrag(event, widget.id)} onpointermove={moveDrag} onpointerup={endDrag} onpointercancel={() => void finish(false)} onclick={event => void openMovement(widget.id, event.currentTarget)}><span aria-hidden="true">&#x283F;</span>Move</button>
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
  .widget-layer { position: fixed; inset: calc(var(--topbar-space) + 16px) 16px 16px; z-index: 80; pointer-events: none; }
  .widget-frame { position: absolute; left: var(--x); top: var(--y); transform: translate(calc(-1 * var(--x)), calc(-1 * var(--y))); max-width: 100%; pointer-events: auto; }
  .widget-frame.is-bar { position: fixed; inset: auto 0 0; transform: none; max-width: none; }
  .widget-frame :global(.mini-player) { position: relative; inset: auto; max-width: 100%; }
  .widget-frame :global(.mini-player.is-docked) { position: relative; inset: auto; }
  .widget-frame :global(.mini-player-spacer) { display: none; }
  .widget-move { position: absolute; right: 12px; top: 12px; display: flex; align-items: center; justify-content: center; gap: 5px; min-height: 44px; padding: 6px 10px; border: 1px solid var(--line); border-radius: 8px; background: var(--surface-3); color: var(--text-dim); font-size: 12px; font-weight: 600; touch-action: none; cursor: grab; user-select: none; }
  .widget-move:hover { color: var(--text); border-color: var(--line-strong); }
  .widget-move:active { cursor: grabbing; }
  .is-bar .widget-move { top: 50%; right: 12px; transform: translateY(-50%); }
  .widget-frame.is-bar :global(.mini-player) { padding-right: calc(var(--side) + 68px); }
  .widget-movement { display: grid; gap: 8px; padding: 12px; border: 1px solid var(--accent); border-radius: 10px; background: var(--surface-2); color: var(--text); font-size: 14px; }
  .is-bar .widget-movement { position: absolute; bottom: 100%; right: 0; }
  .widget-directions { display: flex; gap: 6px; flex-wrap: wrap; }
  .widget-directions button { min-width: 44px; min-height: 44px; padding: 6px 10px; border-radius: 8px; background: var(--surface-3); color: var(--text); }
  .widget-frame button:focus-visible { outline: 3px solid var(--accent); outline-offset: 2px; }
  .is-moving { z-index: 1; max-height: 100%; overflow-y: auto; border-radius: 18px; }
</style>

