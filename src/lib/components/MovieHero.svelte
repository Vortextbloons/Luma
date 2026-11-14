<script lang="ts">
  import type { MediaItem } from '../types';
  let { items, onplay, ondetails }: {
    items: MediaItem[];
    onplay: (item: MediaItem) => void;
    ondetails: (item: MediaItem) => void;
  } = $props();
  let index = $state(0);
  const active = $derived(Math.min(index, items.length - 1));
  const item = $derived(items[active]);
  const genres = $derived(item ? item.subtitle.split(' · ').filter(part => part !== item.year).slice(0, 2) : []);
  function runtime(value: string) {
    const minutes = Number.parseInt(value, 10);
    if (!Number.isFinite(minutes) || minutes <= 0) return value;
    return minutes >= 60 ? `${Math.floor(minutes / 60)}h ${minutes % 60}m` : `${minutes}m`;
  }
</script>

{#if item}
  <section class="hero" aria-label="Featured movie">
    <img class="hero-art" src={item.backdrop || item.poster} alt="" decoding="async" fetchpriority="high" />
    <div class="hero-shade" aria-hidden="true"></div>
    <div class="hero-inner">
      <span class="hero-eyebrow">FEATURED MOVIE</span>
      <h1>{item.title}</h1>
      <div class="hero-meta">
        {#each genres as genre (genre)}<span class="hero-chip">{genre}</span>{/each}
        {#if item.rating}<span class="hero-chip hero-rating">★ {item.rating}</span>{/if}
        {#if item.runtime}<span>{runtime(item.runtime)}</span>{/if}
        {#if item.runtime && item.year}<span class="hero-divider" aria-hidden="true">|</span>{/if}
        {#if item.year}<span>{item.year}</span>{/if}
      </div>
      {#if item.description}<p class="hero-description">{item.description}</p>{/if}
      <div class="hero-actions">
        <button class="btn btn-primary" onclick={() => onplay(item)}>
          <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M8 5v14l11-7z" /></svg>
          Play
        </button>
        <button class="btn btn-glass" onclick={() => ondetails(item)}>
          <svg viewBox="0 0 24 24" aria-hidden="true"><circle cx="12" cy="12" r="9" /><path d="M12 11v5" /><path d="M12 8h.01" /></svg>
          Details
        </button>
      </div>
    </div>
    {#if items.length > 1}
      <div class="hero-dots" role="group" aria-label="Featured movies">
        {#each items as entry, i (entry.id)}
          <button
            class:is-active={i === active}
            aria-label={entry.title}
            aria-current={i === active ? 'true' : undefined}
            onclick={() => index = i}
          ></button>
        {/each}
      </div>
    {/if}
  </section>
{/if}

<style>
  .hero {
    position: relative;
    display: flex;
    flex-direction: column;
    justify-content: flex-end;
    min-height: clamp(460px, 64vh, 700px);
    margin: calc(-1 * var(--topbar-space)) 0 clamp(22px, 2.4vw, 36px);
    overflow: hidden;
    isolation: isolate;
    background: #0b0d12;
  }
  .hero-art {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    object-fit: cover;
    object-position: center 22%;
    filter: saturate(1.02);
  }
  .hero-shade {
    position: absolute;
    inset: 0;
    background:
      linear-gradient(90deg, rgba(11, 13, 18, .97) 0%, rgba(11, 13, 18, .84) 32%, rgba(11, 13, 18, .34) 62%, rgba(11, 13, 18, .12) 100%),
      linear-gradient(0deg, var(--bg) 0%, rgba(11, 13, 18, .6) 18%, transparent 54%),
      linear-gradient(180deg, rgba(11, 13, 18, .76) 0%, transparent 32%);
  }
  .hero-inner {
    position: relative;
    z-index: 1;
    width: 100%;
    max-width: min(760px, 62%);
    padding: calc(var(--topbar-space) + clamp(36px, 5vh, 72px)) var(--side) clamp(34px, 4.4vh, 58px);
  }
  .hero-eyebrow {
    display: block;
    color: #c8ccda;
    font-size: clamp(10px, .68vw, 12px);
    font-weight: 700;
    letter-spacing: .18em;
    margin-bottom: clamp(10px, 1.4vh, 16px);
  }
  h1 {
    margin: 0;
    font-size: clamp(38px, 4.6vw, 72px);
    font-weight: 700;
    line-height: .98;
    letter-spacing: -.045em;
    color: #fbfbfe;
    text-shadow: 0 4px 30px #0009;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
    text-wrap: balance;
  }
  .hero-meta {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 8px 12px;
    margin: clamp(14px, 1.8vh, 20px) 0 0;
    color: #d5d8e2;
    font-size: clamp(12px, .82vw, 15px);
    font-weight: 550;
  }
  .hero-chip {
    display: inline-flex;
    align-items: center;
    min-height: 26px;
    padding: 0 11px;
    border: 1px solid #ffffff2b;
    border-radius: 99px;
    background: #ffffff0f;
    color: #eef0f6;
    font-size: clamp(11px, .72vw, 13px);
    font-weight: 600;
  }
  .hero-rating { color: #ffd98a; border-color: #ffd98a3d; background: #ffd98a14; }
  .hero-divider { color: #ffffff38; }
  .hero-description {
    max-width: 58ch;
    margin: clamp(12px, 1.6vh, 18px) 0 0;
    color: #c3c8d6;
    font-size: clamp(13px, .95vw, 17px);
    line-height: 1.55;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }
  .hero-actions {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 12px;
    margin-top: clamp(18px, 2.4vh, 28px);
  }
  .hero-actions .btn svg {
    width: 18px;
    height: 18px;
    flex: none;
    fill: currentColor;
    stroke: none;
  }
  .hero-actions .btn-glass svg {
    fill: none;
    stroke: currentColor;
    stroke-width: 1.8;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
  .hero-actions .btn { min-width: clamp(124px, 9vw, 156px); }
  .hero-dots {
    position: absolute;
    right: var(--side);
    bottom: clamp(20px, 2.6vh, 32px);
    z-index: 2;
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .hero-dots button {
    width: 8px;
    height: 8px;
    padding: 0;
    border-radius: 99px;
    background: #ffffff47;
    transition: width .2s var(--ease-out), background .2s var(--ease-out);
  }
  .hero-dots button:hover { background: #ffffff78; filter: none; }
  .hero-dots button.is-active { width: 26px; background: #fff; }
  .hero-dots button:focus-visible { outline: 2px solid var(--accent); outline-offset: 4px; }
  @media (max-width: 900px) {
    .hero-inner { max-width: none; }
    .hero-dots { position: static; justify-content: flex-start; padding: 0 var(--side) clamp(18px, 2.4vh, 26px); }
    .hero-shade {
      background:
        linear-gradient(0deg, rgba(11, 13, 18, .98) 0%, rgba(11, 13, 18, .8) 34%, rgba(11, 13, 18, .2) 70%),
        linear-gradient(180deg, rgba(11, 13, 18, .72) 0%, transparent 34%);
    }
  }
  @media (max-width: 520px) {
    .hero-actions .btn { min-width: 0; padding-inline: 18px; }
    .hero-inner { padding-bottom: 26px; }
  }
</style>
