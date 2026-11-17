<script lang="ts">
  import { showPoster } from '../services/artwork';
  let { src, imdb = '', show = false }: { src: string; imdb?: string; show?: boolean } = $props();
  let image = $state('');
  const sourceSet = $derived(src.includes('/poster/medium/') ? `${src.replace('/poster/medium/', '/poster/small/')} 300w, ${src} 500w` : '');
  function observe(node: HTMLElement) {
    image = '';
    let alive = true;
    let observer: IntersectionObserver;
    const load = async () => {
      const preferred = show ? await showPoster(imdb) : null;
      if (alive) image = preferred || src;
    };
    observer = new IntersectionObserver(entries => {
      if (entries.some(entry => entry.isIntersecting)) { observer.disconnect(); void load(); }
    }, { rootMargin: '200px' });
    observer.observe(node);
    return () => { alive = false; observer.disconnect(); };
  }
</script>
{#key src + imdb + show}
  <span class="poster-image" {@attach observe}>
    <span class="missing-art" aria-hidden="true">Artwork unavailable</span>
    {#if image}<img src={image} srcset={image === src ? sourceSet : undefined} sizes={image === src ? '214px' : undefined} alt="" loading="lazy" decoding="async" onerror={event => { const target = event.currentTarget as HTMLImageElement; if (image !== src && src) image = src; else target.style.display = 'none'; }} />{/if}
  </span>
{/key}
<style>
  .poster-image { position: absolute; inset: 0; display: grid; place-items: center; overflow: hidden; background: linear-gradient(145deg,var(--surface-3),var(--surface-2)); }
  .missing-art { padding: 20px; color: var(--dim); text-align: center; font-size: 13px; }
  .poster-image img { position: absolute; inset: 0; width: 100%; height: 100%; object-fit: cover; }
</style>
