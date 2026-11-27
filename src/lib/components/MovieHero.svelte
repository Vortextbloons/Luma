<script module lang="ts">
  export interface HeroChip {
    label: string;
    tone?: 'rating';
  }
  export interface HeroSlide {
    key: string;
    eyebrow: string;
    title: string;
    chips: HeroChip[];
    description: string;
    art: string;
    artShape?: 'square' | 'portrait';
    primaryLabel: string;
    primaryIcon?: 'play' | 'open';
    primary: () => void;
    details?: () => void;
    detailsLabel?: string;
  }
</script>

<script lang="ts">
  let { slides, sourceKey, inset = false }: { slides: HeroSlide[]; sourceKey: string; inset?: boolean } = $props();
  let index = $state(0);
  let shownKey = $state('');
  const active = $derived(Math.min(index, slides.length - 1));
  const item = $derived(slides[active]);
  const art = $derived(item?.art);
  const artKey = $derived(item?.key);
  const artShape = $derived(item?.artShape);
  let loadedArt = $state('');
  let loadedShape = $state<HeroSlide['artShape']>();
  $effect(() => {
    const slide = { art, key: artKey, artShape };
    loadedArt = '';
    let cancelled = false;
    if (!slide?.art) { loadedArt = ''; return; }
    const candidates = [slide.art];
    if (slide.key?.startsWith('steam:')) {
      const id = slide.key.split(':')[1];
      candidates.push(`https://shared.fastly.steamstatic.com/store_item_assets/steam/apps/${id}/library_hero.jpg`, `https://shared.fastly.steamstatic.com/store_item_assets/steam/apps/${id}/header.jpg`, `https://cdn.akamai.steamstatic.com/steam/apps/${id}/header.jpg`);
    }
    const image = new Image();
    let attempt = 0;
    image.onload = () => {
      if (!cancelled) { loadedArt = image.src; loadedShape = slide.artShape; }
    };
    image.onerror = () => {
      if (cancelled) return;
      attempt += 1;
      if (attempt < candidates.length) image.src = candidates[attempt];
      else loadedArt = '';
    };
    image.src = candidates[0];
    return () => { cancelled = true; image.onload = null; image.onerror = null; };
  });
  $effect(() => {
    if (shownKey !== sourceKey) {
      shownKey = sourceKey;
      index = 0;
    }
  });
</script>

