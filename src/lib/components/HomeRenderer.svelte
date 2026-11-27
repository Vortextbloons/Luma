<script module lang="ts">
  import type { ActivityItem, AppId, HomeRow, HomeRowId, MediaItem, SteamGame, StremioItem, StremioStatus, YouTubeFeedItem } from '../types';
  import type { MusicCardItem } from '../types/spotify';
  import type { GameScore } from '../services/sportsScores';
  import type { PluginHomeCard, PluginHomeCardFailure } from '../services/plugins';
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
    smoothAppHover?: boolean;
    catalogVisible: boolean;
    youTubeFeed: YouTubeFeedItem[];
    previewMusic: MusicCardItem[];
    steamRecommendations: SteamGame[];
    steamRecommendationSource: SteamGame | null;
    previewActivity: ActivityItem[];
    musicLoading: boolean;
    popular: MediaItem[];
    featured: MediaItem[];
    previewLeague: AppId | null;
    leagueGames: GameScore[];
    leagueGamesLoading: boolean;
    homeCards: PluginHomeCard[];
    pluginCardPending: string | null;
    pluginCardFailure: PluginHomeCardFailure | null;
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
    watchLeagueGame: (game: GameScore) => void;
    previewLeagueGame: (game: GameScore | null) => void;
    openYouTubeItem: (id: string) => void;
    previewVideo: (item: YouTubeFeedItem | null) => void;
    playMusic: (item: MusicCardItem) => void;
    previewMusicItem: (item: MusicCardItem | null) => void;
    openSteamStore: (id: number) => void;
    previewSteamRecommendation: (item: SteamGame | null) => void;
    select: (item: MediaItem) => void;
    previewMovie: (item: MediaItem | null, eyebrow?: string) => void;
    openPluginCard: (card: PluginHomeCard) => void;
    retryPluginCard: () => void;
  }
</script>

<script lang="ts">
  import type { Snippet } from 'svelte';
  import MovieHero from './MovieHero.svelte';
  import BackdropRow from './BackdropRow.svelte';
  import EmptyState from './EmptyState.svelte';
  import VideoRow from './VideoRow.svelte';
  import PlaylistRow from './spotify/PlaylistRow.svelte';
  import ActivityCard from './activity/ActivityCard.svelte';
  import AppDock from './AppDock.svelte';
  import StremioRow from './StremioRow.svelte';
  import SteamRow from './SteamRow.svelte';
  import ContinueWatching from './activity/ContinueWatching.svelte';
  import LeagueGamesRow from './LeagueGamesRow.svelte';
  import RecentlyPlayed from './activity/RecentlyPlayed.svelte';
  import RecentApps from './activity/RecentApps.svelte';
  import { appProviders } from '../services/activity';
  import { smoothHeight } from '../services/motion';
  import { preserveHomeFocus } from '../services/navigation';
  let { rows, data, actions, columns = 1 }: { rows: HomeRow[]; data: HomeData; actions: HomeActions; columns?: 1 | 2 } = $props();
  const smoothHover = $derived(data.smoothAppHover ?? true);
  const appClass = (id: AppId) => id.toLowerCase().replace(/\+/g, 'plus').replace(/\s+/g, '-');
  const registry: Record<HomeRowId, Snippet<[boolean]>> = { hero, continue: continuing, apps, popular, featured, recent, recentApps };
  const visible = $derived(rows.filter(row => row.visible));
  const leading = $derived(visible[0]?.component === 'hero' ? visible[0] : null);
  const content = $derived((leading ? visible.slice(1) : visible).filter(row => {
    if (row.component === 'hero') return data.heroSlides.length > 0;
    if (row.component === 'continue') return data.previewApp !== 'Steam' || data.steamGames.length > 0;
    if (row.component === 'popular' || row.component === 'featured') return !data.previewApp;
    return true;
  }));
  const cardFailure = $derived.by(() => {
    const failure = data.pluginCardFailure;
    return failure && data.homeCards.some(card => card.id === failure.card.id) ? failure : null;
  });
</script>

