import type { SportsEvent } from './sports';

export function streamEastUrl(value: unknown): string {
  if (typeof value !== 'string') return '';
  try {
    const url = new URL(value);
    if (url.protocol !== 'https:' || url.username || url.password || url.port) return '';
    if (url.hostname === 'golygoals.com' && ['/streameast.html', '/streame.html'].includes(url.pathname) && (url.searchParams.get('id') || url.searchParams.get('key'))) return url.href;
    if (url.hostname === 'streamec.st' && /^\/embed\/ch\d+\.php$/.test(url.pathname)) return url.href;
  } catch {}
  return '';
}

export function parseStreamEast(value: unknown): SportsEvent[] {
  const data = value as { matches?: any[] } | null;
  if (!Array.isArray(data?.matches)) throw new Error('StreamEast listings could not be read.');
  return data.matches.flatMap(match => {
    const sources = [...new Set<string>((Array.isArray(match.streams) ? match.streams : []).map((stream: any) => streamEastUrl(stream.url)).filter(Boolean))].slice(0, 12);
    if (!sources.length) return [];
    const leagueId = String(match.api_id ?? '').match(/league-[a-z]+-(.+?)(?:-\d+)?$/)?.[1] ?? '';
    const league = ({ nba: 'NBA', nfl: 'NFL', mlb: 'MLB', nhl: 'NHL', 'college-football': 'CFB', 'mens-college-basketball': 'NCAA Men', 'womens-college-basketball': 'NCAA Women' } as Record<string, string>)[leagueId] || (/^NFL RedZone$/i.test(String(match.title)) ? 'NFL' : String(match.sport_name || 'Other'));
    const teams = [match.home, match.away].filter(team => typeof team?.name === 'string' && team.name.trim()).map(team => ({ name: team.name.trim(), logo: typeof team.badge === 'string' && team.badge.startsWith('https://') ? team.badge : '' }));
    if (!teams.length && typeof match.title === 'string') teams.push({ name: match.title, logo: '' });
    if (!teams.length) return [];
    const date = new Date(match.starts_at);
    return [{ id: `streameast:${match.id}`, url: sources[0], sources, provider: 'StreamEast', league, title: teams.map(team => team.name).join(' vs '), teams, live: match.live === true, time: match.live ? 'In Progress' : Number.isNaN(date.getTime()) ? 'Upcoming' : date.toLocaleString([], { weekday: 'short', hour: 'numeric', minute: '2-digit' }) }];
  });
}

export function mergeSportsSources(primary: SportsEvent[], fallback: SportsEvent[]): SportsEvent[] {
  const key = (event: SportsEvent) => event.teams.length === 2 ? `${event.league}:${event.teams.map(team => team.name.toLowerCase().replace(/[^a-z0-9]/g, '')).sort().join(':')}` : `${event.league}:${event.title.toLowerCase().replace(/[^a-z0-9]/g, '')}`;
  const consumed = new Set<string>();
  const result = primary.map(event => {
    const matches = fallback.filter(candidate => key(candidate) === key(event));
    if (matches.length !== 1 || primary.filter(candidate => key(candidate) === key(event)).length !== 1) return event;
    consumed.add(matches[0].id);
    return { ...event, sources: [...new Set([...(event.sources ?? [event.url]), matches[0].url])] };
  });
  return [...result, ...fallback.filter(event => !consumed.has(event.id))];
}
