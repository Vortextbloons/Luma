<script lang="ts">
  let failed = $state<Record<number, number>>({});
  const covers = (item: SteamGame) => [item.cover, `https://shared.fastly.steamstatic.com/store_item_assets/steam/apps/${item.appid}/library_600x900.jpg`, `https://shared.fastly.steamstatic.com/store_item_assets/steam/apps/${item.appid}/header.jpg`];
  import type { SteamGame } from '../types';
  const playtime = (minutes: number) => minutes >= 60
    ? `${Math.floor(minutes / 60)}h ${minutes % 60}m played`
    : `${minutes}m played`;
  let { title = 'Continue playing', subtitle = 'Jump back into a game from your Steam library', items, open, onpreview }: {
    title?: string;
    subtitle?: string;
    items: SteamGame[];
    open: (appid: number) => void;
    onpreview?: (item: SteamGame | null) => void;
  } = $props();
</script>

<section class="content-row steam-row" aria-label={title}>
  <div class="row-heading"><div><h2>{title}</h2><p>{subtitle}</p></div></div>
  <div class="activity-track is-cinematic">
    {#each items as item (item.appid)}
      <button class="media-card" aria-label={item.playtimeMinutes > 0 ? `${item.name}, ${playtime(item.playtimeMinutes)}` : item.name} onclick={() => open(item.appid)} onmouseenter={() => onpreview?.(item)} onmouseleave={() => onpreview?.(null)} onfocus={() => onpreview?.(item)} onblur={() => onpreview?.(null)}>
        <span class="poster">
          {#if (failed[item.appid] ?? 0) < covers(item).length}
            <img src={covers(item)[failed[item.appid] ?? 0]} alt="" loading="lazy" decoding="async" onerror={() => failed[item.appid] = (failed[item.appid] ?? 0) + 1} />
          {/if}
          <span class="poster-copy">
            <span class="card-name">{item.name}</span>
            {#if item.playtimeMinutes > 0}<span class="card-meta">{playtime(item.playtimeMinutes)}</span>{/if}
          </span>
        </span>
      </button>
    {/each}
  </div>
</section>

<style>
  .poster { aspect-ratio: 16 / 9; }
  .poster > img { object-fit: contain; }
</style>
