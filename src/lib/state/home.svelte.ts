import type { ActivityItem, AppId, HomeRowId, MediaItem, SteamGame, StremioItem, YouTubeFeedItem } from '../types';
import type { MusicCardItem } from '../types/spotify';
import type { HeroSlide } from '../components/MovieHero.svelte';
import type { CustomizationConfig } from '../services/customization';
import { loadCatalog, loadPopularShows } from '../services/catalog';
import { actionLabel, appProviders, isContinueWatching, progressLabel, providerName, sortRecent, typeLabel } from '../services/activity';
import { majorSportsLeagues, sportsScoresHidden } from '../services/sports';
import type { GameScore } from '../services/sportsScores';
import type { ActivityState } from './activity.svelte';
import type { createSpotify } from './spotify.svelte';
import type { SportsState } from './sports.svelte';
import type { StremioState } from './stremio.svelte';
import type { SteamState } from './steam.svelte';
import type { YouTubeState } from './youtube.svelte';
import nbaCover from '../../../src-tauri/resources/icons/sports-nba.png';
import nflCover from '../../../src-tauri/resources/icons/sports-nfl.png';
import mlbCover from '../../../src-tauri/resources/icons/sports-mlb.png';
import nhlCover from '../../../src-tauri/resources/icons/sports-nhl.png';

const leagueCovers: Record<string, string> = { NBA: nbaCover, NFL: nflCover, MLB: mlbCover, NHL: nhlCover };

export interface HomeActions {
  launch: (id: AppId) => void;
  openItem: (item: ActivityItem) => void;
  openStremio: (id: string) => void;
  openStremioDetail: (kind: string, id: string) => void;
  openSteamGame: (appid: number) => void;
  openSteamStore: (appid: number) => void;
  openYouTubeItem: (id: string) => void;
  select: (item: MediaItem) => void;
  openPlayer: () => void;
  watchLeague: (game: GameScore) => void;
}

