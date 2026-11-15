<script module lang="ts">
  import type { ActivityItem, AppId, HomeRow, HomeRowId, MediaItem, SteamGame, StremioItem, StremioStatus, YouTubeFeedItem } from '../types';
  import type { MusicCardItem } from '../types/spotify';
  import type { HeroSlide } from './MovieHero.svelte';
  export interface HomeData {
    heroSlides: HeroSlide[];
    heroSource: string;
    previewApp: AppId | null;
    stremioRecent: StremioItem[];
    stremio: StremioStatus | null;
    steamGames: SteamGame[];
    activityItems: ActivityItem[];
    trackingEnabled: boolean;
    order: AppId[];
    launching: AppId | null;
    catalogVisible: boolean;
    youTubeFeed: YouTubeFeedItem[];
    previewMusic: MusicCardItem[];
    steamRecommendations: SteamGame[];
    steamRecommendationSource: SteamGame | null;
    previewActivity: ActivityItem[];
    musicLoading: boolean;
    popular: MediaItem[];
    featured: MediaItem[];
  }
  export interface HomeActions {
    openStremio: (id: string) => void;
    openStremioPairing: () => void;
    previewStremio: (item: StremioItem | null) => void;
    openSteamGame: (id: number) => void;
    previewSteamGame: (item: SteamGame | null) => void;
    openItem: (item: ActivityItem) => void;
    removeItem: (item: ActivityItem) => void;
    previewActivityItem: (item: ActivityItem | null) => void;
    launch: (id: AppId) => void;
    preview: (id: AppId) => void;
    openYouTubeItem: (id: string) => void;
    previewVideo: (item: YouTubeFeedItem | null) => void;
    playMusic: (item: MusicCardItem) => void;
    previewMusicItem: (item: MusicCardItem | null) => void;
    openSteamStore: (id: number) => void;
    previewSteamRecommendation: (item: SteamGame | null) => void;
    select: (item: MediaItem) => void;
    previewMovie: (item: MediaItem | null, eyebrow?: string) => void;
  }
</script>

<script lang="ts">
  import type { Snippet } from 'svelte';
  import MovieHero from './MovieHero.svelte';
  import BackdropRow from './BackdropRow.svelte';
  import VideoRow from './VideoRow.svelte';
  import PlaylistRow from './spotify/PlaylistRow.svelte';
  import ActivityCard from './activity/ActivityCard.svelte';
  import AppDock from './AppDock.svelte';
  import StremioRow from './StremioRow.svelte';
  import SteamRow from './SteamRow.svelte';
  import ContinueWatching from './activity/ContinueWatching.svelte';
  import RecentlyPlayed from './activity/RecentlyPlayed.svelte';
  import RecentApps from './activity/RecentApps.svelte';
  import { appProviders } from '../services/activity';
  import { smoothHeight } from '../services/motion';
  import { preserveHomeFocus } from '../services/navigation';
  let { rows, data, actions }: { rows: HomeRow[]; data: HomeData; actions: HomeActions } = $props();
  const registry: Record<HomeRowId, Snippet<[boolean]>> = { hero, continue: continuing, apps, popular, featured, recent, recentApps };
  const visible = $derived(rows.filter(row => row.visible));
  const leading = $derived(visible[0]?.component === 'hero' ? visible[0] : null);
  const content = $derived(leading ? visible.slice(1) : visible);
</script>

