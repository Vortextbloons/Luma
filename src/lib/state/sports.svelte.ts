import { indexMatchups, matchSportsBadge, scoreSummary, type GameScore } from '../services/sportsScores';
import {
  cachedSportsArtwork, cachedSportsListing, cachedSportsScores, loadSportsArtwork, loadSportsListing,
  loadSportsScores, majorSportsLeagues, onSportsClosed, openSportsStream, sportsScoresHidden,
  updateSportsScore, type SportsEvent, type SportsListing,
} from '../services/sports';
import { native, onSportsBadges, pluginSportsBadges } from '../services/tauri';
import type { SportsBadge } from '../types';

const order = (game: GameScore) => ['in', 'pre', 'post'].indexOf(game.state);

function mergeScores(scores: GameScore[], games: GameScore[], leagues: string[]) {
  return [...scores.filter(score => !leagues.includes(score.league)), ...games];
}

class SportsStore {
  listing = $state.raw<SportsListing | null>(cachedSportsListing());
  scores = $state.raw<GameScore[]>(cachedSportsScores().scores);
  artwork = $state<Record<string, string>>(Object.fromEntries(cachedSportsArtwork()));
  loading = $state(false);
  error = $state('');
  scoreError = $state('');
  updatedAt = $state(cachedSportsScores().updatedAt);
  previewLoading = $state(false);
  previewError = $state('');
  opening = $state(false);
  watching = $state(false);
  closedAt = $state(0);
  badges = $state.raw<SportsBadge[]>([]);
  private scoreMatch = $derived(indexMatchups(this.scores));
  private streamMatch = $derived(indexMatchups(this.events));
  private scoreBusy = false;
  private watchingGame: GameScore | null = null;
  private previewRequest = 0;

  get events() {
    return this.listing?.events ?? [];
  }

  games(league: string) {
    return this.scores.filter(game => game.league === league && majorSportsLeagues.includes(league) && game.state !== 'post').sort((a, b) => order(a) - order(b)).slice(0, 5);
  }

  artworkFor(id: string) {
    return this.artwork[id] ?? '';
  }

  clearArtwork(id: string) {
    this.artwork[id] = '';
  }

  badgeFor(event: { league: string; teams: { name: string }[] }) {
    return matchSportsBadge(event, this.badges);
  }

  async refreshListing() {
    this.listing = await loadSportsListing();
  }

  scoreFor(event: SportsEvent) {
    return this.scoreMatch(event);
  }

  streamFor(game: GameScore) {
    return this.streamMatch(game);
  }

  eventFor(game: GameScore) {
    return this.streamFor(game) ?? {
      id: game.id, url: '', league: game.league,
      title: game.teams.map(team => team.name).join(' vs '),
      teams: game.teams.map(team => ({ name: team.name, logo: team.logo })),
      time: game.status, live: game.state === 'in',
    };
  }

  async start() {
    if (!native) return () => {};
    let active = true;
    void pluginSportsBadges().then(list => { if (active) this.badges = list; }).catch(() => {});
    const [stopClosed, stopBadges] = await Promise.all([
      onSportsClosed(() => {
        this.closedAt++;
        this.watching = false;
        this.opening = false;
        this.watchingGame = null;
      }),
      onSportsBadges(list => {
        if (active) this.badges = list;
      }),
    ]);
    return () => {
      active = false;
      stopClosed();
      stopBadges();
    };
  }

  async load(force = false) {
    if (!native || this.loading) return;
    this.loading = true;
    this.error = '';
    try {
      this.listing = await loadSportsListing(force);
      await this.refreshScores(undefined, force);
    } catch {
      this.error = this.listing ? 'Games could not refresh. Showing the last loaded list.' : 'Games could not load. Check your connection and try again.';
    } finally {
      this.loading = false;
    }
  }

  async refreshScores(leagues?: string[], force = false) {
    if (!native || this.scoreBusy) return false;
    const targets = [...new Set((leagues ?? this.events.map(event => event.league)).filter(league => majorSportsLeagues.includes(league)))];
    if (!targets.length) return false;
    this.scoreBusy = true;
    const results = await Promise.allSettled(targets.map(league => loadSportsScores(league, force)));
    if (targets.length) {
      const succeeded = targets.filter((_, index) => results[index].status === 'fulfilled');
      const games = results.flatMap(result => result.status === 'fulfilled' ? result.value : []);
      if (succeeded.length) {
        this.scores = mergeScores(this.scores, games, succeeded);
        this.updatedAt = cachedSportsScores().updatedAt;
      }
      if (this.watchingGame) this.publishScore();
      this.scoreError = results.some(result => result.status === 'rejected') ? 'Some scores could not refresh. Game streams are still available.' : '';
    }
    this.scoreBusy = false;
    return results.some(result => result.status === 'fulfilled');
  }

  async preview(league: string) {
    if (!native) return;
    const request = ++this.previewRequest;
    this.previewLoading = true;
    this.previewError = '';
    try {
      if (!this.listing) {
        const listing = await loadSportsListing().catch(() => null);
        if (listing && request === this.previewRequest) this.listing = listing;
      }
      const games = await loadSportsScores(league);
      if (request !== this.previewRequest) return;
      this.scores = mergeScores(this.scores, games, [league]);
      const first = [...games].filter(game => game.state !== 'post').sort((a, b) => order(a) - order(b))[0];
      if (first && !this.artwork[first.id]) {
        const art = await loadSportsArtwork(league, first.id).catch(() => '');
        if (request === this.previewRequest) this.artwork[first.id] = art;
      }
    } catch {
      if (request === this.previewRequest) this.previewError = 'Games could not load. Check your connection and retry.';
    } finally {
      if (request === this.previewRequest) this.previewLoading = false;
    }
  }

  endPreview() {
    this.previewRequest++;
    this.previewLoading = false;
    this.previewError = '';
  }

  async loadArtwork(league: string, eventId: string) {
    if (!native || this.artwork[eventId] !== undefined) return;
    const art = await loadSportsArtwork(league, eventId).catch(() => '');
    this.artwork[eventId] = art;
  }

  async openEvent(event: SportsEvent, game: GameScore | undefined, sourceIndex = 0) {
    if (!native || this.opening) return false;
    this.opening = true;
    try {
      await this.open(event, game, sourceIndex);
      return true;
    } catch {
      return false;
    } finally {
      this.opening = false;
    }
  }

  private async open(event: SportsEvent, game: GameScore | undefined, sourceIndex = 0) {
    await openSportsStream(event.url, sourceIndex, event.title, game ? scoreSummary(game, sportsScoresHidden()) : '', event.sources);
    this.watching = true;
    this.watchingGame = game ?? null;
    if (game) this.publishScore(game);
  }

  publishScore(game?: GameScore) {
    const latest = game ?? (this.watchingGame ? this.scores.find(candidate => candidate.id === this.watchingGame!.id) : undefined);
    if (!latest) return;
    this.watchingGame = latest;
    void updateSportsScore(scoreSummary(latest, sportsScoresHidden())).catch(() => {});
  }

  refreshOverlay() {
    if (this.watchingGame) this.publishScore(this.watchingGame);
  }
}

export function createSports() {
  return new SportsStore();
}

export type SportsState = SportsStore;
