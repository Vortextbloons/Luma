<script lang="ts">
  import { tick } from 'svelte';
  import type { ActivityItem, AppId, HomeRow, MediaItem, WidgetConfig } from '../types';
  import { homeRowMeta } from '../services/home';
  import HomePreview from './HomePreview.svelte';

  let { rows, columns = 1, popular, featured, activityItems, apps, widget, onmove, onmoveto, ontoggle, onreset, oncolumns, onspan, onpreview, onreload, onrestore, busy = false }: {
    rows: HomeRow[];
    columns?: 1 | 2;
    popular: MediaItem[];
    featured: MediaItem[];
    activityItems: ActivityItem[];
    apps: AppId[];
    widget: WidgetConfig;
    onmove: (id: string, direction: -1 | 1) => void;
    onmoveto: (id: string, index: number) => void;
    ontoggle: (id: string) => void;
    onreset: () => void;
    oncolumns?: (columns: 1 | 2) => void;
    onspan?: (id: string, span: 1 | 2) => void;
    onpreview?: () => Promise<void>;
    onreload?: () => void;
    onrestore?: () => Promise<void>;
    busy?: boolean;
  } = $props();

  let lifted = $state<string | null>(null);
  let liftOrigin = $state(0);
  let status = $state('');
  let hovered = $state<string | null>(null);
  let focused = $state<string | null>(null);
  let listElement = $state<HTMLOListElement | null>(null);

  const highlight = $derived(hovered ?? focused);
  const visibleCount = $derived(rows.filter(row => row.visible).length);

  function labelOf(id: string) {
    const row = rows.find(item => item.id === id);
    return row ? homeRowMeta[row.component].label : '';
  }

  function startLift(id: string, index: number) {
    if (busy) return;
    lifted = id;
    liftOrigin = index;
    status = `${labelOf(id)} is ready to move. Use the arrow keys or the buttons, then choose Done.`;
  }

  async function refocus(target: HTMLElement | null, id: string) {
    await tick();
    const fallback = listElement?.querySelector<HTMLButtonElement>(`li[data-row="${id}"] .layout-action`);
    (target?.isConnected ? target : fallback)?.focus({ preventScroll: true });
  }

  function focusedElement() {
    return document.activeElement instanceof HTMLElement ? document.activeElement : null;
  }

  function move(id: string, direction: -1 | 1) {
    if (busy || lifted !== id) return;
    const index = rows.findIndex(row => row.id === id);
    const target = index + direction;
    if (index < 0 || target < 0 || target >= rows.length) {
      status = `${labelOf(id)} is already ${direction < 0 ? 'at the top' : 'at the bottom'}.`;
      return;
    }
    const focusTarget = focusedElement();
    onmove(id, direction);
    status = `${labelOf(id)} moved to position ${target + 1} of ${rows.length}.`;
    void refocus(focusTarget, id);
  }

  function commit() {
    if (busy || !lifted) return;
    const id = lifted;
    const index = rows.findIndex(row => row.id === id);
    status = `${labelOf(id)} placed at position ${index + 1} of ${rows.length} in this draft.`;
    lifted = null;
  }

  function cancel() {
    if (busy || !lifted) return;
    const id = lifted;
    const focusTarget = focusedElement();
    onmoveto(id, liftOrigin);
    status = `${labelOf(id)} returned to position ${liftOrigin + 1} of ${rows.length}. Move cancelled.`;
    lifted = null;
    void refocus(focusTarget, id);
  }

  function toggle(row: HomeRow) {
    if (busy) return;
    const wasVisible = row.visible;
    ontoggle(row.id);
    status = `${homeRowMeta[row.component].label} ${wasVisible ? 'hidden from Home' : 'shown on Home'}.`;
  }

  function reset() {
    if (busy) return;
    const focusTarget = focusedElement();
    const id = lifted;
    lifted = null;
    onreset();
    status = 'Home layout reset to its default order in this draft.';
    if (id) void refocus(focusTarget, id);
  }

  function changeColumns(event: Event) {
    if (busy || !oncolumns) return;
    const next: 1 | 2 = (event.currentTarget as HTMLSelectElement).value === '2' ? 2 : 1;
    oncolumns(next);
    status = next === 2 ? 'Two columns in this draft. Set a width for each row.' : 'One column in this draft. Every row fills the width.';
  }

  function changeSpan(id: string, event: Event) {
    if (busy || !onspan) return;
    const span: 1 | 2 = (event.currentTarget as HTMLSelectElement).value === '1' ? 1 : 2;
    onspan(id, span);
    status = `${labelOf(id)} set to ${span === 2 ? 'full width' : 'one column'} in this draft.`;
  }

  function rowKeydown(event: KeyboardEvent, id: string) {
    if (lifted !== id) return;
    if (event.key === 'Escape') {
      event.preventDefault();
      event.stopPropagation();
      cancel();
      return;
    }
    if (event.key !== 'ArrowUp' && event.key !== 'ArrowDown') return;
    event.preventDefault();
    event.stopPropagation();
    move(id, event.key === 'ArrowUp' ? -1 : 1);
  }

  function rowBlur(event: FocusEvent, id: string) {
    const next = event.relatedTarget as Node | null;
    if (next && event.currentTarget instanceof HTMLElement && event.currentTarget.contains(next)) return;
    if (focused === id) focused = null;
  }

  function rowLeave(id: string) {
    if (hovered === id) hovered = null;
  }
