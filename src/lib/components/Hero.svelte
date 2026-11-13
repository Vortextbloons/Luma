<script lang="ts">
  import { heroArtwork } from '../demo';
  let { wallpaper, launch, details }: { wallpaper: string; launch: () => void; details: () => void } = $props();
</script>

<section class="hero" aria-labelledby="hero-title">
  <div class="hero-art" style:background-image={`url("${wallpaper || heroArtwork}")`}></div>
  <div class="hero-vignette"></div>
  <div class="hero-copy">
    <div class="eyebrow"><span class="spotlight-dot"></span> FEATURED TONIGHT</div>
    <h1 id="hero-title">Killers of the<br />Flower Moon</h1>
    <div class="metadata"><span class="meta-chip">MOVIE</span><span>2023</span><span class="meta-divider"></span><span>3h 26m</span><span class="meta-quality">4K</span><span class="meta-quality">DOLBY VISION</span></div>
    <p class="hero-desc">A story of love, greed, and betrayal in the heart of 1920s Oklahoma.</p>
    <div class="hero-actions"><button class="btn btn-primary" onclick={launch}><span aria-hidden="true">▶</span> Open Stremio</button><button class="btn btn-glass" onclick={details}><span class="circle-info" aria-hidden="true">i</span> More info</button></div>
  </div>
  <div class="hero-footer"><span class="hero-hint">Your entertainment, together</span><span class="hero-hint">Demo collection</span></div>
</section>
