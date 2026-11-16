<script lang="ts">
  import type { MediaItem } from '../types';
  let { title, items, id, onselect, onpreview }: {
    title: string;
    items: MediaItem[];
    id: string;
    onselect: (item: MediaItem) => void;
    onpreview?: (item: MediaItem | null) => void;
  } = $props();
  const rowArt = (item: MediaItem) => (item.backdrop || item.poster).replace('/background/medium/', '/background/small/');
  const rowArtSet = (item: MediaItem) => `${rowArt(item)} 1x, ${item.backdrop || item.poster} 2x`;
</script>

<section {id} class="content-row" aria-label={title}>
  <div class="row-heading"><div><h2>{title}</h2></div></div>
  <div class="backdrop-track">
    {#each items as item (item.id)}
      <button data-focus-key={item.id} class="backdrop-card" aria-label={`${item.title}, ${item.subtitle}`} onclick={() => onselect(item)} onmouseenter={() => onpreview?.(item)} onmouseleave={() => onpreview?.(null)} onfocus={() => onpreview?.(item)} onblur={() => onpreview?.(null)}>
        <span class="backdrop-art">
          <img src={rowArt(item)} srcset={rowArtSet(item)} alt="" loading="lazy" decoding="async" />
          <span class="backdrop-fade" aria-hidden="true"></span>
          <span class="backdrop-name">{item.title}</span>
        </span>
      </button>
    {/each}
  </div>
</section>

<style>
  .backdrop-track {
    display: grid;
    grid-auto-flow: column;
    grid-auto-columns: clamp(250px, 22vw, 380px);
    gap: var(--gap);
    overflow-x: auto;
    overflow-y: hidden;
    padding: var(--track-pad);
    margin: calc(-1 * var(--track-pad));
    scrollbar-width: none;
    overscroll-behavior-x: contain;
  }
  .backdrop-track::-webkit-scrollbar { display: none; width: 0; height: 0; }
  .backdrop-card {
    min-width: 0;
    padding: 0;
    background: transparent;
    text-align: left;
    border-radius: calc(var(--card-radius) + 2px);
    transition: transform var(--motion-base) var(--ease-out);
  }
  .backdrop-art {
    position: relative;
    display: block;
    aspect-ratio: 16 / 9;
    overflow: hidden;
    border: 1px solid var(--line);
    border-radius: calc(var(--card-radius) + 2px);
    background: var(--surface);
    box-shadow: 0 10px 26px #0005;
    transition: border-color var(--motion-base) var(--ease-out), box-shadow var(--motion-base) var(--ease-out);
  }
  .backdrop-art img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
    filter: saturate(.92);
    transition: transform .25s var(--ease-out), filter .25s var(--ease-out);
  }
  .backdrop-card:hover .backdrop-art,
  .backdrop-card:focus-visible .backdrop-art {
    border-color: #ffffff33;
    box-shadow: 0 18px 40px #0008;
  }
  .backdrop-card:hover .backdrop-art img,
  .backdrop-card:focus-visible .backdrop-art img {
    transform: scale(1.04);
    filter: saturate(1.03);
  }
  .backdrop-card:hover { transform: translateY(-2px); }
  .backdrop-card:focus-visible {
    outline: 3px solid var(--accent);
    outline-offset: 4px;
    box-shadow: var(--focus-glow);
    transform: translateY(-3px);
  }
  .backdrop-card:focus-visible .backdrop-art {
    border-color: color-mix(in srgb, var(--accent) 55%, transparent);
  }
  .backdrop-fade {
    position: absolute;
    inset: 0;
    background: linear-gradient(0deg, #05070cd9 0%, #05070c4d 38%, transparent 66%);
  }
  .backdrop-name {
    position: absolute;
    left: 16px;
    right: 16px;
    bottom: 14px;
    color: var(--text);
    font-size: clamp(15px, 1.05vw, 19px);
    font-weight: 650;
    letter-spacing: -.02em;
    line-height: 1.2;
    text-shadow: 0 2px 12px #000c;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }
</style>
