<script lang="ts">
  import type { SteamGame } from '../types';
  import ProviderIcon from './activity/ProviderIcon.svelte';
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
  <div class="steam-track">
    {#each items as item (item.appid)}
      <button data-focus-key={String(item.appid)} class="steam-card" aria-label={item.playtimeMinutes > 0 ? `${item.name}, ${playtime(item.playtimeMinutes)}` : item.name} onclick={() => open(item.appid)} onmouseenter={() => onpreview?.(item)} onmouseleave={() => onpreview?.(null)} onfocus={() => onpreview?.(item)} onblur={() => onpreview?.(null)}>
        <span class="steam-art"><img src={item.header} alt="" loading="lazy" decoding="async" onerror={event => { (event.currentTarget as HTMLImageElement).style.display = 'none'; }} /></span>
        <span class="steam-icon">
          {#if item.icon}<img src={item.icon} alt="" loading="lazy" decoding="async" />
          {:else}<ProviderIcon provider="steam" />{/if}
        </span>
        <span class="steam-copy">
          <span class="steam-name">{item.name}</span>
          {#if item.playtimeMinutes > 0}<span class="steam-meta">{playtime(item.playtimeMinutes)}</span>{/if}
        </span>
      </button>
    {/each}
  </div>
</section>

<style>
  .steam-track {
    display: grid;
    grid-auto-flow: column;
    grid-auto-columns: clamp(200px, 16vw, 280px);
    gap: var(--gap);
    overflow-x: auto;
    overflow-y: hidden;
    padding: var(--track-pad);
    margin: calc(-1 * var(--track-pad));
    scrollbar-width: none;
    overscroll-behavior-x: contain;
  }
  .steam-track::-webkit-scrollbar { display: none; width: 0; height: 0; }
  .steam-card {
    position: relative;
    display: flex;
    flex-direction: column;
    align-items: stretch;
    gap: clamp(10px, .9vw, 14px);
    min-width: 0;
    padding: 0;
    overflow: hidden;
    border: 1px solid var(--line);
    border-radius: calc(var(--card-radius) + 2px);
    background: linear-gradient(120deg, var(--surface-3), var(--surface-2));
    box-shadow: 0 8px 22px #0003;
    text-align: left;
    transition: transform var(--motion-base) var(--ease-out), border-color var(--motion-base) var(--ease-out), box-shadow var(--motion-base) var(--ease-out);
  }
  .steam-card:hover { transform: translateY(-1px); border-color: var(--line-strong); box-shadow: 0 14px 30px #0005; }
  .steam-card:focus-visible { outline: 3px solid var(--accent); outline-offset: 4px; transform: translateY(-2px); }
  .steam-art { display: block; aspect-ratio: 460 / 215; background: var(--surface-3); }
  .steam-art img { width: 100%; height: 100%; object-fit: contain; }
  .steam-icon {
    flex: none;
    display: grid;
    place-items: center;
    position: absolute;
    bottom: 12px;
    left: 12px;
    width: 32px;
    height: 32px;
    overflow: hidden;
    border-radius: 10px;
    background: #0b0d12;
  }
  .steam-icon img { width: 100%; height: 100%; object-fit: contain; }
  .steam-icon :global(.provider-icon) { width: 72%; height: 72%; background: transparent; }
  .steam-copy { padding: 0 12px 12px 54px; min-height: 44px; justify-content: center; display: flex; flex-direction: column; gap: 3px; min-width: 0; }
  .steam-name {
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
    font-size: clamp(13px, .92vw, 16px);
    font-weight: 650;
    line-height: 1.25;
    letter-spacing: -.015em;
  }
  .steam-meta { color: var(--muted); font-size: clamp(11px, .74vw, 13px); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
</style>

