<script lang="ts">
  import MusicIcon from './MusicIcon.svelte';
  let { artworkUrl, title, subtitle, onselect, onpreview, onpreviewend, kind = 'Album' }: {
    artworkUrl?: string;
    title: string;
    subtitle?: string;
    onselect: () => void;
    onpreview?: () => void;
    onpreviewend?: () => void;
    kind?: string;
  } = $props();
</script>

<button class="music-card" onclick={onselect} aria-label={`${kind}: ${title}`} onmouseenter={() => onpreview?.()} onmouseleave={() => onpreviewend?.()} onfocus={() => onpreview?.()} onblur={() => onpreviewend?.()}>
  <span class="music-card-art">
    {#if artworkUrl}<img src={artworkUrl} alt="" loading="lazy" decoding="async" />{:else}<span class="music-card-placeholder" aria-hidden="true"><MusicIcon name="music" /></span>{/if}
    <span class="music-card-copy">
      <span class="music-card-name">{title}</span>
      {#if subtitle}<span class="music-card-subtitle">{subtitle}</span>{/if}
    </span>
  </span>
</button>

<style>
  .music-card { border-radius: 12px; outline-offset: 6px; } .music-card:focus-visible { outline: 3px solid var(--accent); }
  .music-card-art { position: relative; width: 100%; border-radius: 10px; } .music-card-art img { object-fit: contain; }
  .music-card-copy { position: absolute; left: 0; right: 0; bottom: 0; display: flex; flex-direction: column; padding: 36px 12px 13px; background: linear-gradient(0deg, #05070cf2 16%, #05070ca8 52%, transparent); text-align: left; }
  .music-card-name { font-size: clamp(15px,1.05vw,19px); margin: 0; text-shadow: 0 2px 10px #000c; } .music-card-subtitle { font-size: clamp(12px,.85vw,15px); margin-top: 2px; color: var(--text-dim); text-shadow: 0 2px 8px #000c; }
</style>
