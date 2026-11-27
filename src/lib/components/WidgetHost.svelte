<script lang="ts">
  import { tick } from 'svelte';
  import { widgetAvailable, widgetDefinition, type WidgetDefinition } from '../services/widgets';
  import { visibleInContext, widgetInstances } from '../services/widgetModel';
  import type { Settings, WidgetConfig, WidgetContext, WidgetPlacement } from '../types';

  type Item = { id: string; config: WidgetConfig; definition: WidgetDefinition };
  type Point = { x: number; y: number };

  let { widgets, context = 'home', onopen = () => {}, onposition = () => {}, interactive = true, contained = false, preview = false, selected = null, onselect = (() => {}) }: {
    widgets: Settings['widgets'];
    context?: WidgetContext;
    onopen?: () => void;
    onposition?: (id: string, position: Point) => void;
    interactive?: boolean;
    contained?: boolean;
    preview?: boolean;
    selected?: string | null;
    onselect?: (id: string) => void;
  } = $props();

  let moving = $state<string | null>(null);
  let draft = $state<Point>({ x: 1000, y: 1000 });
  let dragging = $state(false);
  let dragView = $state<{ left: number; top: number } | null>(null);
  let trigger: HTMLButtonElement | null = null;
  let suppressClick = false;
  let drag: { id: number; widget: string; button: HTMLButtonElement; x: number; y: number; offsetX: number; offsetY: number; area: DOMRect; width: number; height: number; moved: boolean } | null = null;

  const groupOrder: WidgetPlacement[] = ['top-left', 'top-center', 'top-right', 'rail-left', 'rail-right', 'bottom-left', 'bottom-center', 'bottom-right', 'bar'];

  const items = $derived(widgetInstances(widgets).map(item => ({ id: item.id, config: item.config, definition: widgetDefinition(item.config.type) })).filter((item): item is Item => {
    if (!item.definition) return false;
    if (!item.config.enabled && moving !== item.id) return false;
    if (!visibleInContext(item.config, context)) return false;
    return preview || widgetAvailable(item.config);
  }));

  const floating = $derived(moving && !dragging ? items.find(item => item.id === moving) ?? null : null);
  const free = $derived(items.filter(item => item.id !== floating?.id && item.config.position));
  const groups = $derived(groupOrder.map(placement => ({
    placement,
    items: items.filter(item => item.id !== floating?.id && !item.config.position && item.config.placement === placement),
  })).filter(group => group.items.length));
  const hasBar = $derived(items.some(item => !item.config.position && item.config.placement === 'bar'));
  const barCount = $derived(items.filter(item => !item.config.position && item.config.placement === 'bar').length);

  function position(config: WidgetConfig): Point {
    if (config.position) return config.position;
    const x = config.placement.endsWith('left') || config.placement === 'rail-left' ? 0 : config.placement.includes('center') || config.placement === 'bar' ? 500 : 1000;
    const y = config.placement.startsWith('top') ? 0 : config.placement.startsWith('bottom') || config.placement === 'bar' ? 1000 : 500;
    return { x, y };
  }
  function clamp(value: number) { return Math.round(Math.max(0, Math.min(1000, value))); }
  function begin(id: string, button: HTMLButtonElement) {
    trigger = button;
    moving = id;
    draft = { ...position(widgets[id]) };
  }
  async function finish(save: boolean) {
    const id = moving;
    if (id && save) onposition(id, { ...draft });
    moving = null;
    dragging = false;
    drag = null;
    dragView = null;
    await tick();
    const target = id ? document.querySelector<HTMLButtonElement>(`[data-widget-id="${CSS.escape(id)}"] .widget-move`) ?? trigger : trigger;
    target?.focus({ preventScroll: true });
  }
  export function cancelMove() {
    if (!moving) return false;
    void finish(false);
    return true;
  }
  function startDrag(event: PointerEvent, id: string) {
    if (event.button !== 0 || !interactive || preview) return;
    if (widgets[id]?.placement === 'bar' && !widgets[id]?.position) return;
    const button = event.currentTarget as HTMLButtonElement;
    const frame = button.closest('.widget-frame')!.getBoundingClientRect();
    const area = button.closest('.widget-layer')!.getBoundingClientRect();
    suppressClick = false;
    button.setPointerCapture(event.pointerId);
    drag = { id: event.pointerId, widget: id, button, x: event.clientX, y: event.clientY, offsetX: event.clientX - frame.left, offsetY: event.clientY - frame.top, area, width: 1, height: 1, moved: false };
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
      current.offsetX = Math.max(0, Math.min(frame.width, event.clientX - frame.left));
      current.offsetY = Math.max(0, Math.min(frame.height, current.offsetY));
    }
    dragView = { left: event.clientX - current.offsetX, top: event.clientY - current.offsetY };
    draft = { x: clamp((event.clientX - current.offsetX - current.area.left) / current.width * 1000), y: clamp((event.clientY - current.offsetY - current.area.top) / current.height * 1000) };
  }
  function endDrag(event: PointerEvent) {
    if (!drag || drag.id !== event.pointerId) return;
    const moved = drag.moved;
    suppressClick = moved;
    if (moved) draft = { x: draft.x < 40 ? 0 : draft.x > 960 ? 1000 : draft.x, y: draft.y < 40 ? 0 : draft.y > 960 ? 1000 : draft.y };
    drag = null;
    dragging = false;
    dragView = null;
    if (moved) void finish(true);
  }
  async function openMovement(id: string, button: HTMLButtonElement) {
    if (suppressClick) { suppressClick = false; return; }
    if (!moving) begin(id, button);
    await tick();
    document.querySelector<HTMLButtonElement>(`[data-widget-id="${CSS.escape(id)}"] .widget-directions button`)?.focus();
  }
  function moveKey(event: KeyboardEvent) {
    if (event.key === 'Escape' || event.key === 'Backspace') {
      event.preventDefault();
      event.stopPropagation();
      void finish(false);
    }
  }
