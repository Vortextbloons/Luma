<script lang="ts">
  import { tick } from 'svelte';
  import ProviderIcon from './activity/ProviderIcon.svelte';
  import { appProviders } from '../services/activity';
  import { defaultAppOrder } from '../state/settings.svelte';
  import type { AppId, ExecutableAppId, Settings } from '../types';

  let { settings, onchange }: { settings: Settings; onchange: (order: AppId[]) => void } = $props();

  let dragging = $state<AppId | null>(null);
  let overIndex = $state<number | null>(null);
  let overAfter = $state(false);
  let lifted = $state<AppId | null>(null);
  let liftOrigin = $state(0);
  let status = $state('');
  let listElement = $state<HTMLOListElement | null>(null);

  const movable: AppId[] = $derived(settings.order.filter(app => app !== 'Custom'));

  function builtInNote(id: AppId) {
    if (['NBA', 'NFL', 'MLB', 'NHL'].includes(id)) return 'Opens live and upcoming games inside Luma.';
    if (id === 'YouTube') return 'YouTube TV opens in its own Luma window.';
    if (id === 'Steam') return 'Steam opens directly in Big Picture.';
    if (id === 'Stremio') return 'Stremio opens inside Luma.';
    return '';
  }

  function applyOrder(order: AppId[]) {
    onchange([...order, 'Custom']);
  }

  function startDrag(event: DragEvent, app: AppId) {
    if (app === 'Custom') {
      event.preventDefault();
      return;
    }
    dragging = app;
    overIndex = null;
    if (event.dataTransfer) {
      event.dataTransfer.effectAllowed = 'move';
      event.dataTransfer.setData('text/plain', app);
    }
  }

  function dragOver(event: DragEvent, index: number, app: AppId) {
    if (!dragging) return;
    event.preventDefault();
    if (event.dataTransfer) event.dataTransfer.dropEffect = 'move';
    const row = event.currentTarget as HTMLElement;
    const rect = row.getBoundingClientRect();
    overIndex = index;
    overAfter = app === 'Custom' ? false : event.clientY > rect.top + rect.height / 2;
  }

  function drop(event: DragEvent, index: number, app: AppId) {
    event.preventDefault();
    if (!dragging) return;
    const source = movable.indexOf(dragging);
    if (source < 0) {
      endDrag();
      return;
    }
    let target = app === 'Custom' ? movable.length : index + (overAfter ? 1 : 0);
    if (source < target) target -= 1;
    const next = [...movable];
    next.splice(source, 1);
    next.splice(Math.max(0, Math.min(target, next.length)), 0, dragging);
    applyOrder(next);
    status = `${dragging} moved to position ${next.indexOf(dragging) + 1} of ${settings.order.length}.`;
    endDrag();
  }

  function endDrag() {
    dragging = null;
    overIndex = null;
  }

  async function refocus(target: HTMLElement | null, app: AppId) {
    await tick();
    const fallback = listElement?.querySelector<HTMLButtonElement>(`li[data-app="${app}"] .app-order-action`);
    (target?.isConnected ? target : fallback)?.focus({ preventScroll: true });
  }

  function focusedElement() {
    return document.activeElement instanceof HTMLElement ? document.activeElement : null;
  }

  function startLift(app: AppId, index: number) {
    lifted = app;
    liftOrigin = index;
    status = `${app} is ready to move. Use the arrow keys or the buttons, then choose Done.`;
  }

  function move(app: AppId, direction: -1 | 1) {
    if (lifted !== app) return;
    const index = movable.indexOf(app);
    const target = index + direction;
    if (index < 0 || target < 0 || target >= movable.length) {
      status = `${app} is already ${direction < 0 ? 'first' : 'last'} in the dock.`;
      return;
    }
    const focusTarget = focusedElement();
    const next = [...movable];
    [next[index], next[target]] = [next[target], next[index]];
    applyOrder(next);
    status = `${app} moved to position ${target + 1} of ${settings.order.length}.`;
    void refocus(focusTarget, app);
  }

  function commit() {
    if (!lifted) return;
    const app = lifted;
    const index = movable.indexOf(app);
    status = `${app} saved at position ${index + 1} of ${settings.order.length}.`;
    lifted = null;
  }

  function cancel() {
    if (!lifted) return;
    const app = lifted;
    const focusTarget = focusedElement();
    const next = [...movable];
    const from = next.indexOf(app);
    next.splice(from, 1);
    next.splice(Math.max(0, Math.min(liftOrigin, next.length)), 0, app);
    applyOrder(next);
    status = `${app} returned to position ${liftOrigin + 1} of ${settings.order.length}. Move cancelled.`;
    lifted = null;
    void refocus(focusTarget, app);
  }

  function rowKeydown(event: KeyboardEvent, app: AppId) {
    if (lifted !== app) return;
    if (event.key === 'Escape') {
      event.preventDefault();
      event.stopPropagation();
      cancel();
      return;
    }
    if (event.key !== 'ArrowUp' && event.key !== 'ArrowDown') return;
    event.preventDefault();
    event.stopPropagation();
    move(app, event.key === 'ArrowUp' ? -1 : 1);
  }

  function reset() {
    const focusTarget = focusedElement();
    const app = lifted;
    lifted = null;
    onchange(defaultAppOrder());
    status = 'App order reset and saved to the default dock. App locations and accounts were not changed.';
    if (app) void refocus(focusTarget, app);
  }
