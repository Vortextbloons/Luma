<script lang="ts">
  import spotify from '../../../assets/spotify.svg';
  import stremio from '../../../assets/stremio.svg';
  import youtube from '../../../assets/youtube.png';

  let { provider }: { provider: string } = $props();
  const source = $derived(provider === 'spotify' ? spotify : provider === 'stremio' ? stremio : provider === 'youtube' ? youtube : '');
  const monogram = $derived(({
    plex: 'P', jellyfin: 'J', trakt: 'T', local: 'L', netflix: 'N',
    disneyplus: 'D+', primevideo: 'P', hulu: 'H', max: 'M', peacock: 'P', appletv: 'TV',
  } as Record<string, string>)[provider] ?? provider.slice(0, 1).toUpperCase());
</script>

<span class={`provider-icon provider-${provider}`} aria-hidden="true">
  {#if source}<img src={source} alt="" loading="lazy" />{:else}<span>{monogram}</span>{/if}
</span>
