<script lang="ts">
  import type { YouTubeFeedItem } from '../types';
  let { items, open, onpreview }: { items: YouTubeFeedItem[]; open: (id: string) => void; onpreview?: (item: YouTubeFeedItem | null) => void } = $props();
  const thumbnail = (id: string) => `https://i.ytimg.com/vi/${id}/hqdefault.jpg`;
</script>

<section id="youtube-feed" class="content-row" aria-label="Recommended on YouTube">
  <div class="video-track">
    {#each items.slice(0, 8) as item (item.id)}
      <button data-focus-key={item.id} class="video-card" onclick={() => open(item.id)} onmouseenter={() => onpreview?.(item)} onmouseleave={() => onpreview?.(null)} onfocus={() => onpreview?.(item)} onblur={() => onpreview?.(null)}>
        <span class="video-art"><img src={thumbnail(item.id)} alt="" loading="lazy" decoding="async" /></span>
        <span class="video-name">{item.title}</span>
      </button>
    {/each}
  </div>
</section>
