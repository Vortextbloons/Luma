<script lang="ts">
  import PosterImage from './PosterImage.svelte';
  import type { MediaItem } from '../types';
  let { title, items, id, onselect, kind = 'movie' }: { kind?: 'movie' | 'series'; title: string; items: MediaItem[]; id: string; onselect: (item: MediaItem) => void } = $props();
</script>

<section {id} class="content-row" aria-label={title}>
  <div class="row-heading"><div><h2>{title}</h2><p>From Stremio's catalog{#if kind === 'series'} ? Artwork: <a href="https://www.tvmaze.com" target="_blank" rel="noreferrer">TVmaze</a>{/if}</p></div></div>
  <div class="card-track">
    {#each items as item (item.id)}
      <button class="media-card" onclick={() => onselect(item)}>
        <span class="poster">
          <PosterImage src={item.poster} imdb={item.id} show={kind === 'series'} />
          <span class="poster-copy">
            <span class="card-name">{item.title}</span><span class="card-meta">{item.subtitle}</span>
          </span>
        </span>
      </button>
    {/each}
  </div>
</section>
