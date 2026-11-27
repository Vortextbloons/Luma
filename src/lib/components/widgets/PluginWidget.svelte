<script lang="ts">
  import type { WidgetProps } from '../../services/widgets';
  import { widgetMeta } from '../../services/widgetModel';
  import { pluginWidgets } from '../../state/pluginWidgets.svelte';

  let { config, compact = false, moving = false, onopen }: WidgetProps = $props();

  const meta = $derived(widgetMeta(config.type));
  const data = $derived(pluginWidgets.forType(config.type));
  const variant = $derived(config.variant || 'glass');
  const title = $derived(data?.title || meta?.name || 'Plugin widget');
  const subtitle = $derived(data?.subtitle || (data ? '' : 'Waiting for plugin data…'));
  const badge = $derived(data?.badge);
  const primary = $derived(data?.primary);
  const rows = $derived(data?.rows ?? []);
  const metaTags = $derived(data?.meta ?? []);
  const progress = $derived(typeof data?.progress === 'number' ? Math.max(0, Math.min(100, data.progress)) : null);
  const tone = $derived(data?.tone || 'muted');
</script>

<button
  type="button"
  class={['plugin-widget', `variant-${variant}`, `size-${config.size}`, `tone-${tone}`, { 'is-compact': compact, 'is-moving': moving }]}
  onclick={onopen}
>
  <header class="pw-head">
    <div class="pw-titles">
      <strong class="pw-title">{title}</strong>
      {#if subtitle}<span class="pw-subtitle">{subtitle}</span>{/if}
    </div>
    {#if badge}
      <span class="pw-badge">{badge}</span>
    {/if}
  </header>

  {#if primary && config.size !== 'small'}
    <div class="pw-primary">{primary}</div>
  {/if}

  {#if rows.length && (config.size !== 'small' || !primary)}
    <div class="pw-rows">
      {#each rows as row (row.label)}
        <div class="pw-row">
          <span class="pw-row-label">{row.label}</span>
          <span class="pw-row-value">{row.value}</span>
          {#if row.sub && config.size === 'large'}<span class="pw-row-sub">{row.sub}</span>{/if}
        </div>
      {/each}
    </div>
  {/if}

  {#if progress !== null && variant !== 'minimal'}
    <div class="pw-progress-wrap">
      <div class="pw-progress-bar" aria-hidden="true">
        <div class="pw-progress-fill" style:width={`${progress}%`}></div>
      </div>
      {#if data?.progressLabel && config.size !== 'small'}
        <span class="pw-progress-label">{data.progressLabel}</span>
      {/if}
    </div>
  {/if}

  {#if metaTags.length && config.size !== 'small'}
    <div class="pw-meta">
      {#each metaTags as tag (tag)}
        <span class="pw-meta-chip">{tag}</span>
      {/each}
    </div>
  {/if}
</button>

<style>
  .plugin-widget {
    width: 100%;
    display: grid;
    gap: 10px;
    padding: 14px 16px;
    border-radius: 16px;
    border: 1px solid color-mix(in srgb, var(--line) 85%, transparent);
    background: color-mix(in srgb, var(--surface) 78%, transparent);
    backdrop-filter: blur(18px);
    color: var(--text);
    text-align: left;
    cursor: pointer;
    transition: transform .16s ease, border-color .16s ease, background .16s ease;
  }
  .plugin-widget:hover { border-color: var(--line-strong); transform: translateY(-1px); }
  .plugin-widget:focus-visible { outline: 3px solid var(--accent); outline-offset: 3px; }
  .plugin-widget.variant-cinematic {
    background: linear-gradient(145deg, color-mix(in srgb, var(--surface-2) 92%, transparent), color-mix(in srgb, var(--accent) 16%, var(--surface)));
    border-color: color-mix(in srgb, var(--accent) 32%, var(--line));
  }
  .plugin-widget.variant-ticker { padding: 10px 14px; border-radius: 12px; gap: 6px; }
  .plugin-widget.variant-minimal { background: transparent; border-color: transparent; backdrop-filter: none; padding: 8px 10px; }
  .plugin-widget.size-small { padding: 10px 12px; gap: 6px; }
  .plugin-widget.is-compact { padding: 10px 12px; gap: 6px; }
  .pw-head { display: flex; align-items: flex-start; justify-content: space-between; gap: 10px; }
  .pw-titles { display: grid; gap: 2px; min-width: 0; }
  .pw-title { font-size: clamp(14px, 1vw, 17px); font-weight: 700; letter-spacing: -.01em; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .pw-subtitle { font-size: clamp(12px, .82vw, 14px); color: var(--text-dim); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .pw-badge {
    flex: none;
    padding: 3px 8px;
    border-radius: 999px;
    font-size: 11px;
    font-weight: 700;
    letter-spacing: .04em;
    text-transform: uppercase;
    background: color-mix(in srgb, var(--surface-3) 85%, transparent);
    color: var(--text-dim);
  }
  .tone-live .pw-badge { background: color-mix(in srgb, #ef4444 22%, transparent); color: #fca5a5; }
  .tone-accent .pw-badge { background: color-mix(in srgb, var(--accent) 22%, transparent); color: var(--text); }
  .pw-primary { font-size: clamp(20px, 1.5vw, 26px); font-weight: 800; font-variant-numeric: tabular-nums; letter-spacing: -.02em; }
  .pw-rows { display: grid; gap: 4px; }
  .pw-row { display: flex; align-items: baseline; justify-content: space-between; gap: 10px; font-size: clamp(13px, .9vw, 15px); }
  .pw-row-label { font-weight: 600; color: var(--text); }
  .pw-row-value { font-weight: 700; font-variant-numeric: tabular-nums; margin-left: auto; }
  .pw-row-sub { font-size: 12px; color: var(--muted); }
  .pw-progress-wrap { display: grid; gap: 4px; }
  .pw-progress-bar { height: 5px; border-radius: 999px; background: color-mix(in srgb, var(--surface-3) 90%, transparent); overflow: hidden; }
  .pw-progress-fill { height: 100%; border-radius: 999px; background: var(--accent); transition: width .3s ease; }
  .pw-progress-label { font-size: 11px; color: var(--muted); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .pw-meta { display: flex; flex-wrap: wrap; gap: 6px; }
  .pw-meta-chip { padding: 2px 8px; border-radius: 999px; font-size: 11px; font-weight: 600; background: color-mix(in srgb, var(--surface-3) 80%, transparent); color: var(--text-dim); }
</style>
