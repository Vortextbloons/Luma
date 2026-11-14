<script lang="ts">
  import type { MediaItem } from '../types';
  let { wallpaper, item, launch, details }: { wallpaper: string; item: MediaItem | null; launch: () => void; details: () => void } = $props();
</script>

<section class="hero" aria-labelledby={item ? 'hero-title' : undefined}>
  <div class="hero-art" style:background-image={wallpaper || item?.backdrop ? `url("${wallpaper || item?.backdrop}")` : undefined}></div>
  <div class="hero-vignette"></div>
  {#if item}
  <div class="hero-copy">
    <div class="eyebrow"><span class="spotlight-dot"></span> TRENDING NOW</div>
    <h1 id="hero-title">{item.title}</h1>
    <div class="metadata"><span class="meta-chip">MOVIE</span><span>{item.year}</span><span class="meta-divider"></span><span>{item.runtime}</span>{#if item.rating}<span class="meta-quality">★ {item.rating}</span>{/if}</div>
    <p class="hero-desc">{item.description}</p>
    <div class="hero-actions"><button class="btn btn-primary" onclick={launch}><span aria-hidden="true">▶</span> Open Stremio</button><button class="btn btn-glass" onclick={details}><span class="circle-info" aria-hidden="true">i</span> More info</button></div>
  </div>
  {/if}
  <div class="hero-footer"><span class="hero-hint">Your entertainment, together</span><span class="hero-hint">Popular on Stremio</span></div>
</section>
