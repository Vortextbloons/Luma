<script lang="ts">
  import type { StremioItem } from '../types';
  let { title, items, open, connected = true, connect }: { title: string; items: StremioItem[]; open: (id: string) => void; connected?: boolean; connect?: () => void } = $props();
  const kind = (item: StremioItem) => item.kind === 'series' ? 'Series' : item.kind === 'movie' ? 'Movie' : 'Video';
  const label = (item: StremioItem) => `${item.name}, ${item.progress > 0 ? `${Math.round(item.progress)} percent watched` : kind(item)}`;
</script>

<section class="content-row" aria-label={title}>
  <div class="row-heading"><div><h2>{title}</h2><p>Synced from your Stremio account</p></div></div>
  {#if !connected}
    <div class="activity-empty"><strong>Connect Stremio to continue watching</strong><span>Link your account to see your last watched titles and progress.</span>{#if connect}<button class="text-link" onclick={connect}>Connect Stremio</button>{/if}</div>
  {:else if items.length === 0}
    <div class="activity-empty"><strong>Nothing in progress on Stremio</strong><span>Titles with saved playback progress will appear here after your library syncs.</span></div>
  {:else}
  <div class="card-track">
    {#each items as item (item.id)}
      <button class="media-card" aria-label={label(item)} onclick={() => open(item.id)}>
        <span class="poster">
          {#if item.poster}<img src={item.poster} alt="" loading="lazy" decoding="async" />{/if}
          {#if item.progress > 0}<span class="watch-progress" aria-hidden="true"><span style:width={`${Math.min(item.progress, 100)}%`}></span></span>{/if}
        </span>
        <span class="card-name">{item.name}</span>
        <span class="card-meta">{item.progress > 0 ? `${Math.round(item.progress)}% watched` : kind(item)}</span>
      </button>
    {/each}
  </div>
  {/if}
</section>
