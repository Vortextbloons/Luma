<script lang="ts">
  import spotify from '../../../assets/spotify.jpg';
  import netflix from '../../../assets/netflix.png';
  import disneyplus from '../../../assets/disneyplus.png';
  import primevideo from '../../../assets/primevideo.png';
  import appletv from '../../../assets/appletv.png';
  import hulu from '../../../assets/hulu.png';
  import max from '../../../assets/max.jpg';
  import peacock from '../../../assets/peacock.png';
  import stremio from '../../../assets/stremio.svg';
  import youtube from '../../../assets/youtube.jpg';

  let { provider }: { provider: string } = $props();
  const source = $derived(provider === 'spotify' ? spotify : provider === 'stremio' ? stremio : provider === 'youtube' ? youtube : ({ netflix, disneyplus, primevideo, appletv, hulu, max, peacock } as Record<string, string>)[provider] ?? '');
  const monogram = $derived(({
    plex: 'P', jellyfin: 'J', trakt: 'T', local: 'L',
  } as Record<string, string>)[provider] ?? provider.slice(0, 1).toUpperCase());
</script>

<span class={`provider-icon provider-${provider}`} aria-hidden="true">
  {#if source}<img src={source} alt="" loading="lazy" />{:else}<span>{monogram}</span>{/if}
</span>
