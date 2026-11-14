<script lang="ts">
  import type { MediaItem } from '../types';
  let { title, items, id, onselect }: { title: string; items: MediaItem[]; id: string; onselect: (item: MediaItem) => void } = $props();
  const rowPoster = (item: MediaItem) => item.poster.replace('/poster/medium/', '/poster/small/');
  const rowPosterSet = (item: MediaItem) => `${rowPoster(item)} 1x, ${item.poster} 2x`;
</script>

<section {id} class="content-row" aria-label={title}>
  <div class="row-heading"><div><h2>{title}</h2><p>From Stremio's catalog</p></div></div>
  <div class="card-track">
    {#each items as item (item.id)}
      <button class="media-card" onclick={() => onselect(item)}>
        <span class="poster">
          <img src={rowPoster(item)} srcset={rowPosterSet(item)} alt="" loading="lazy" decoding="async" />
          <span class="poster-copy">
            <span class="card-name">{item.title}</span><span class="card-meta">{item.subtitle}</span>
          </span>
        </span>
      </button>
    {/each}
  </div>
</section>