</script>

{#snippet frame(item: Item, point: Point | null, fixed = false)}
  {@const Component = item.definition.component}
  <section
    class={['widget-frame', `size-${item.config.size}`, {
      'is-bar': item.config.placement === 'bar' && !point && !fixed,
      'is-free': Boolean(point) && !fixed,
      'is-dragging': fixed,
      'is-moving': moving === item.id,
      'is-selected': selected === item.id,
      'is-preview': preview,
    }]}
    style:--x={point ? `${point.x / 10}%` : undefined}
    style:--y={point ? `${point.y / 10}%` : undefined}
    style:--drag-left={dragView ? `${dragView.left}px` : undefined}
    style:--drag-top={dragView ? `${dragView.top}px` : undefined}
    data-widget-id={item.id}
    aria-label={`${item.definition.name} widget`}
  >
    <div class="widget-content" inert={preview}>
      <Component config={item.config} {context} compact={context === 'music'} moving={moving === item.id} {onopen} />
    </div>
    {#if preview}
      <button class="widget-select" aria-label={`Select ${item.definition.name} widget`} onclick={() => onselect(item.id)}></button>
    {:else if interactive}
      <button class="widget-move" aria-label={`Move ${item.definition.name} widget`} title={item.config.placement === 'bar' && !item.config.position ? 'Select for movement controls' : 'Drag to move, or select for movement controls'} onpointerdown={event => startDrag(event, item.id)} onpointermove={moveDrag} onpointerup={endDrag} onpointercancel={() => void finish(false)} onclick={event => void openMovement(item.id, event.currentTarget)}><span aria-hidden="true">&#x283F;</span>Move</button>
      {#if moving === item.id && !dragging}
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
    {/if}
  </section>
{/snippet}

<div class={['widget-layer', { 'is-contained': contained, 'is-preview': preview, 'has-bar': hasBar }]} style:--bar-count={barCount}>
  {#each groups as group (group.placement)}
    <div class={`widget-group is-${group.placement}`}>
      {#each group.items as item (item.id)}
        {@render frame(item, null, moving === item.id && dragging)}
      {/each}
    </div>
  {/each}
  {#each free as item (item.id)}
    {@render frame(item, item.config.position, moving === item.id && dragging)}
  {/each}
  {#if floating}
    {@render frame(floating, draft)}
  {/if}
</div>
{#if !contained && !preview && hasBar}
  <div class="widget-bar-spacer" style:height={`calc(${barCount} * var(--mini-bar-h) + ${barCount > 1 ? (barCount - 1) * 12 : 0}px)`} aria-hidden="true"></div>
{/if}

<style>
  .widget-layer { position: fixed; inset: calc(var(--topbar-space) + 16px) 16px 16px; z-index: 80; pointer-events: none; }
  .widget-layer.is-contained { position: absolute; inset: 0; z-index: 2; }
  .widget-group { position: absolute; display: flex; gap: 12px; pointer-events: none; }
  .widget-group.is-top-left { top: 0; left: 0; flex-direction: column; align-items: flex-start; }
  .widget-group.is-top-center { top: 0; left: 0; right: 0; flex-direction: column; align-items: center; }
  .widget-group.is-top-right { top: 0; right: 0; flex-direction: column; align-items: flex-end; }
  .widget-group.is-bottom-left { bottom: 0; left: 0; flex-direction: column; align-items: flex-start; }
  .widget-group.is-bottom-center { bottom: 0; left: 0; right: 0; flex-direction: column; align-items: center; }
  .widget-group.is-bottom-right { bottom: 0; right: 0; flex-direction: column; align-items: flex-end; }
  .widget-group.is-rail-left { left: 0; top: 0; bottom: 0; flex-direction: column; justify-content: center; align-items: flex-start; }
  .widget-group.is-rail-right { right: 0; top: 0; bottom: 0; flex-direction: column; justify-content: center; align-items: flex-end; }
  .widget-group.is-bar { left: -16px; right: -16px; bottom: -16px; flex-direction: column; align-items: stretch; }
  .widget-layer.is-contained .widget-group.is-bar { left: 0; right: 0; bottom: 0; }
  .widget-layer.has-bar .widget-group.is-bottom-left,
  .widget-layer.has-bar .widget-group.is-bottom-center,
  .widget-layer.has-bar .widget-group.is-bottom-right,
  .widget-layer.has-bar .widget-group.is-rail-left,
  .widget-layer.has-bar .widget-group.is-rail-right { bottom: calc(var(--bar-count, 1) * var(--mini-bar-h) + 4px); }
  .widget-layer.is-contained.has-bar .widget-group.is-bottom-left,
  .widget-layer.is-contained.has-bar .widget-group.is-bottom-center,
  .widget-layer.is-contained.has-bar .widget-group.is-bottom-right,
  .widget-layer.is-contained.has-bar .widget-group.is-rail-left,
  .widget-layer.is-contained.has-bar .widget-group.is-rail-right { bottom: calc(var(--bar-count, 1) * var(--mini-bar-h) + 12px); }
  .widget-frame { position: relative; width: var(--widget-width, clamp(260px, 24vw, 360px)); max-width: 100%; pointer-events: auto; }
  .widget-frame.size-small { --widget-width: clamp(180px, 16vw, 240px); }
  .widget-frame.size-medium { --widget-width: clamp(280px, 24vw, 380px); }
  .widget-frame.size-large { --widget-width: clamp(380px, 34vw, 560px); }
  .widget-group.is-bar .widget-frame { width: 100%; }
  .widget-frame.is-free { position: absolute; left: var(--x); top: var(--y); transform: translate(calc(-1 * var(--x)), calc(-1 * var(--y))); }
  .widget-frame.is-dragging { position: fixed; left: var(--drag-left); top: var(--drag-top); transform: none; z-index: 4; }
  .widget-frame.is-selected { outline: 3px solid var(--accent); outline-offset: 4px; border-radius: 18px; }
  .widget-frame.is-preview { cursor: pointer; }
  .widget-frame.is-preview:hover { outline: 2px solid color-mix(in srgb, var(--accent) 55%, transparent); outline-offset: 3px; border-radius: 18px; }
  .widget-content { position: relative; }
  .widget-select { position: absolute; inset: 0; z-index: 2; border: 0; border-radius: inherit; background: transparent; cursor: pointer; }
  .widget-select:focus-visible { outline: 3px solid var(--accent); outline-offset: 4px; }
  .widget-frame :global(.mini-player) { position: relative; inset: auto; max-width: 100%; width: 100%; }
  .widget-frame :global(.mini-player.is-docked) { position: relative; inset: auto; width: 100%; }
  .widget-frame :global(.mini-player-spacer) { display: none; }
  .widget-move { position: absolute; right: 12px; top: 12px; display: flex; align-items: center; justify-content: center; gap: 5px; min-height: 44px; padding: 6px 10px; border: 1px solid var(--line); border-radius: 8px; background: var(--surface-3); color: var(--text-dim); font-size: 12px; font-weight: 600; touch-action: none; cursor: grab; user-select: none; }
  .widget-move:hover { color: var(--text); border-color: var(--line-strong); }
  .widget-move:active { cursor: grabbing; }
  .is-bar .widget-move { top: 50%; right: 12px; transform: translateY(-50%); }
  .widget-frame.is-bar:not(.is-preview) :global(.mini-player) { padding-right: calc(var(--side) + 68px); }
  .widget-movement { position: absolute; right: 0; top: 100%; z-index: 3; display: grid; gap: 8px; width: max-content; padding: 12px; border: 1px solid var(--accent); border-radius: 10px; background: var(--surface-2); color: var(--text); font-size: 14px; }
  .is-bar .widget-movement { bottom: 100%; top: auto; }
  .widget-directions { display: flex; gap: 6px; flex-wrap: wrap; }
  .widget-directions button { min-width: 44px; min-height: 44px; padding: 6px 10px; border-radius: 8px; background: var(--surface-3); color: var(--text); }
  .widget-frame button:focus-visible { outline: 3px solid var(--accent); outline-offset: 2px; }
  .is-moving { z-index: 1; max-height: 100%; overflow-y: auto; border-radius: 18px; }
  .widget-bar-spacer { flex: none; }
</style>
