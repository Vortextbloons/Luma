<script lang="ts">
  import type { ActivityItem, AppId, HomeRow, HomeRowId, MediaItem, Settings } from '../types';
  import { appProviders, isApplication, isContinueWatching, isRecentlyPlayed, sortRecent } from '../services/activity';
  import { homeRowMeta } from '../services/home';
  import { widgetMeta } from '../services/widgetModel';
  import ProviderIcon from './activity/ProviderIcon.svelte';

  type PreviewThumb = { key: string; art: string | null; provider: string | null; app: boolean };

  let { rows, columns = 1, popular, featured, activityItems, apps, widgets, highlight }: {
    rows: HomeRow[];
    columns?: 1 | 2;
    popular: MediaItem[];
    featured: MediaItem[];
    activityItems: ActivityItem[];
    apps: AppId[];
    widgets: Settings['widgets'];
    highlight: string | null;
  } = $props();

  const activeWidgets = $derived(Object.entries(widgets)
    .map(([id, config]) => ({ id, config, definition: widgetMeta(config.type) }))
    .filter(item => item.definition && item.config.enabled)
    .sort((a, b) => a.config.order - b.config.order || a.id.localeCompare(b.id)));
  const barWidgets = $derived(activeWidgets.filter(item => !item.config.position && item.config.placement === 'bar'));
  const freeWidgets = $derived(activeWidgets.filter(item => item.config.position));
  const widgetGroups = $derived(['top-left', 'top-center', 'top-right', 'rail-left', 'rail-right', 'bottom-left', 'bottom-center', 'bottom-right'].map(placement => ({
    placement,
    items: activeWidgets.filter(item => !item.config.position && item.config.placement === placement),
  })).filter(group => group.items.length));

  const placeholders: PreviewThumb[] = [0, 1, 2, 3].map(index => ({ key: `empty-${index}`, art: null, provider: null, app: false }));

  const activityThumbs = (items: ActivityItem[]): PreviewThumb[] => sortRecent(items).slice(0, 4).map(item => ({ key: item.id, art: item.artwork, provider: item.provider, app: false }));
  const mediaThumbs = (items: MediaItem[]): PreviewThumb[] => items.slice(0, 4).map(item => ({ key: item.id, art: item.backdrop || item.poster || null, provider: null, app: false }));

  function thumbsFor(component: HomeRowId): PreviewThumb[] {
    if (component === 'hero') return [];
    if (component === 'continue') return activityThumbs(activityItems.filter(isContinueWatching));
    if (component === 'recent') return activityThumbs(activityItems.filter(isRecentlyPlayed));
    if (component === 'recentApps') return activityThumbs(activityItems.filter(isApplication));
    if (component === 'popular') return mediaThumbs(popular);
    if (component === 'featured') return mediaThumbs(featured);
    return apps.slice(0, 6).map(app => ({ key: app, art: null, provider: appProviders[app] ?? null, app: true }));
  }

  const previewRows = $derived(rows.filter(row => row.visible).map((row, index) => ({ id: row.id, component: row.component, label: homeRowMeta[row.component].label, thumbs: thumbsFor(row.component), span: row.span, leadingHero: index === 0 && row.component === 'hero' })));
  const hero = $derived(popular[0] ?? featured[0] ?? null);
  const heroArt = $derived(popular[0]?.backdrop || featured[0]?.backdrop || '');
</script>

