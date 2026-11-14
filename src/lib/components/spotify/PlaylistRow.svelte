<script lang="ts">
  import MusicCard from './MusicCard.svelte';
  import type { MusicCardItem } from '../../types/spotify';
  let { title, subtitle, items, empty, kind = 'Playlist', onselect }: {
    title: string;
    subtitle?: string;
    items: MusicCardItem[];
    empty: string;
    kind?: string;
    onselect: (item: MusicCardItem) => void;
  } = $props();
</script>

<section class="content-row music-row">
  <div class="row-heading"><div><h2>{title}</h2>{#if subtitle}<p>{subtitle}</p>{/if}</div></div>
  {#if items.length}
    <div class="music-card-track">
      {#each items as item (item.id)}
        <MusicCard artworkUrl={item.artworkUrl} title={item.title} subtitle={item.subtitle} kind={kind} onselect={() => onselect(item)} />
      {/each}
    </div>
  {:else}
    <p class="music-empty-line">{empty}</p>
  {/if}
</section>