export function createHome(options: {
  layout: () => CustomizationConfig;
  activity: ActivityState;
  spotify: ReturnType<typeof createSpotify>;
  sports: SportsState;
  stremio: StremioState;
  steam: SteamState;
  youtube: YouTubeState;
  actions: HomeActions;
}) {
  const { layout, activity, spotify, sports, stremio, steam, youtube, actions } = options;
  let popular = $state.raw<MediaItem[]>([]);
  let featured = $state.raw<MediaItem[]>([]);
  let popularShows = $state.raw<MediaItem[]>([]);
  let unavailable = $state.raw<string[]>([]);
  let showsUnavailable = $state.raw<string[]>([]);
  const unavailableNames = $derived([...unavailable, ...showsUnavailable]);
  const error = $derived(unavailableNames.length ? `Could not refresh: ${unavailableNames.join(', ')}. Check your internet connection and retry.` : '');
  let loading = $state(false);
  let showsLoaded = false;
  let showsLoading = $state(false);
  let previewApp = $state<AppId | null>(null);
  let heroPick = $state<HeroSlide | null>(null);
  let hoveredGame = $state.raw<GameScore | null>(null);

  const rowVisible = (id: HomeRowId) => layout().home.some(row => row.component === id && row.visible);
  const previewLeague = $derived<AppId | null>(previewApp && majorSportsLeagues.includes(previewApp) ? previewApp : null);
  const previewProvider = $derived(previewApp ? appProviders[previewApp] : null);
  const stremioRecent = $derived(stremio.recent);
  const previewActivity = $derived(previewProvider ? activity.items.filter(item => item.provider === previewProvider && item.type !== 'application').slice(0, 8) : []);
  const previewMusic = $derived(spotify.home.recentlyPlayed.map((item, index) => ({ id: `${item.id ?? item.title}-${index}`, title: item.title, uri: item.uri, artworkUrl: item.artworkUrl, subtitle: item.artist })));
  const continueItems = $derived(rowVisible('continue') ? sortRecent(activity.items.filter(item => item.type !== 'application' && isContinueWatching(item))) : []);
  const appHeroItems = $derived.by(() => {
    const app = previewApp;
    if (!app) return [];
    const own = sortRecent(activity.items.filter(item => item.provider === appProviders[app] && item.type !== 'application'));
    const resumable = own.filter(isContinueWatching);
    return resumable.length ? resumable : own;
  });
  const spotifyHero = $derived(previewApp === 'Spotify' && !appHeroItems.length ? previewMusic.slice(0, 5) : []);
  const stremioHero = $derived(previewApp === 'Stremio' ? (stremioRecent.length ? stremioRecent : stremio.status?.items ?? []).slice(0, 5) : []);
  const steamHero = $derived(previewApp === 'Steam' ? steam.games.slice(0, 5) : []);
  const leagueGames = $derived(previewLeague ? sports.games(previewLeague) : []);
  const leagueGamesLoading = $derived(sports.previewLoading);
  const catalogVisible = $derived(rowVisible('popular') || rowVisible('featured'));
  const heroSource = $derived(heroPick ? `pick:${heroPick.key}` : hoveredGame && previewLeague ? `pick:league:${previewLeague}:${hoveredGame.id}` : previewApp && (previewLeague || appHeroItems.length || spotifyHero.length || stremioHero.length || steamHero.length) ? `app:${previewApp}` : continueItems.length ? 'continue' : 'catalog');
  const heroSlides = $derived.by<HeroSlide[]>(() => {
    if (heroPick) return [heroPick];
    if (previewLeague) {
      if (hoveredGame && hoveredGame.league === previewLeague) return [leagueSlide(previewLeague, hoveredGame)];
      return leagueGames.length ? leagueGames.map(game => leagueSlide(previewLeague, game)) : [leagueSlide(previewLeague)];
    }
    if (previewApp) {
      const eyebrow = `FROM ${providerName(appProviders[previewApp]).toUpperCase()}`;
      if (appHeroItems.length) return appHeroItems.slice(0, 5).map(item => activitySlide(item, eyebrow));
      if (steamHero.length) return steamHero.map(steamSlide);
      if (spotifyHero.length) return spotifyHero.map(musicSlide);
      if (stremioHero.length) return stremioHero.map(stremioSlide);
    }
    if (continueItems.length) return continueItems.slice(0, 5).map(item => activitySlide(item, 'CONTINUE WATCHING'));
    if (rowVisible('featured')) return featured.slice(0, 5).map(item => movieSlide(item));
    if (rowVisible('popular')) return popular.slice(0, 5).map(item => movieSlide(item));
    return [];
  });

  async function start() {
    if (loading) return;
    loading = true;
    try {
      const data = await loadCatalog(update => {
        if (update.popular) popular = update.popular;
        if (update.featured) featured = update.featured;
      });
      unavailable = data.unavailable;
    } finally {
      loading = false;
    }
  }

  async function loadShows(force = false) {
    if (showsLoading || (showsLoaded && !force)) return;
    showsLoading = true;
    try {
      popularShows = await loadPopularShows();
      showsLoaded = true;
      showsUnavailable = [];
    } catch {
      showsUnavailable = ['Popular shows'];
    } finally {
      showsLoading = false;
    }
  }

  function preview(id: AppId) {
    const changed = previewApp !== id;
    previewApp = id;
    if (changed) {
      heroPick = null;
      hoveredGame = null;
      sports.endPreview();
    }
    if (id === 'Spotify') void spotify.loadHome();
    if (id === 'Steam' && changed) void steam.refresh();
    if (id === 'Stremio' && changed) void stremio.refreshIfStale();
    if (changed && majorSportsLeagues.includes(id)) void sports.preview(id);
  }

  function clearPreview() {
    previewApp = null;
    heroPick = null;
    hoveredGame = null;
    sports.endPreview();
  }

  function formatRuntime(value: string) {
    const minutes = Number.parseInt(value, 10);
    if (!Number.isFinite(minutes) || minutes <= 0) return value;
    return minutes >= 60 ? `${Math.floor(minutes / 60)}h ${minutes % 60}m` : `${minutes}m`;
  }

  function activitySlide(item: ActivityItem, eyebrow: string): HeroSlide {
    const progress = progressLabel(item);
    const label = actionLabel(item);
    return {
      key: `activity:${item.id}`,
      eyebrow,
      title: item.title,
      chips: [
        { label: providerName(item.provider) },
        { label: typeLabel(item) },
        ...(progress ? [{ label: progress }] : []),
      ],
      description: item.subtitle ?? '',
      art: item.artwork ?? '',
      primaryLabel: label,
      primaryIcon: label === 'Resume' ? 'play' : 'open',
      primary: () => { void actions.openItem(item); },
    };
  }

  function activityEyebrow(item: ActivityItem) {
    const provider = providerName(item.provider).toUpperCase();
    if (previewApp && appProviders[previewApp] === item.provider) return `FROM ${provider}`;
    return isContinueWatching(item) ? 'CONTINUE WATCHING' : `FROM ${provider}`;
  }

  function movieSlide(item: MediaItem, eyebrow = 'FEATURED MOVIE'): HeroSlide {
    const genres = item.subtitle.split(' · ').filter(part => part !== item.year).slice(0, 2);
    return {
      key: `movie:${item.id}`,
      eyebrow,
      title: item.title,
      chips: [
        ...genres.map(label => ({ label })),
        ...(item.rating ? [{ label: `★ ${item.rating}`, tone: 'rating' as const }] : []),
        ...(item.runtime ? [{ label: formatRuntime(item.runtime) }] : []),
        ...(item.year ? [{ label: item.year }] : []),
      ],
      description: item.description,
      art: item.backdrop || item.poster,
      primaryLabel: 'Play',
      primary: () => { void actions.openStremioDetail('movie', item.id); },
      details: () => actions.select(item),
    };
  }

  function musicSlide(track: { id: string; title: string; uri?: string; artworkUrl?: string; subtitle?: string }): HeroSlide {
    return {
      key: `music:${track.id}`,
      eyebrow: 'FROM SPOTIFY',
      title: track.title,
      chips: [...(track.subtitle ? [{ label: track.subtitle }] : []), { label: 'Music' }],
      description: '',
      art: track.artworkUrl ?? '',
      artShape: 'square',
      primaryLabel: 'Play',
      primary: () => { if (track.uri) void spotify.playUri(track.uri).then(played => { if (played) actions.openPlayer(); }); },
    };
  }

  function stremioSlide(item: StremioItem): HeroSlide {
    const label = item.kind === 'series' ? 'Series' : item.kind === 'movie' ? 'Movie' : 'Video';
    const remaining = item.duration > item.timeOffset ? `${Math.max(1, Math.round((item.duration - item.timeOffset) / 60_000))}m left` : '';
    return {
      key: `stremio:${item.id}`,
      eyebrow: 'FROM STREMIO',
      title: item.name,
      chips: [{ label: 'Stremio' }, { label }, ...(remaining ? [{ label: remaining }] : [])],
      description: '',
      art: item.poster,
      artShape: 'portrait',
      primaryLabel: 'Open Stremio',
      primaryIcon: 'open',
      primary: () => { void actions.openStremio(item.id); },
    };
  }

  function steamPlaytime(minutes: number) {
    return minutes >= 60 ? `${Math.floor(minutes / 60)}h ${minutes % 60}m` : `${minutes}m`;
  }

  function steamSlide(game: SteamGame): HeroSlide {
    return {
      key: `steam:${game.appid}`,
      eyebrow: 'FROM STEAM',
      title: game.name,
      chips: [{ label: 'Steam' }, { label: `${steamPlaytime(game.playtimeMinutes)} played` }],
      description: '',
      art: game.hero,
      primaryLabel: 'Continue playing',
      primary: () => { void actions.openSteamGame(game.appid); },
    };
  }

  function steamRecommendationSlide(game: SteamGame): HeroSlide {
    return {
      key: `steam-store:${game.appid}`,
      eyebrow: 'RECOMMENDED FOR YOU',
      title: game.name,
      chips: [{ label: 'Steam' }, ...(steam.recommendationSource ? [{ label: `Like ${steam.recommendationSource.name}` }] : [])],
      description: '',
      art: game.hero,
      primaryLabel: 'View in Steam',
      primaryIcon: 'open',
      primary: () => { void actions.openSteamStore(game.appid); },
    };
  }

  function videoSlide(item: YouTubeFeedItem): HeroSlide {
    return {
      key: `youtube:${item.id}`,
      eyebrow: 'FROM YOUTUBE',
      title: item.title,
      chips: [{ label: 'YouTube' }, { label: 'Video' }],
      description: '',
      art: `https://i.ytimg.com/vi/${item.id}/hqdefault.jpg`,
      primaryLabel: 'Open video',
      primaryIcon: 'open',
      primary: () => { void actions.openYouTubeItem(item.id); },
    };
  }

  function leagueSlide(league: string, game?: GameScore): HeroSlide {
    const art = game ? sports.artworkFor(game.id) : '';
    const loading = !game && leagueGamesLoading;
    return {
      key: game ? `league:${league}:${game.id}` : `league:${league}:${loading ? 'loading' : 'empty'}`,
      eyebrow: `FROM ${league}`,
      title: game ? game.teams.map(team => team.name).join(' vs ') : loading ? league : `No ${league} games right now`,
      chips: [{ label: league }, ...(game ? [{ label: game.status }] : loading ? [{ label: 'Loading' }] : [{ label: 'No games' }])],
      description: game?.venue || (loading ? 'Looking up live and upcoming games.' : 'There are no live or upcoming games listed for this league.'),
      art: art || game?.leagueLogo || leagueCovers[league],
      artShape: art ? undefined : 'square',
      primaryLabel: game ? game.state === 'post' && !sportsScoresHidden() ? 'Final score' : 'Game details' : `Open ${league}`,
      primaryIcon: 'open',
      primary: () => { if (game) actions.watchLeague(game); else actions.launch(league as AppId); },
      ...(game ? { detailsLabel: `Open ${league}`, details: () => { actions.launch(league as AppId); } } : {}),
    };
  }

  return {
    get popular() { return popular; },
    get featured() { return featured; },
    get popularShows() { return popularShows; },
    get error() { return error; },
    get loading() { return loading || showsLoading; },
    get showsLoading() { return showsLoading; },
    get previewApp() { return previewApp; },
    get heroSlides() { return heroSlides; },
    get heroSource() { return heroSource; },
    get previewLeague() { return previewLeague; },
    get leagueGames() { return leagueGames; },
    get leagueGamesLoading() { return leagueGamesLoading; },
    get stremioRecent() { return stremioRecent; },
    get previewActivity() { return previewActivity; },
    get previewMusic() { return previewMusic; },
    get catalogVisible() { return catalogVisible; },
    start,
    loadShows,
    preview,
    clearPreview,
    previewActivityItem(item: ActivityItem | null) { heroPick = item ? activitySlide(item, activityEyebrow(item)) : null; },
    previewLeagueGame(game: GameScore | null) {
      hoveredGame = game && previewLeague && game.league === previewLeague ? game : null;
      if (hoveredGame) void sports.loadArtwork(hoveredGame.league, hoveredGame.id);
    },
    previewMovie(item: MediaItem | null, eyebrow?: string) { heroPick = item ? movieSlide(item, eyebrow) : null; },
    previewStremio(item: StremioItem | null) { heroPick = item ? stremioSlide(item) : null; },
    previewSteamGame(item: SteamGame | null) { heroPick = item ? steamSlide(item) : null; },
    previewSteamRecommendation(item: SteamGame | null) { heroPick = item ? steamRecommendationSlide(item) : null; },
    previewVideo(item: YouTubeFeedItem | null) { heroPick = item ? videoSlide(item) : null; },
    previewMusicItem(item: MusicCardItem | null) { heroPick = item ? musicSlide(item) : null; },
  };
}

export type HomeState = ReturnType<typeof createHome>;