{#snippet hero(inset: boolean)}
  {#if data.heroSlides.length}<MovieHero slides={data.heroSlides} sourceKey={data.heroSource} {inset} />{/if}
{/snippet}

{#snippet continuing()}
  {#if data.previewApp === 'Stremio'}
    <div class="view-swap"><StremioRow title="Recently watched on Stremio" items={data.stremioRecent.slice(0, 4)} open={actions.openStremio} connected={data.stremio?.connected ?? false} connect={actions.openStremioPairing} onpreview={actions.previewStremio} cinematic /></div>
  {:else if data.previewApp === 'Steam' && data.steamGames.length}
    <div class="view-swap"><SteamRow items={data.steamGames} open={actions.openSteamGame} onpreview={actions.previewSteamGame} /></div>
  {:else if data.previewApp !== 'Steam'}
    <ContinueWatching items={data.activityItems} trackingEnabled={data.trackingEnabled} onopen={actions.openItem} focusProvider={data.previewApp ? appProviders[data.previewApp] : null} onpreview={actions.previewActivityItem} />
  {/if}
{/snippet}

{#snippet apps()}
  <AppDock order={data.order} launch={actions.launch} launching={data.launching} preview={actions.preview} active={data.previewApp} />
  {#if data.previewApp && data.catalogVisible}
    {#key data.previewApp}
      <div class="view-swap">
        {#if data.previewApp === 'YouTube' && data.youTubeFeed.length}
          <VideoRow items={data.youTubeFeed} open={actions.openYouTubeItem} onpreview={actions.previewVideo} />
        {:else if data.previewApp === 'Spotify' && data.previewMusic.length}
          <PlaylistRow title="Recently played on Spotify" items={data.previewMusic} empty="" kind="Track" onselect={actions.playMusic} onpreview={actions.previewMusicItem} />
        {:else if data.previewApp === 'Stremio' && data.stremio?.items.length}
          <StremioRow title="Your Stremio library" items={data.stremio.items.slice(0, 10)} open={actions.openStremio} onpreview={actions.previewStremio} />
        {:else if data.previewApp === 'Steam' && data.steamRecommendations.length}
          <SteamRow title="Recommendations" subtitle={data.steamRecommendationSource ? `More games like ${data.steamRecommendationSource.name}` : 'More games from the Steam store'} items={data.steamRecommendations} open={actions.openSteamStore} onpreview={actions.previewSteamRecommendation} />
        {:else if data.previewActivity.length}
          <section class="content-row"><div class="row-heading"><h2>From {data.previewApp}</h2></div><div class="activity-track">{#each data.previewActivity as item (item.id)}<ActivityCard {item} onopen={actions.openItem} onremove={actions.removeItem} onpreview={actions.previewActivityItem} />{/each}</div></section>
        {:else}
          <section class="content-row" aria-label={`${data.previewApp} recommendations`}><div class="row-heading"><h2>From {data.previewApp === 'Custom' ? 'your custom app' : data.previewApp}</h2></div><div class="activity-empty"><strong>{data.previewApp === 'Spotify' && data.musicLoading ? 'Loading your music...' : data.previewApp === 'Steam' ? (data.steamGames.length ? 'No recommendations right now' : 'No games to continue') : 'No recommendations available yet'}</strong><span>{data.previewApp === 'YouTube' ? 'Open YouTube to load its recommendations.' : data.previewApp === 'Steam' ? (data.steamGames.length ? 'Steam recommendations could not load. Check your connection and hover Steam again.' : 'Play a game through Steam, then hover Steam here again to see it.') : 'Content from this app will appear here when available.'}</span><button class="text-link" onclick={() => data.previewApp && actions.launch(data.previewApp)}>Open {data.previewApp === 'Custom' ? 'custom app' : data.previewApp}</button></div></section>
        {/if}
      </div>
    {/key}
  {/if}
{/snippet}

{#snippet popular()}
  {#if !data.previewApp}<BackdropRow id="popular" title="Popular movies" items={data.popular} onselect={actions.select} onpreview={item => actions.previewMovie(item, 'POPULAR MOVIE')} />{/if}
{/snippet}

{#snippet featured()}
  {#if !data.previewApp}<BackdropRow id="featured" title="Top-rated movies" items={data.featured} onselect={actions.select} onpreview={item => actions.previewMovie(item, 'TOP-RATED MOVIE')} />{/if}
{/snippet}

{#snippet recent()}
  <RecentlyPlayed items={data.activityItems} trackingEnabled={data.trackingEnabled} onopen={actions.openItem} onremove={actions.removeItem} onpreview={actions.previewActivityItem} />
{/snippet}

{#snippet recentApps()}
  <RecentApps items={data.activityItems} trackingEnabled={data.trackingEnabled} onopen={actions.openItem} onremove={actions.removeItem} onpreview={actions.previewActivityItem} />
{/snippet}

<div class="home-layout" {@attach preserveHomeFocus}>
  {#if leading}
    <div class="home-instance" data-home-instance={leading.id}>
      {@render registry[leading.component](false)}
    </div>
  {/if}
  <div {@attach smoothHeight}><div class="resizing-content">
    {#each content as row (row.id)}
      <div class="home-instance" data-home-instance={row.id}>
        {@render registry[row.component](true)}
      </div>
    {/each}
  </div></div>
</div>

<style>
  .home-layout, .home-instance { display: contents; }
  .resizing-content { display: flow-root; }
  .home-instance:has(:global(.activity-row), :global(.steam-row)) + .home-instance :global(.apps-row) { margin-top: calc(6px - var(--row-gap)); }
</style>