</script>

<section class="settings-group app-order-settings">
  <div class="app-order-head">
    <div>
      <h3>App order</h3>
      <p class="panel-description">Drag a row to move it, or focus Move and use the arrow keys. This order is shared with Home, the Display Box, and the web remote.</p>
    </div>
    <button class="panel-secondary app-order-reset" onclick={reset}>Reset order</button>
  </div>

  <div class="app-dock-preview" aria-hidden="true">
    <div class="app-dock-label"><span>Dock preview</span><small>Home, Display Box, and web remote</small></div>
    <div class="app-dock-track">
      {#each settings.order as app (app)}
        <span class="app-dock-tile" class:is-lifted={lifted === app}>
          {#if app === 'Custom'}
            <span class="app-dock-plus">＋</span>
          {:else}
            <ProviderIcon provider={appProviders[app]} />
          {/if}
          <span>{app}</span>
        </span>
      {/each}
    </div>
  </div>

  <ol bind:this={listElement} class="app-order-list" aria-label="App order">
    {#each settings.order as app, index (app)}
      <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
      <li
        data-app={app}
        class:is-lifted={lifted === app}
        class:is-dragging={dragging === app}
        class:is-drop-before={Boolean(dragging) && overIndex === index && !overAfter}
        class:is-drop-after={Boolean(dragging) && overIndex === index && overAfter}
        draggable={app !== 'Custom'}
        ondragstart={(event) => startDrag(event, app)}
        ondragover={(event) => dragOver(event, index, app)}
        ondrop={(event) => drop(event, index, app)}
        ondragend={endDrag}
        onkeydown={(event) => rowKeydown(event, app)}
      >
        <span class="app-order-index" aria-hidden="true">{index + 1}</span>
        <span class="app-order-grip" aria-hidden="true" title="Drag to reorder">
          <svg viewBox="0 0 10 16"><circle cx="3" cy="3" r="1.2" /><circle cx="7" cy="3" r="1.2" /><circle cx="3" cy="8" r="1.2" /><circle cx="7" cy="8" r="1.2" /><circle cx="3" cy="13" r="1.2" /><circle cx="7" cy="13" r="1.2" /></svg>
        </span>
        <div class="app-order-main">
          <div class="app-order-title">
            <ProviderIcon provider={appProviders[app]} />
            <strong>{app}</strong>
            {#if app === 'YouTube'}
              <span class="settings-badge is-muted">Own window</span>
            {:else if app === 'Steam'}
              <span class="settings-badge is-muted">Big Picture</span>
            {:else if app === 'Stremio' || ['NBA', 'NFL', 'MLB', 'NHL'].includes(app)}
              <span class="settings-badge is-muted">Inside Luma</span>
            {:else if settings.paths[app as ExecutableAppId]}
              <span class="settings-badge is-good">Ready</span>
            {:else if app === 'Apple TV+' || app === 'Hulu' || app === 'Max' || app === 'Peacock'}
              <span class="settings-badge is-muted">Opens in a window</span>
            {:else}
              <span class="settings-badge is-warn">Choose app</span>
            {/if}
            {#if app === 'Custom'}<span class="app-order-fixed">Fixed last</span>{/if}
          </div>
          {#if app === 'YouTube' || app === 'Steam' || app === 'Stremio' || ['NBA', 'NFL', 'MLB', 'NHL'].includes(app)}
            <p class="app-order-note">{builtInNote(app)}</p>
          {:else if app !== 'Custom' && !settings.paths[app as ExecutableAppId]}
            <p class="app-order-note">{app === 'Apple TV+' || app === 'Hulu' || app === 'Max' || app === 'Peacock' ? 'No program is needed. It opens in its own window.' : 'Choose this app under App locations if you use it.'}</p>
          {/if}
        </div>
        <div class="app-order-controls">
          {#if lifted === app}
            <div class="app-order-nudge">
              <button onclick={() => move(app, -1)} aria-label={`Move ${app} up`}>↑</button>
              <button onclick={() => move(app, 1)} aria-label={`Move ${app} down`}>↓</button>
            </div>
          {/if}
          {#if app !== 'Custom'}
            <button
              class="app-order-action"
              class:is-done={lifted === app}
              aria-label={lifted === app ? `Finish moving ${app}` : `Move ${app}`}
              onclick={() => lifted === app ? commit() : startLift(app, index)}
            >{lifted === app ? 'Done' : 'Move'}</button>
          {/if}
        </div>
      </li>
    {/each}
  </ol>
  <p class="app-order-status" role="status" aria-live="polite" aria-atomic="true">{status}</p>
</section>

<style>
  .app-order-head { display: flex; align-items: flex-start; justify-content: space-between; gap: 20px; margin-bottom: 18px; }
  .app-order-head > div { min-width: 0; }
  .app-order-head .panel-description { margin: 0; }
  .app-order-reset { flex: none; }
  .app-dock-preview { padding: 12px 14px 14px; border: 1px solid var(--line); border-radius: 14px; background: color-mix(in srgb, var(--surface-2) 60%, transparent); margin-bottom: 16px; }
  .app-dock-label { display: flex; align-items: baseline; justify-content: space-between; gap: 12px; margin-bottom: 9px; color: var(--text-dim); font-size: 11px; font-weight: 700; letter-spacing: .08em; text-transform: uppercase; }
  .app-dock-label small { color: var(--dim); font-size: 11px; font-weight: 500; letter-spacing: 0; text-transform: none; }
  .app-dock-track { display: flex; gap: 8px; overflow-x: auto; padding: 2px; scrollbar-width: none; }
  .app-dock-track::-webkit-scrollbar { display: none; }
  .app-dock-tile { display: grid; place-items: center; gap: 5px; flex: 0 0 auto; min-width: 72px; max-width: 92px; padding: 8px 9px; border: 1px solid var(--line); border-radius: 11px; background: var(--surface-3); color: var(--text-dim); font-size: 11px; font-weight: 600; transition: border-color .18s, background-color .18s; }
  .app-dock-tile.is-lifted { border-color: var(--accent); background: var(--accent-soft); color: var(--text); }
  .app-dock-tile > span:last-child { max-width: 100%; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .app-dock-tile :global(.provider-icon) { width: 24px; height: 24px; border-radius: 7px; }
  .app-dock-plus { display: grid; place-items: center; width: 24px; height: 24px; color: var(--muted); font-size: 20px; line-height: 1; }
  .app-order-list { display: grid; gap: 8px; margin: 0; padding: 0; list-style: none; }
  .app-order-list > li { position: relative; display: grid; grid-template-columns: auto auto minmax(0, 1fr) auto; align-items: center; gap: 12px; padding: 12px 14px; border: 1px solid var(--line); border-radius: 12px; background: #ffffff04; cursor: grab; transition: background-color var(--motion-fast) var(--ease-out), border-color var(--motion-fast) var(--ease-out), opacity var(--motion-fast) var(--ease-out); }
  .app-order-list > li:hover { background: #ffffff07; border-color: var(--line-strong); }
  .app-order-list > li.is-lifted { border-color: var(--accent); background: var(--accent-soft); cursor: grabbing; }
  .app-order-list > li.is-dragging { opacity: .4; }
  .app-order-list > li.is-drop-before::before, .app-order-list > li.is-drop-after::after { content: ''; position: absolute; left: 10px; right: 10px; height: 2px; border-radius: 99px; background: var(--accent); box-shadow: 0 0 10px var(--accent); }
  .app-order-list > li.is-drop-before::before { top: -5px; }
  .app-order-list > li.is-drop-after::after { bottom: -5px; }
  .app-order-list > li[draggable='false'] { cursor: default; }
  .app-order-index { display: grid; place-items: center; width: 28px; height: 28px; border: 1px solid var(--line); border-radius: 8px; background: #ffffff08; color: var(--muted); font-size: 12px; font-weight: 700; font-variant-numeric: tabular-nums; }
  .app-order-grip { display: grid; place-items: center; width: 20px; color: var(--dim); }
  .app-order-grip svg { width: 10px; height: 16px; fill: currentColor; }
  .app-order-main { display: grid; gap: 4px; min-width: 0; }
  .app-order-title { display: flex; align-items: center; gap: 9px; flex-wrap: wrap; }
  .app-order-title strong { font-size: 14px; letter-spacing: -.01em; }
  .app-order-note { margin: 0; color: var(--muted); font-size: 12px; line-height: 1.45; }
  .app-order-fixed { padding: 3px 8px; border: 1px solid var(--line); border-radius: 99px; color: var(--dim); font-size: 10.5px; font-weight: 700; letter-spacing: .04em; text-transform: uppercase; }
  .app-order-controls { display: flex; align-items: center; justify-content: flex-end; flex-wrap: wrap; gap: 6px; }
  .app-order-nudge { display: flex; gap: 5px; }
  .app-order-nudge button, .app-order-action { min-height: 40px; padding: 0 12px; border: 1px solid var(--line-strong); border-radius: 9px; background: #ffffff08; color: var(--text-dim); font-size: 12.5px; font-weight: 600; white-space: nowrap; }
  .app-order-nudge button { width: 40px; padding: 0; font-size: 15px; }
  .app-order-nudge button:hover, .app-order-action:hover { background: #ffffff12; color: var(--text); }
  .app-order-action.is-done { border-color: transparent; background: var(--accent); color: var(--accent-ink); }
  .app-order-action.is-done:hover { background: var(--accent-strong); color: var(--accent-ink); }
  .app-order-status { min-height: 18px; margin: 10px 2px 0; color: var(--muted); font-size: 12px; line-height: 1.45; }
  @media (max-width: 820px) {
    .app-order-head { flex-direction: column; }
    .app-order-list > li { grid-template-columns: auto minmax(0, 1fr) auto; }
    .app-order-grip { display: none; }
  }
</style>