<div class="home-preview" aria-hidden="true">
  <div class="tv">
    <div class="viewport">
      <div class="screen">
        <div class="preview-top">
          <span class="logo-dot"></span>
          <span class="nav-pills"><i class="is-active"></i><i></i><i></i><i></i></span>
          <span class="clock"></span>
        </div>
        <div class="preview-rows" class:is-columns={columns === 2}>
          {#each previewRows as row (row.id)}
            {#if row.component === 'hero'}
              <div class="hero" class:is-highlighted={highlight === row.id} style:grid-column={row.leadingHero ? '1 / -1' : `span ${row.span}`}>
                {#if heroArt}<img src={heroArt} alt="" loading="lazy" decoding="async" />{/if}
                <span class="hero-shade"></span>
                <div class="hero-copy">
                  <span class="hero-kicker">FEATURED</span>
                  {#if hero}<span class="hero-title">{hero.title}</span>{:else}<span class="hero-bar"></span>{/if}
                  <span class="hero-meta"></span>
                  <span class="hero-actions"><i class="hero-play"></i><i class="hero-more"></i></span>
                </div>
              </div>
            {:else}
              <div class="preview-row" class:is-highlighted={highlight === row.id} style:grid-column={`span ${row.span}`}>
                <span class="preview-row-label">{row.label}</span>
                <span class="preview-thumbs">
                  {#each row.thumbs.length ? row.thumbs : placeholders as thumb (thumb.key)}
                    <span class="preview-thumb" class:is-app={thumb.app}>
                      {#if thumb.art}
                        <img src={thumb.art} alt="" loading="lazy" decoding="async" />
                      {:else if thumb.provider}
                        <ProviderIcon provider={thumb.provider} />
                      {/if}
                    </span>
                  {/each}
                </span>
              </div>
            {/if}
          {:else}
            <span class="preview-empty">No rows on Home</span>
          {/each}
        </div>
        {#if barWidgets.length}
          <div class="widget-bar-stack">
            {#each barWidgets as item (item.id)}
              <div class="widget-bar">
                <span class="widget-glyph" aria-hidden="true">{item.definition!.glyph}</span>
                <span class="widget-name">{item.definition!.name}</span>
                <span class="widget-controls" aria-hidden="true"><i></i><i></i><i></i></span>
              </div>
            {/each}
          </div>
        {/if}
        <div class="widget-layer-preview" aria-hidden="true">
          {#each widgetGroups as group (group.placement)}
            <div class={`widget-card-group is-${group.placement}`}>
              {#each group.items as item (item.id)}
                <div class="widget-card">
                  <span class="widget-glyph">{item.definition!.glyph}</span>
                  <span class="widget-name">{item.definition!.name}</span>
                </div>
              {/each}
            </div>
          {/each}
          {#each freeWidgets as item (item.id)}
            <div class="widget-card is-free" style:left={`${item.config.position!.x / 10}%`} style:top={`${item.config.position!.y / 10}%`} style:transform={`translate(-${item.config.position!.x / 10}%, -${item.config.position!.y / 10}%)`}>
              <span class="widget-glyph">{item.definition!.glyph}</span>
              <span class="widget-name">{item.definition!.name}</span>
            </div>
          {/each}
        </div>
      </div>
    </div>
  </div>
</div>

<style>
  .home-preview { display: block; }
  .tv {
    padding: 7px;
    border: 1px solid var(--line);
    border-radius: 20px;
    background: linear-gradient(160deg, #232736, #0c0e14 58%);
    box-shadow: 0 24px 60px #0007, inset 0 1px #ffffff14;
  }
  .viewport {
    container-type: inline-size;
    overflow: hidden;
    border-radius: 13px;
  }
  .screen {
    position: relative;
    display: flex;
    flex-direction: column;
    gap: 1.2cqw;
    width: 100%;
    aspect-ratio: 16 / 10;
    padding: 2.2cqw;
    overflow: hidden;
    background: radial-gradient(120% 90% at 85% 0%, var(--accent-soft), transparent 60%), var(--bg);
  }
  .preview-top {
    flex: none;
    display: grid;
    grid-template-columns: auto minmax(0, 1fr) auto;
    align-items: center;
    height: 3.4cqw;
  }
  .logo-dot {
    width: 2.1cqw;
    height: 2.1cqw;
    border-radius: 50%;
    background: radial-gradient(circle at 34% 28%, #fff, var(--accent) 62%, var(--accent-strong));
    box-shadow: 0 0 1.2cqw var(--accent-soft);
  }
  .nav-pills {
    justify-self: center;
    display: flex;
    align-items: center;
    gap: .7cqw;
    padding: .45cqw 1.1cqw;
    border-radius: 99px;
    background: #ffffff0d;
  }
  .nav-pills i {
    display: block;
    width: 4.2cqw;
    height: .8cqw;
    border-radius: 99px;
    background: #ffffff26;
  }
  .nav-pills i.is-active { background: #eef0f7; }
  .clock {
    width: 3.2cqw;
    height: .8cqw;
    border-radius: 99px;
    background: #ffffff26;
  }
  .hero {
    position: relative;
    flex: 0 0 13cqw;
    overflow: hidden;
    border: 1px solid #ffffff12;
    border-radius: 1.2cqw;
    background: linear-gradient(120deg, var(--accent-soft), var(--surface-2) 48%, var(--ambient));
  }
  .hero.is-highlighted {
    border-color: var(--accent);
    box-shadow: 0 0 0 1px var(--accent), 0 0 24px var(--accent-soft);
  }
  .hero img {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .hero-shade {
    position: absolute;
    inset: 0;
    background: linear-gradient(90deg, #05070ce6, #05070c80 52%, transparent 82%), linear-gradient(0deg, #05070cb3, transparent 70%);
  }
  .hero-copy {
    position: relative;
    z-index: 1;
    display: flex;
    flex-direction: column;
    justify-content: center;
    gap: .9cqw;
    height: 100%;
    padding: 0 2.4cqw;
  }
  .hero-kicker {
    font-size: 1.35cqw;
    font-weight: 700;
    letter-spacing: .2em;
    color: var(--text-dim);
  }
  .hero-title {
    max-width: 64%;
    font-size: 3.2cqw;
    font-weight: 650;
    letter-spacing: -.02em;
    line-height: 1.1;
    color: #fff;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    text-shadow: 0 2px 12px #000a;
  }
  .hero-bar {
    width: 30cqw;
    height: 2.6cqw;
    border-radius: .6cqw;
    background: #ffffff2e;
  }
  .hero-meta {
    width: 18cqw;
    height: .9cqw;
    border-radius: 99px;
    background: #ffffff2e;
  }
  .hero-actions {
    display: flex;
    align-items: center;
    gap: .9cqw;
    margin-top: .3cqw;
  }
  .hero-play {
    width: 8.5cqw;
    height: 2.6cqw;
    border-radius: 99px;
    background: #f2f4fb;
  }
  .hero-more {
    width: 2.6cqw;
    height: 2.6cqw;
    border-radius: 50%;
    background: #ffffff2b;
  }
  .preview-rows {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: .9cqw;
    min-height: 0;
  }
  .preview-rows.is-columns {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    grid-auto-rows: minmax(0, 1fr);
  }
  .preview-rows.is-columns .preview-empty {
    grid-column: 1 / -1;
  }
  .preview-rows.is-columns .hero,
  .preview-rows.is-columns .preview-row {
    min-width: 0;
  }
  .preview-row {
    flex: 1 1 0;
    display: flex;
    align-items: center;
    gap: 1.2cqw;
    min-height: 0;
    padding: .5cqw .9cqw;
    overflow: hidden;
    border: 1px solid transparent;
    border-radius: 1cqw;
    transition: background-color var(--motion-fast) var(--ease-out), border-color var(--motion-fast) var(--ease-out);
  }
  .preview-row.is-highlighted {
    border-color: color-mix(in srgb, var(--accent) 55%, transparent);
    background: var(--accent-soft);
  }
  .preview-row-label {
    flex: 0 0 31%;
    min-width: 0;
    font-size: 2.5cqw;
    font-weight: 650;
    letter-spacing: -.01em;
    line-height: 1.1;
    color: var(--text-dim);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .preview-thumbs {
    flex: 1;
    display: flex;
    align-items: center;
    gap: .7cqw;
    height: 100%;
    min-width: 0;
    overflow: hidden;
  }
  .preview-thumb {
    flex: 1 1 0;
    display: grid;
    place-items: center;
    height: 100%;
    max-height: 4.8cqw;
    max-width: 13cqw;
    overflow: hidden;
    border: 1px solid #ffffff0d;
    border-radius: .8cqw;
    background: linear-gradient(135deg, var(--surface-3), var(--surface-2));
  }
  .preview-thumb img {
    display: block;
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .preview-thumb.is-app {
    flex: 0 0 auto;
    width: auto;
    max-width: 4.8cqw;
    aspect-ratio: 1;
    border-radius: 1.1cqw;
    background: #ffffff0a;
  }
  .preview-thumb.is-app :global(.provider-icon) {
    width: 70%;
    height: 70%;
    font-size: 1.8cqw;
    border-radius: 28%;
  }
  .preview-empty {
    display: grid;
    place-items: center;
    flex: 1;
    font-size: 2.2cqw;
    color: var(--dim);
  }
  .widget-bar-stack {
    flex: none;
    display: grid;
    gap: .5cqw;
  }
  .widget-bar {
    display: flex;
    align-items: center;
    gap: .8cqw;
    height: 5cqw;
    padding: .6cqw 1cqw;
    border: 1px solid var(--line);
    border-radius: 1.1cqw;
    background: color-mix(in srgb, var(--surface-3) 92%, transparent);
    box-shadow: 0 4px 14px #0006;
  }
  .widget-glyph {
    flex: none;
    display: grid;
    place-items: center;
    width: 2.8cqw;
    height: 2.8cqw;
    border-radius: .8cqw;
    background: var(--accent-soft);
    color: var(--text);
    font-size: 1.35cqw;
    font-weight: 700;
  }
  .widget-name {
    flex: 1;
    overflow: hidden;
    color: var(--text-dim);
    font-size: 1.15cqw;
    font-weight: 600;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .widget-controls {
    display: flex;
    align-items: center;
    gap: .7cqw;
  }
  .widget-controls i {
    display: block;
    width: 1.4cqw;
    height: 1.4cqw;
    border-radius: 50%;
    background: #ffffff47;
  }
  .widget-controls i:nth-child(2) {
    width: 2.4cqw;
    height: 2.4cqw;
    background: #f2f4fb;
  }
  .widget-layer-preview {
    position: absolute;
    inset: 2.2cqw;
    z-index: 2;
    pointer-events: none;
  }
  .widget-card-group {
    position: absolute;
    display: flex;
    gap: .6cqw;
  }
  .widget-card-group.is-top-left { top: 0; left: 0; flex-direction: column; align-items: flex-start; }
  .widget-card-group.is-top-center { top: 0; left: 0; right: 0; flex-direction: column; align-items: center; }
  .widget-card-group.is-top-right { top: 0; right: 0; flex-direction: column; align-items: flex-end; }
  .widget-card-group.is-bottom-left { bottom: 0; left: 0; flex-direction: column; align-items: flex-start; }
  .widget-card-group.is-bottom-center { bottom: 0; left: 0; right: 0; flex-direction: column; align-items: center; }
  .widget-card-group.is-bottom-right { bottom: 0; right: 0; flex-direction: column; align-items: flex-end; }
  .widget-card-group.is-rail-left { left: 0; top: 0; bottom: 0; flex-direction: column; justify-content: center; align-items: flex-start; }
  .widget-card-group.is-rail-right { right: 0; top: 0; bottom: 0; flex-direction: column; justify-content: center; align-items: flex-end; }
  .widget-card {
    display: flex;
    align-items: center;
    gap: .6cqw;
    width: 15cqw;
    padding: .7cqw;
    border: 1px solid var(--line-strong);
    border-radius: 1.1cqw;
    background: color-mix(in srgb, var(--surface-3) 94%, transparent);
    box-shadow: 0 8px 24px #0008;
    backdrop-filter: blur(4px);
  }
  .widget-card.is-free { position: absolute; }
  .widget-card .widget-glyph { width: 2.4cqw; height: 2.4cqw; font-size: 1.1cqw; }
  .widget-card .widget-name { font-size: 1cqw; }
</style>
