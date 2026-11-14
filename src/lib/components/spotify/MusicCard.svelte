<script lang="ts">
  import MusicIcon from './MusicIcon.svelte';
  let { artworkUrl, title, subtitle, onselect, kind = 'Album' }: {
    artworkUrl?: string;
    title: string;
    subtitle?: string;
    onselect: () => void;
    kind?: string;
  } = $props();
</script>

<button class="music-card" onclick={onselect} aria-label={`${kind}: ${title}`}>
  <span class="music-card-art">
    {#if artworkUrl}<img src={artworkUrl} alt="" loading="lazy" />{:else}<span class="music-card-placeholder" aria-hidden="true"><MusicIcon name="music" /></span>{/if}
  </span>
  <span class="music-card-name">{title}</span>
  {#if subtitle}<span class="music-card-subtitle">{subtitle}</span>{/if}
</button>

<style>
  .music-card { border-radius: 12px; outline-offset: 6px; } .music-card:focus-visible { outline: 3px solid var(--accent); }
  .music-card-art { width: 100%; border-radius: 10px; } .music-card-art img { object-fit: contain; }
  .music-card-name { font-size: clamp(18px,1.4vw,26px); margin-top: 16px; } .music-card-subtitle { font-size: clamp(16px,1.1vw,20px); margin-top: 6px; color: #aab6cd; }
</style>