{#snippet hero(inset: boolean)}
  {#if data.heroSlides.length}<MovieHero slides={data.heroSlides} sourceKey={data.heroSource} {inset} />{/if}
{/snippet}

{#snippet previewOpenAction()}
  <button class="text-link" onclick={() => data.previewApp && actions.launch(data.previewApp)}>Open {data.previewApp === 'Custom' ? 'custom app' : data.previewApp}</button>
{/snippet}

{#snippet continuing()}
  <div class={['app-continue-shelf', smoothHover && 'is-smooth', smoothHover && data.previewApp && appClass(data.previewApp)]}>
    {#if data.previewLeague}
      {#key data.previewLeague}
        <div class={smoothHover ? 'preview-swap' : undefined}>
          <LeagueGamesRow league={data.previewLeague} games={data.leagueGames} loading={data.leagueGamesLoading} watch={actions.watchLeagueGame} onpreview={actions.previewLeagueGame} />
        </div>
      {/key}
    {:else if data.previewApp === 'Stremio'}
      <div class={smoothHover ? 'preview-swap' : 'view-swap'}><StremioRow title="Recently watched on Stremio" items={data.stremioRecent.slice(0, 4)} open={actions.openStremio} connected={data.stremio?.connected ?? false} connect={actions.openStremioPairing} onpreview={actions.previewStremio} cinematic /></div>
    {:else if data.previewApp === 'Steam' && data.steamGames.length}
      <div class={smoothHover ? 'preview-swap' : 'view-swap'}><SteamRow items={data.steamGames} open={actions.openSteamGame} onpreview={actions.previewSteamGame} /></div>
    {:else if data.previewApp !== 'Steam'}
      <div class={smoothHover ? 'preview-swap' : undefined}>
        <ContinueWatching items={data.activityItems} trackingEnabled={data.trackingEnabled} onopen={actions.openItem} focusProvider={data.previewApp ? appProviders[data.previewApp] : null} onpreview={actions.previewActivityItem} />
      </div>
    {/if}
  </div>
{/snippet}

{#snippet apps()}
  <AppDock order={data.order} launch={actions.launch} launching={data.launching} preview={actions.preview} active={data.previewApp} {smoothHover} />
  {#if data.previewApp && data.catalogVisible && !data.previewLeague}
    <div class={['app-preview-shelf', smoothHover && `is-smooth ${appClass(data.previewApp)}`]}>
      {#key data.previewApp}
        <div class={smoothHover ? 'preview-swap' : 'view-swap'}>
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
          <section class="content-row" aria-label={`${data.previewApp} recommendations`}><div class="row-heading"><h2>From {data.previewApp === 'Custom' ? 'your custom app' : data.previewApp}</h2></div>
            <EmptyState
              title={data.previewApp === 'Spotify' && data.musicLoading ? 'Loading your music...' : data.previewApp === 'Steam' ? (data.steamGames.length ? 'No recommendations right now' : 'No games to continue') : 'No recommendations available yet'}
              description={data.previewApp === 'YouTube' ? 'Open YouTube to load its recommendations.' : data.previewApp === 'Steam' ? (data.steamGames.length ? 'Steam recommendations could not load. Check your connection and hover Steam again.' : 'Play a game through Steam, then hover Steam here again to see it.') : 'Content from this app will appear here when available.'}
              actions={previewOpenAction} />
          </section>
        {/if}
        </div>
      {/key}
    </div>
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
  {#if leading && data.heroSlides.length}
    <div class="home-instance" data-home-instance={leading.id}>
      {@render registry[leading.component](false)}
    </div>
  {/if}
  <div {@attach smoothHeight}><div class="resizing-content" class:columns={columns === 2}>
    {#each content as row (row.id)}
      <div class="home-instance" data-home-instance={row.id} data-span={row.span}>
        {@render registry[row.component](true)}
      </div>
    {/each}
    {#if data.homeCards.length}
      <div class="home-instance" data-home-instance="plugins" data-span="2">
        <section class="content-row plugin-home-row" aria-label="Plugin cards">
          <div class="row-heading"><div><h2>From your plugins</h2></div></div>
          <div class="app-track">
            {#each data.homeCards as card (card.id)}
              <button class={['app-card plugin-card', { 'is-launching': data.pluginCardPending === card.id }]} data-plugin-card={card.id} data-focus-key={card.id} aria-busy={data.pluginCardPending === card.id} aria-disabled={data.pluginCardPending === card.id} onclick={() => actions.openPluginCard(card)}>
                <span class="plus-logo" aria-hidden="true">◆</span>
                <span>{data.pluginCardPending === card.id ? `Opening ${card.title}…` : card.title}</span>
              </button>
            {/each}
          </div>
          {#if cardFailure}
            <div class="plugin-card-status" role="alert">
              <span>{cardFailure.card.title} couldn't open. {cardFailure.message}</span>
              <button class="text-link" data-focus-key={`${cardFailure.card.id}:retry`} onclick={actions.retryPluginCard}>Retry</button>
            </div>
          {/if}
        </section>
      </div>
    {/if}
  </div></div>
</div>

<style>
  .home-layout, .home-instance { display: contents; }
  .resizing-content { display: flow-root; }
  .plugin-card .plus-logo { color: var(--accent); }
  .plugin-card-status { display: flex; align-items: center; justify-content: space-between; gap: 16px; margin-top: 10px; padding: 12px 14px; border: 1px solid color-mix(in srgb, var(--danger) 32%, transparent); border-radius: 10px; background: color-mix(in srgb, var(--danger) 8%, transparent); color: var(--text); font-size: clamp(12px, .8vw, 15px); line-height: 1.4; }
  .plugin-card-status span { min-width: 0; }
  .plugin-card-status .text-link { flex: none; min-height: 40px; padding: 0 12px; border: 1px solid #ffffff1f; border-radius: 8px; background: #ffffff0a; }
  .home-instance:has(> :global(.activity-row), > :global(.steam-row)) + .home-instance > :global(.apps-row) { margin-top: calc(6px - var(--row-gap)); }
  @media (min-width: 1000px) {
    .columns { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: var(--row-gap) var(--gap); padding-inline: var(--side); }
    .columns > .home-instance { display: block; min-width: 0; }
    .columns > .home-instance[data-span="2"] { grid-column: span 2; }
    .columns .home-instance :global(.content-row) { padding-inline: 0; }
    .columns > .home-instance > :global(.content-row) { margin-bottom: 0; }
    .columns > .home-instance > :global(.apps-row) { margin-top: 0; }
    .columns > .home-instance:has(> :global(.activity-row), > :global(.steam-row)) + .home-instance > :global(.apps-row) { margin-top: 0; }
    .columns .home-instance :global(.hero.is-inset) { margin-inline: 0; }
  }
</style>