{#if item}
  <section class="hero" class:is-inset={inset} aria-label={item.eyebrow}>
    {#if loadedArt}
      <img class="hero-art" class:is-shaped={loadedShape} src={loadedArt} alt="" decoding="async" fetchpriority="high" />
      {#if loadedShape}
        <img class="hero-cover" class:is-portrait={loadedShape === 'portrait'} src={loadedArt} alt="" decoding="async" />
      {/if}
    {/if}
    <div class="hero-shade" aria-hidden="true"></div>
    {#key item.key}
      <div class="hero-inner">
      <span class="hero-eyebrow">{item.eyebrow}</span>
      <h1>{item.title}</h1>
      {#if item.chips.length}
        <div class="hero-meta">
          {#each item.chips as chip, i (chip.label + i)}
            <span class="hero-chip" class:is-rating={chip.tone === 'rating'}>{chip.label}</span>
          {/each}
        </div>
      {/if}
      {#if item.description}<p class="hero-description">{item.description}</p>{/if}
      <div class="hero-actions">
        <button data-focus-key={`${item.key}:primary`} class="btn btn-primary" onclick={item.primary}>
          {#if item.primaryIcon === 'open'}
            <svg class="is-stroke" viewBox="0 0 24 24" aria-hidden="true"><path d="M7 17 17 7" /><path d="M9 7h8v8" /></svg>
          {:else}
            <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M8 5v14l11-7z" /></svg>
          {/if}
          {item.primaryLabel}
        </button>
        {#if item.details}
          <button data-focus-key={`${item.key}:details`} class="btn btn-glass" onclick={item.details}>
            <svg viewBox="0 0 24 24" aria-hidden="true"><circle cx="12" cy="12" r="9" /><path d="M12 11v5" /><path d="M12 8h.01" /></svg>
            {item.detailsLabel ?? 'Details'}
          </button>
        {/if}
      </div>
    </div>
    {/key}
    {#if slides.length > 1}
      <div class="hero-dots" role="group" aria-label="Featured items">
        {#each slides as slide, i (slide.key)}
          <button
            data-focus-key={`${slide.key}:dot`}
            class:is-active={i === active}
            aria-label={slide.title}
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
    min-height: clamp(320px, 44vh, 500px);
    margin: calc(-1 * var(--topbar-space)) 0 clamp(16px, 1.8vw, 24px);
    overflow: hidden;
    isolation: isolate;
    background:
      radial-gradient(80% 90% at 74% 20%, var(--ambient), transparent 70%),
      linear-gradient(120deg, var(--accent-ink), var(--bg));
  }
  .hero.is-inset {
    margin-top: 0;
  }
  .hero-art {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    object-fit: cover;
    object-position: center 24%;
    filter: saturate(1.02);
    animation: hero-art-in .32s var(--ease-out);
  }
  @keyframes hero-art-in {
    from { opacity: .35; }
    to { opacity: 1; }
  }
  .hero-art.is-shaped {
    filter: blur(46px) saturate(1.5) brightness(.42);
    transform: scale(1.16);
  }
  .hero-cover {
    position: absolute;
    z-index: 1;
    top: 50%;
    right: clamp(40px, 8vw, 160px);
    width: clamp(150px, 14vw, 230px);
    aspect-ratio: 1;
    object-fit: cover;
    border-radius: 16px;
    box-shadow: 0 24px 70px #000a;
    transform: translateY(-50%);
    animation: hero-art-in .32s var(--ease-out);
  }
  .hero-cover.is-portrait {
    width: clamp(130px, 11vw, 180px);
    aspect-ratio: 2 / 3;
  }
  .hero-shade {
    position: absolute;
    inset: 0;
    background:
      linear-gradient(90deg, color-mix(in srgb, var(--bg) 97%, transparent) 0%, color-mix(in srgb, var(--bg) 84%, transparent) 34%, color-mix(in srgb, var(--bg) 32%, transparent) 64%, color-mix(in srgb, var(--bg) 10%, transparent) 100%),
      linear-gradient(0deg, var(--bg) 0%, color-mix(in srgb, var(--bg) 55%, transparent) 16%, transparent 50%),
      linear-gradient(180deg, color-mix(in srgb, var(--bg) 76%, transparent) 0%, transparent 30%);
  }
  .hero-inner {
    position: relative;
    z-index: 2;
    width: 100%;
    max-width: min(920px, 64%);
    padding: calc(var(--topbar-space) + clamp(12px, 2.4vh, 28px)) var(--side) clamp(16px, 2.2vh, 28px);
    animation: hero-copy-in .18s var(--ease-out);
  }
  @keyframes hero-copy-in {
    from { opacity: 0; transform: translateY(6px); }
    to { opacity: 1; transform: none; }
  }
  .hero-eyebrow {
    display: block;
    color: var(--text-dim);
    font-size: clamp(10px, .64vw, 11px);
    font-weight: 700;
    letter-spacing: .18em;
    margin-bottom: clamp(6px, 1vh, 10px);
  }
  h1 {
    margin: 0;
    font-size: clamp(30px, 3.2vw, 54px);
    font-weight: 700;
    line-height: 1.04;
    letter-spacing: -.045em;
    color: var(--text);
    text-shadow: 0 4px 30px #0009;
    display: -webkit-box;
    -webkit-line-clamp: 3;
    line-clamp: 3;
    -webkit-box-orient: vertical;
    overflow: hidden;
    text-wrap: balance;
  }
  .hero-meta {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 6px 10px;
    margin: clamp(8px, 1.2vh, 12px) 0 0;
    color: var(--text-dim);
    font-size: clamp(12px, .78vw, 14px);
    font-weight: 550;
  }
  .hero-chip {
    display: inline-flex;
    align-items: center;
    min-height: 24px;
    padding: 0 10px;
    border: 1px solid var(--line-strong);
    border-radius: 99px;
    background: #ffffff0f;
    color: var(--text);
    font-size: clamp(11px, .68vw, 12.5px);
    font-weight: 600;
    white-space: nowrap;
  }
  .hero-chip.is-rating { color: var(--rating); border-color: #ffd98a3d; background: #ffd98a14; }
  .hero-description {
    max-width: 60ch;
    margin: clamp(6px, 1vh, 10px) 0 0;
    color: var(--text-dim);
    font-size: clamp(12px, .82vw, 15px);
    line-height: 1.45;
    display: -webkit-box;
    -webkit-line-clamp: 3;
    line-clamp: 3;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }
  .hero-actions {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 10px;
    margin-top: clamp(12px, 1.8vh, 18px);
  }
  .hero-actions .btn {
    min-width: clamp(116px, 8vw, 148px);
    min-height: 46px;
    padding: 0 22px;
    border-radius: var(--control-radius);
  }
  .hero-actions .btn svg {
    width: 17px;
    height: 17px;
    flex: none;
    fill: currentColor;
    stroke: none;
  }
  .hero-actions .btn svg.is-stroke,
  .hero-actions .btn-glass svg {
    fill: none;
    stroke: currentColor;
    stroke-width: 1.8;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
  .hero-dots {
    position: absolute;
    right: var(--side);
    bottom: clamp(12px, 1.8vh, 20px);
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
  .hero-dots button.is-active { width: 26px; background: var(--accent); }
  .hero-dots button:focus-visible { outline: 3px solid var(--accent); outline-offset: 4px; box-shadow: var(--focus-glow); }
  @media (max-width: 1100px) {
    .hero-cover { display: none; }
  }
  @media (max-width: 900px) {
    .hero-inner { max-width: none; }
    .hero-dots { position: static; justify-content: flex-start; padding: 0 var(--side) clamp(14px, 2vh, 20px); }
    .hero-shade {
      background:
        linear-gradient(0deg, color-mix(in srgb, var(--bg) 98%, transparent) 0%, color-mix(in srgb, var(--bg) 80%, transparent) 34%, color-mix(in srgb, var(--bg) 20%, transparent) 70%),
        linear-gradient(180deg, color-mix(in srgb, var(--bg) 72%, transparent) 0%, transparent 34%);
    }
  }
  @media (max-width: 520px) {
    .hero-actions .btn { min-width: 0; padding-inline: 16px; }
    .hero-inner { padding-bottom: 20px; }
  }
</style>
