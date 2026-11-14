<script lang="ts">
  import spotify from '../../../assets/spotify.svg';
  import netflix from '../../../assets/netflix.svg';
  import disneyplus from '../../../assets/disneyplus.png';
  import primevideo from '../../../assets/primevideo.png';
  import appletv from '../../../assets/appletv.png';
  import hulu from '../../../assets/hulu.png';
  import max from '../../../assets/max.png';
  import peacock from '../../../assets/peacock.png';
  import stremio from '../../../assets/stremio.svg';
  import youtube from '../../../assets/youtube.svg';
  import steam from '../../../assets/steam.png';

  let { provider }: { provider: string } = $props();
  const source = $derived(provider === 'spotify' ? spotify : provider === 'stremio' ? stremio : provider === 'youtube' ? youtube : ({ netflix, disneyplus, primevideo, appletv, hulu, max, peacock, steam } as Record<string, string>)[provider] ?? '');
  const monogram = $derived(({
    plex: 'P', jellyfin: 'J', trakt: 'T', local: 'L',
  } as Record<string, string>)[provider] ?? provider.slice(0, 1).toUpperCase());
</script>

<span class={`provider-icon provider-${provider}`} class:has-image={source !== ''} aria-hidden="true">
  {#if source}<img src={source} alt="" loading="lazy" decoding="async" />{:else}<span>{monogram}</span>{/if}
</span>

<style>
  .provider-icon.has-image { background: transparent; }
  .provider-icon.has-image img { width: 100%; height: 100%; object-fit: contain; padding: 0; }
</style>
