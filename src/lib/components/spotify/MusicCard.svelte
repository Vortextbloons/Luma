<script lang="ts">
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
    {#if artworkUrl}<img src={artworkUrl} alt="" loading="lazy" />{:else}<span class="music-card-placeholder" aria-hidden="true">♪</span>{/if}
  </span>
  <span class="music-card-name">{title}</span>
  {#if subtitle}<span class="music-card-subtitle">{subtitle}</span>{/if}
</button>
