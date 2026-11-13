<script lang="ts">
  import type { MediaItem } from '../types';
  let { title, items, id, onselect }: { title: string; items: MediaItem[]; id: string; onselect: (item: MediaItem) => void } = $props();
</script>

<section {id} class="content-row" aria-label={title}>
  <div class="row-heading"><div><h2>{title}</h2><p>Demo collection</p></div></div>
  <div class="card-track">
    {#each items as item (item.title)}
      <button class="media-card" onclick={() => onselect(item)}>
        <span class="poster"><img src={item.artwork} alt="" loading="lazy" />{#if item.progress !== undefined}<span class="progress"><i style:width={`${item.progress}%`}></i></span>{/if}</span>
        <span class="card-name">{item.title}</span><span class="card-meta">{item.subtitle}</span>
      </button>
    {/each}
  </div>
</section>