</script>

<section class="settings-group home-layout">
  <header class="layout-head">
    <div class="layout-head-copy">
      <h3>Home layout</h3>
      <p class="panel-description">Choose how many columns Home uses, which rows appear, and the order they play in. Select Move, then use the arrow keys or the up and down buttons to place the row. Changes stay a draft until you preview Home and keep them.</p>
    </div>
    <button class="panel-secondary layout-reset" disabled={busy} onclick={reset}>Reset layout</button>
  </header>
  <div class="layout-grid">
    <div class="layout-preview">
      <div class="preview-label"><span>Live preview</span><small>{visibleCount} of {rows.length} rows</small></div>
      <HomePreview {rows} {columns} {popular} {featured} {activityItems} {apps} {widget} {highlight} />
      <p class="preview-note">Focus or hover a row to find it in the preview.{#if columns === 2} Full width rows take both columns. The first hero stays full width. Narrow windows stack rows.{/if}</p>
    </div>
    <div class="layout-editor">
      <div class="layout-options">
        <div class="layout-options-copy">
          <strong>Home columns</strong>
          <small>One column stacks every row. Two columns places rows side by side.</small>
        </div>
        <label class="layout-columns">Columns
          <select value={columns} disabled={busy || !oncolumns} onchange={changeColumns}>
            <option value={1}>One column</option>
            <option value={2}>Two columns</option>
          </select>
        </label>
      </div>
      <ol bind:this={listElement} class="layout-list" aria-label="Home rows">
        {#each rows as row, index (row.id)}
          <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
          <li
            data-row={row.id}
            class:is-lifted={lifted === row.id}
            class:is-hidden={!row.visible}
            onkeydown={(event) => rowKeydown(event, row.id)}
            onfocusin={() => focused = row.id}
            onfocusout={(event) => rowBlur(event, row.id)}
            onmouseenter={() => hovered = row.id}
            onmouseleave={() => rowLeave(row.id)}
          >
            <span class="layout-index" aria-hidden="true">{index + 1}</span>
            <div class="layout-copy">
              <strong>{homeRowMeta[row.component].label}{#if !row.visible}<span class="settings-badge is-muted">Hidden</span>{/if}</strong>
              <small>{homeRowMeta[row.component].detail}</small>
            </div>
            <div class="layout-controls">
              <label class="layout-width">Width
                <select
                  value={row.span}
                  aria-label={`Width for ${homeRowMeta[row.component].label}`}
                  disabled={columns === 1 || busy || !onspan}
                  onchange={(event) => changeSpan(row.id, event)}
                >
                  <option value={1}>One column</option>
                  <option value={2}>Full width</option>
                </select>
              </label>
              <label class="layout-visibility">
                <input type="checkbox" checked={row.visible} disabled={busy} aria-label={`Show ${homeRowMeta[row.component].label} on Home`} onchange={() => toggle(row)} />
                <span>Visible</span>
              </label>
              {#if lifted === row.id}
                <div class="layout-nudge">
                  <button disabled={busy} onclick={() => move(row.id, -1)} aria-label={`Move ${homeRowMeta[row.component].label} up`}>↑</button>
                  <button disabled={busy} onclick={() => move(row.id, 1)} aria-label={`Move ${homeRowMeta[row.component].label} down`}>↓</button>
                </div>
              {/if}
              <button
                class="layout-action"
                class:is-done={lifted === row.id}
                disabled={busy}
                aria-label={lifted === row.id ? `Finish moving ${homeRowMeta[row.component].label}` : `Move ${homeRowMeta[row.component].label}`}
                onclick={() => lifted === row.id ? commit() : startLift(row.id, index)}
              >{lifted === row.id ? 'Done' : 'Move'}</button>
            </div>
          </li>
        {/each}
      </ol>
      <p class="layout-status" role="status" aria-live="polite" aria-atomic="true">{status}</p>
    </div>
  </div>
  <footer class="layout-footer">
    <div class="layout-footer-copy">
      <strong>Preview before you save</strong>
      <small>Draft changes save only after you preview them on Home and choose Keep. Reload returns to the saved layout; Restore returns to the previous saved layout.</small>
    </div>
    <div class="settings-actions">
      <button class="panel-secondary is-primary" disabled={busy || !onpreview} onclick={() => void onpreview?.()}>Preview on Home</button>
      <button class="panel-secondary" disabled={busy || !onreload} onclick={() => onreload?.()}>Reload saved layout</button>
      <button class="panel-secondary" disabled={busy || !onrestore} onclick={() => void onrestore?.()}>Restore previous saved layout</button>
    </div>
  </footer>
</section>

<style>
  .home-layout {
    container-type: inline-size;
  }
  .layout-head {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 20px;
    margin-bottom: 20px;
  }
  .layout-head-copy {
    min-width: 0;
  }
  .layout-head .panel-description {
    margin: 0;
  }
  .layout-reset {
    flex: none;
  }
  .layout-grid {
    display: grid;
    grid-template-columns: minmax(0, 480px) minmax(0, 1fr);
    align-items: start;
    gap: clamp(20px, 2.4vw, 38px);
  }
  .layout-options {
    display: flex;
    align-items: center;
    justify-content: space-between;
    flex-wrap: wrap;
    gap: 14px 20px;
    margin-bottom: 12px;
    padding: 12px 14px;
    border: 1px solid var(--line);
    border-radius: 12px;
    background: #ffffff04;
  }
  .layout-options-copy {
    display: grid;
    gap: 3px;
    min-width: 0;
  }
  .layout-options-copy strong {
    color: var(--text);
    font-size: 14px;
    line-height: 1.25;
  }
  .layout-options-copy small {
    color: var(--muted);
    font-size: 12.5px;
    line-height: 1.45;
  }
  .layout-columns,
  .layout-width {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    color: var(--muted);
    font-size: 12px;
    font-weight: 600;
    white-space: nowrap;
  }
  .layout-columns select,
  .layout-width select {
    min-height: 40px;
    padding: 0 10px;
    border: 1px solid var(--line-strong);
    border-radius: 9px;
    background: #ffffff08;
    color: var(--text-dim);
    font: inherit;
    font-size: 13px;
    font-weight: 600;
  }
  .layout-columns select:hover:not(:disabled),
  .layout-width select:hover:not(:disabled) {
    background: #ffffff12;
    color: #fff;
  }
  .layout-columns select:disabled,
  .layout-width select:disabled {
    opacity: .4;
    cursor: default;
  }
  .layout-footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    flex-wrap: wrap;
    gap: 16px 24px;
    margin-top: 20px;
    padding-top: 18px;
    border-top: 1px solid var(--line);
  }
  .layout-footer-copy {
    display: grid;
    gap: 3px;
    min-width: 0;
    max-width: 62ch;
  }
  .layout-footer-copy strong {
    color: var(--text);
    font-size: 14px;
    line-height: 1.25;
  }
  .layout-footer-copy small {
    color: var(--muted);
    font-size: 12.5px;
    line-height: 1.5;
  }
  .layout-footer .settings-actions {
    margin-top: 0;
  }
  .layout-preview {
    position: sticky;
    top: 8px;
    align-self: start;
    width: 100%;
    max-width: 560px;
  }
  .preview-label {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 12px;
    margin-bottom: 10px;
    color: var(--text-dim);
    font-size: 12px;
    font-weight: 650;
    letter-spacing: .04em;
    text-transform: uppercase;
  }
  .preview-label small {
    color: var(--muted);
    font-size: 12px;
    font-weight: 500;
    letter-spacing: 0;
    text-transform: none;
  }
  .preview-note {
    margin: 10px 0 0;
    color: var(--dim);
    font-size: 12px;
    line-height: 1.5;
  }
  .layout-list {
    display: grid;
    gap: 8px;
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .layout-list > li {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr) auto;
    align-items: center;
    gap: 14px;
    padding: 12px 14px;
    border: 1px solid var(--line);
    border-radius: 12px;
    background: #ffffff04;
    transition: background-color var(--motion-fast) var(--ease-out), border-color var(--motion-fast) var(--ease-out), opacity var(--motion-fast) var(--ease-out);
  }
  .layout-list > li:hover {
    background: #ffffff07;
    border-color: var(--line-strong);
  }
  .layout-list > li.is-hidden {
    background: #ffffff02;
    border-color: #ffffff0a;
  }
  .layout-list > li.is-hidden .layout-copy {
    opacity: .55;
  }
  .layout-list > li.is-hidden:hover .layout-copy,
  .layout-list > li.is-hidden:focus-within .layout-copy {
    opacity: 1;
  }
  .layout-list > li.is-lifted {
    border-color: var(--accent);
    background: var(--accent-soft);
  }
  .layout-index {
    display: grid;
    place-items: center;
    width: 30px;
    height: 30px;
    border: 1px solid var(--line);
    border-radius: 9px;
    background: #ffffff08;
    color: var(--muted);
    font-size: 13px;
    font-weight: 700;
    font-variant-numeric: tabular-nums;
  }
  .is-lifted .layout-index {
    border-color: color-mix(in srgb, var(--accent) 45%, transparent);
    background: color-mix(in srgb, var(--accent) 20%, transparent);
    color: var(--text);
  }
  .layout-copy {
    display: grid;
    gap: 3px;
    min-width: 0;
    transition: opacity var(--motion-fast) var(--ease-out);
  }
  .layout-copy strong {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 8px;
    color: var(--text);
    font-size: 15px;
    line-height: 1.25;
  }
  .layout-copy small {
    color: var(--muted);
    font-size: 12.5px;
    line-height: 1.45;
  }
  .layout-controls {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    flex-wrap: wrap;
    gap: 8px;
  }
  .layout-visibility {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    min-height: 40px;
    padding: 0 10px;
    border: 1px solid var(--line);
    border-radius: 9px;
    background: #ffffff06;
    color: var(--text-dim);
    font-size: 12.5px;
    font-weight: 600;
    cursor: pointer;
  }
  .layout-visibility:hover {
    background: #ffffff0d;
    border-color: var(--line-strong);
  }
  .layout-visibility input[type='checkbox'] {
    width: 18px;
    height: 18px;
    margin: 0;
    padding: 0;
    accent-color: var(--accent);
  }
  .layout-nudge {
    display: flex;
    gap: 6px;
  }
  .layout-nudge button,
  .layout-action {
    min-height: 40px;
    padding: 0 14px;
    border: 1px solid var(--line-strong);
    border-radius: 9px;
    background: #ffffff08;
    color: var(--text-dim);
    font-size: 13px;
    font-weight: 600;
    white-space: nowrap;
  }
  .layout-nudge button {
    width: 42px;
    padding: 0;
    font-size: 16px;
    line-height: 1;
  }
  .layout-nudge button:hover,
  .layout-action:hover {
    background: #ffffff12;
    color: #fff;
  }
  .layout-action.is-done {
    border-color: transparent;
    background: var(--accent);
    color: var(--accent-ink);
  }
  .layout-action.is-done:hover {
    background: var(--accent-strong);
    color: var(--accent-ink);
  }
  .layout-status {
    min-height: 20px;
    margin: 12px 2px 0;
    color: var(--muted);
    font-size: 12.5px;
    line-height: 1.5;
  }
  @container (max-width: 1040px) {
    .layout-grid {
      grid-template-columns: minmax(0, 1fr);
    }
    .layout-preview {
      position: static;
    }
  }
</style>
