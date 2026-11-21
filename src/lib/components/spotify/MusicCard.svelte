<script lang="ts">
  import MusicIcon from './MusicIcon.svelte';
  let { artworkUrl, title, subtitle, onselect, onpreview, onpreviewend, kind = 'Album', focusKey }: {
    focusKey?: string;
    artworkUrl?: string;
    title: string;
    subtitle?: string;
    onselect: () => void;
    onpreview?: () => void;
    onpreviewend?: () => void;
    kind?: string;
  } = $props();
</script>

<button class="music-card" data-focus-key={focusKey} onclick={onselect} aria-label={`${kind}: ${title}`} onmouseenter={() => onpreview?.()} onmouseleave={() => onpreviewend?.()} onfocus={() => onpreview?.()} onblur={() => onpreviewend?.()}>
  <span class="music-card-media">
    <span class="music-card-art">
      {#if artworkUrl}<img src={artworkUrl} alt="" loading="lazy" decoding="async" />{:else}<span class="music-card-placeholder" aria-hidden="true"><MusicIcon name="music" /></span>{/if}
    </span>
    <span class="music-card-glyph" aria-hidden="true"><MusicIcon name={kind === 'Track' ? 'play' : 'chevron-right'} size={16} /></span>
  </span>
  <span class="music-card-copy">
    <span class="music-card-name">{title}</span>
    {#if subtitle}<span class="music-card-subtitle">{subtitle}</span>{/if}
  </span>
</button>

<style>
  .music-card { display: flex; flex-direction: column; gap: 9px; align-items: stretch; padding: 0; background: transparent; border: 0; text-align: left; min-width: 0; border-radius: 12px; }
  .music-card-media { display: block; position: relative; }
  .music-card:focus-visible { outline: 2px solid var(--accent); outline-offset: 4px; }
</style>
