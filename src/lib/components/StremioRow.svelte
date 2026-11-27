<script lang="ts">
  import type { StremioItem } from '../types';
  import ProviderIcon from './activity/ProviderIcon.svelte';
  import PosterImage from './PosterImage.svelte';
  import EmptyState from './EmptyState.svelte';
  let { title, items, open, connected = true, connect, onpreview, cinematic = false }: { title: string; items: StremioItem[]; open: (id: string) => void; connected?: boolean; connect?: () => void; onpreview?: (item: StremioItem | null) => void; cinematic?: boolean } = $props();
  const kind = (item: StremioItem) => item.kind === 'series' ? 'Series' : item.kind === 'movie' ? 'Movie' : 'Video';
  const label = (item: StremioItem) => `${item.name}, ${item.progress > 0 ? `${Math.round(item.progress)} percent watched` : kind(item)}`;
</script>

<section class="content-row" class:cinematic aria-label={title}>
  <div class="row-heading"><div><h2>{title}</h2><p>Synced from your Stremio account</p></div></div>
  {#if !connected}
    <EmptyState title="Connect Stremio to continue watching" description="Link your account to see your last watched titles and progress.">
      {#snippet actions()}{#if connect}<button class="text-link" onclick={connect}>Connect Stremio</button>{/if}{/snippet}
    </EmptyState>
  {:else if items.length === 0}
    <EmptyState title="Nothing in progress on Stremio" description="Titles with saved playback progress will appear here after your library syncs." />
  {:else}
  <div class={cinematic ? 'activity-track is-cinematic' : 'card-track'}>
    {#each items as item (item.id)}
      <button data-focus-key={item.id} class="media-card" aria-label={label(item)} onclick={() => open(item.id)} onmouseenter={() => onpreview?.(item)} onmouseleave={() => onpreview?.(null)} onfocus={() => onpreview?.(item)} onblur={() => onpreview?.(null)}>
        <span class="poster">
          {#if cinematic && item.poster}<span class="poster-fill" style:background-image={`url("${item.poster}")`} aria-hidden="true"></span>{/if}
          {#if item.poster}<PosterImage src={item.poster} show={item.kind === 'series'} />{/if}
          {#if cinematic}<span class="activity-provider"><ProviderIcon provider="stremio" /><span>Stremio</span></span>{/if}
          {#if item.progress > 0}<span class="watch-progress" aria-hidden="true"><span style:width={`${Math.min(item.progress, 100)}%`}></span></span>{/if}
          <span class="poster-copy">
            <span class="card-name">{item.name}</span>
            <span class="card-meta">{item.progress > 0 ? `${Math.round(item.progress)}% watched` : kind(item)}</span>
          </span>
        </span>
      </button>
    {/each}
  </div>
  {/if}
</section>

<style>
  .cinematic .poster { aspect-ratio: 16 / 9; }
  .cinematic .poster-fill { position: absolute; inset: 0; background-position: center; background-size: cover; filter: blur(20px) saturate(1.15); opacity: .55; transform: scale(1.2); }
  .cinematic .poster :global(.poster-image) { position: absolute; }
  .cinematic .poster :global(.poster-image img) { object-fit: contain; }
  .cinematic .poster-copy { padding: 36px 12px 13px; }
  .cinematic .poster .card-name { font-size: clamp(14px, .92vw, 17px); line-height: 1.22; }
</style>
