<script lang="ts">
  import type { StremioItem } from '../types';
  import ProviderIcon from './activity/ProviderIcon.svelte';
  let { title, items, open, connected = true, connect, onpreview, cinematic = false }: { title: string; items: StremioItem[]; open: (id: string) => void; connected?: boolean; connect?: () => void; onpreview?: (item: StremioItem | null) => void; cinematic?: boolean } = $props();
  const kind = (item: StremioItem) => item.kind === 'series' ? 'Series' : item.kind === 'movie' ? 'Movie' : 'Video';
  const label = (item: StremioItem) => `${item.name}, ${item.progress > 0 ? `${Math.round(item.progress)} percent watched` : kind(item)}`;
</script>

<section class="content-row" class:cinematic aria-label={title}>
  <div class="row-heading"><div><h2>{title}</h2><p>Synced from your Stremio account</p></div></div>
  {#if !connected}
    <div class="activity-empty"><strong>Connect Stremio to continue watching</strong><span>Link your account to see your last watched titles and progress.</span>{#if connect}<button class="text-link" onclick={connect}>Connect Stremio</button>{/if}</div>
  {:else if items.length === 0}
    <div class="activity-empty"><strong>Nothing in progress on Stremio</strong><span>Titles with saved playback progress will appear here after your library syncs.</span></div>
  {:else}
  <div class={cinematic ? 'activity-track is-cinematic' : 'card-track'}>
    {#each items as item (item.id)}
      <button class="media-card" aria-label={label(item)} onclick={() => open(item.id)} onmouseenter={() => onpreview?.(item)} onmouseleave={() => onpreview?.(null)} onfocus={() => onpreview?.(item)} onblur={() => onpreview?.(null)}>
        <span class="poster">
          {#if cinematic && item.poster}<span class="poster-fill" style:background-image={`url("${item.poster}")`} aria-hidden="true"></span>{/if}
          {#if item.poster}<img src={item.poster} alt="" loading="lazy" decoding="async" />{/if}
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
  .cinematic .poster > img { position: relative; object-fit: contain; }
  .cinematic .poster-copy { padding: 36px 12px 13px; }
  .cinematic .poster .card-name { font-size: clamp(14px, .92vw, 17px); line-height: 1.22; }
</style>
