<script lang="ts">
  import type { StremioItem } from '../types';
  let { title, items, open }: { title: string; items: StremioItem[]; open: (id: string) => void } = $props();
  const kind = (item: StremioItem) => item.kind === 'series' ? 'Series' : item.kind === 'movie' ? 'Movie' : 'Video';
  const label = (item: StremioItem) => `${item.name}, ${item.progress > 0 ? `${Math.round(item.progress)} percent watched` : kind(item)}`;
</script>

<section class="content-row" aria-label={title}>
  <div class="row-heading"><div><h2>{title}</h2><p>Synced from your Stremio account</p></div></div>
  <div class="card-track">
    {#each items as item (item.id)}
      <button class="media-card" aria-label={label(item)} onclick={() => open(item.id)}>
        <span class="poster">
          {#if item.poster}<img src={item.poster} alt="" loading="lazy" />{/if}
          {#if item.progress > 0}<span class="watch-progress" aria-hidden="true"><span style:width={`${Math.min(item.progress, 100)}%`}></span></span>{/if}
        </span>
        <span class="card-name">{item.name}</span>
        <span class="card-meta">{item.progress > 0 ? `${Math.round(item.progress)}% watched` : kind(item)}</span>
      </button>
    {/each}
  </div>
</section>
