<script lang="ts">
  import MusicCard from './MusicCard.svelte';
  import MusicIcon from './MusicIcon.svelte';
  import type { MusicCardItem } from '../../types/spotify';
  let { title, subtitle, items, empty, kind = 'Playlist', onselect, onmore, onpreview }: {
    title: string;
    subtitle?: string;
    items: MusicCardItem[];
    empty: string;
    kind?: string;
    onselect: (item: MusicCardItem) => void;
    onmore?: () => void;
    onpreview?: (item: MusicCardItem | null) => void;
  } = $props();
</script>

<section class="content-row music-row">
  <div class="row-heading">
    <div><h2>{title}</h2>{#if subtitle}<p>{subtitle}</p>{/if}</div>
    {#if onmore && items.length}<button class="music-more" onclick={onmore}>See all <MusicIcon name="chevron-right" size={14} /></button>{/if}
  </div>
  {#if items.length}
    <div class="music-card-track">
      {#each items as item (item.id)}
        <MusicCard focusKey={item.id} artworkUrl={item.artworkUrl} title={item.title} subtitle={item.subtitle} kind={kind} onselect={() => onselect(item)} onpreview={() => onpreview?.(item)} onpreviewend={() => onpreview?.(null)} />
      {/each}
    </div>
  {:else}
    <p class="music-empty-line">{empty}</p>
  {/if}
</section>

<style>
  .music-card-track { display: grid; grid-auto-flow: column; grid-auto-columns: clamp(150px,12.4vw,208px); grid-template-columns: none; gap: clamp(14px,1.4vw,24px); overflow-x: auto; padding: 8px; margin: -8px; scrollbar-width: none; }
  .row-heading h2 { font-size: clamp(18px,1.45vw,26px); font-weight: 700; letter-spacing: -.035em; }
  .music-more { display: inline-flex; align-items: center; gap: 2px; }
</style>
