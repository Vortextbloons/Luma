import type { SportsBadge } from '../types';

export interface GameTeam {
  name: string;
  abbreviation: string;
  logo: string;
  score: string;
  record: string;
  color: string;
}

export interface GameScore {
  id: string;
  league: string;
  leagueLogo: string;
  date: string;
  state: string;
  status: string;
  venue: string;
  teams: GameTeam[];
}

export function parseScoreboard(value: unknown, league: string): GameScore[] {
  const data = value as { events?: any[]; leagues?: { logos?: { href?: string }[] }[] } | null;
  if (!Array.isArray(data?.events)) throw new Error('Scores are unavailable.');
  return data.events.flatMap(event => {
    const game = event.competitions?.[0];
    if (!game || !Array.isArray(game.competitors) || game.competitors.length !== 2) return [];
    const status = game.status ?? event.status;
    return [{
      id: String(event.id), league, leagueLogo: data.leagues?.[0]?.logos?.[0]?.href ?? '', date: event.date ?? '',
      state: status?.type?.state ?? 'pre',
      status: status?.type?.shortDetail ?? status?.type?.detail ?? '',
      venue: game.venue?.fullName ?? '',
      teams: game.competitors.map((competitor: any) => ({
        name: competitor.team?.displayName ?? '',
        abbreviation: competitor.team?.abbreviation ?? '',
        logo: competitor.team?.logo ?? '',
        score: competitor.score == null ? '' : String(competitor.score),
        record: competitor.records?.find((record: any) => record.type === 'total')?.summary ?? '',
        color: /^[0-9a-f]{6}$/i.test(competitor.team?.color ?? '') ? `#${competitor.team.color}` : '#39374f',
      })),
    }];
  });
}

const normalize = (name: string) => name.toLowerCase().replace(/\blos angeles\b/g, 'la')
  .replace(/\bsaint\b/g, 'st').replace(/[^a-z0-9]/g, '');

export function matchScore(event: { league: string; teams: { name: string }[] }, scores: GameScore[]): GameScore | undefined {
  if (event.teams.length !== 2) return;
  const names = event.teams.map(team => normalize(team.name));
  const matches = scores.filter(game => game.league === event.league
    && game.teams.length === 2 && game.teams.every(team => names.includes(normalize(team.name))));
  return matches.length === 1 ? matches[0] : undefined;
}

export function matchEvent<T extends { league: string; teams: { name: string }[] }>(game: GameScore, events: T[]): T | undefined {
  if (game.teams.length !== 2) return;
  const names = game.teams.map(team => normalize(team.name));
  const matches = events.filter(event => event.league === game.league
    && event.teams.length === 2 && event.teams.every(team => names.includes(normalize(team.name))));
  return matches.length === 1 ? matches[0] : undefined;
}

export function scoreTeam(game: GameScore | undefined, name: string): GameTeam | undefined {
  return game?.teams.find(team => normalize(team.name) === normalize(name));
}

export function scoreSummary(game: GameScore | undefined, hideScores: boolean): string {
  if (!game) return '';
  if (hideScores || game.state === 'pre') return game.state === 'pre' ? game.status : 'Scores hidden';
  return `${game.teams.map(team => `${team.abbreviation || team.name} ${team.score}`).join('  ·  ')}  —  ${game.status}`;
}

function teamMatches(a: string, b: string) {
  const left = normalize(a);
  const right = normalize(b);
  return Boolean(left && right && (left === right || left.includes(right) || right.includes(left)));
}

export function matchSportsBadge(event: { league: string; teams: { name: string }[] }, badges: SportsBadge[]): SportsBadge | undefined {
  if (event.teams.length !== 2) return;
  const league = event.league.toLowerCase();
  return badges.find(badge => badge.showOnCards
    && badge.league.toLowerCase() === league
    && badge.teams.length >= 2
    && event.teams.every(team => badge.teams.some(candidate => teamMatches(team.name, candidate))));
}
