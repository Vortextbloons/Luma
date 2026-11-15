import { streamEastUrl, parseStreamEast, mergeSportsSources } from './sportsSources';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { parseScoreboard, type GameScore } from './sportsScores';

export interface SportsTeam {
  name: string;
  logo: string;
}

export interface SportsEvent {
  id: string;
  url: string;
  sources?: string[];
  provider?: string;
  league: string;
  title: string;
  teams: SportsTeam[];
  time: string;
  live: boolean;
}

export interface SportsListing {
  events: SportsEvent[];
  updatedAt: number;
}

export interface SportsFavorite {
  league: string;
  team: string;
}

export const majorSportsLeagues = ['NBA', 'NFL', 'MLB', 'NHL'];

export function sportsScoresHidden() {
  try { return localStorage.getItem('luma-sports-hide-scores') === 'true'; } catch { return false; }
}

const favoritesKey = 'luma-sports-favorites-v1';
interface CachedValue<T> { value: T; updatedAt: number }
const cachePrefix = 'luma-sports-cache-v1-';
function readCache<T>(key: string): CachedValue<T> | null {
  try {
    const cached = JSON.parse(localStorage.getItem(cachePrefix + key) ?? 'null');
    return cached && Number.isFinite(cached.updatedAt) && Date.now() - cached.updatedAt < 86400000 && cached.value != null ? cached : null;
  } catch { return null; }
}
function saveCache<T>(key: string, value: T, updatedAt = Date.now()) {
  try { localStorage.setItem(cachePrefix + key, JSON.stringify({ value, updatedAt })); } catch {}
}
const savedListing = readCache<SportsListing>('listing-sources-v2')?.value;
let listingCache: SportsListing | null = savedListing && Array.isArray(savedListing.events) && savedListing.events.every(event => eventUrl(event.url) && Array.isArray(event.teams) && typeof event.league === 'string') ? savedListing : null;
const scoreCache = new Map<string, CachedValue<GameScore[]>>();
const scoreRequests = new Map<string, Promise<GameScore[]>>();
for (const league of majorSportsLeagues) {
  const cached = readCache<GameScore[]>('scores-' + league);
  if (cached && new Date(cached.updatedAt).toDateString() === new Date().toDateString() && Array.isArray(cached.value) && cached.value.every(game => game.league === league && Array.isArray(game.teams))) scoreCache.set(league, cached);
}
const artworkCache: Record<string, CachedValue<string>> = readCache<Record<string, CachedValue<string>>>('artwork')?.value ?? {};
const artworkRequests = new Map<string, Promise<string>>();
let listingRequest: Promise<SportsListing> | null = null;

const text = (value: string | null | undefined) => (value ?? '').replace(/\s+/g, ' ').trim();

function eventUrl(value: string | null) {
  if (!value) return '';
  try {
    const url = new URL(value, 'https://isportsurge.ws/');
    if (url.host !== 'isportsurge.ws') return streamEastUrl(value);
    if (url.protocol !== 'https:') return '';
    if (!url.pathname.startsWith('/watch/') && !url.pathname.startsWith('/event/')) return '';
    return url.href;
  } catch {
    return '';
  }
}

function imageUrl(value: string | null | undefined) {
  if (!value) return '';
  try {
    const url = new URL(value, 'https://isportsurge.ws/');
    return url.protocol === 'https:' ? url.href : '';
  } catch {
    return '';
  }
}

function parseTeams(row: HTMLElement) {
  const teams: SportsTeam[] = [];
  for (const team of row.querySelectorAll<HTMLElement>('.team-name-event-row')) {
    const image = team.querySelector('img');
    const name = text(team.querySelector('span')?.textContent) || text(image?.getAttribute('alt'));
    if (!name) continue;
    teams.push({ name, logo: imageUrl(image?.getAttribute('src')) });
  }
  if (!teams.length) {
    const name = text(row.querySelector('h4')?.textContent);
    if (name) teams.push({ name, logo: imageUrl(row.querySelector('img')?.getAttribute('src')) });
  }
  return teams;
}

export function parseSportsListing(html: string): SportsEvent[] {
  const document = new DOMParser().parseFromString(html, 'text/html');
  const events: SportsEvent[] = [];
  const seen = new Set<string>();
  for (const row of document.querySelectorAll<HTMLAnchorElement>('a.row.MaclariListele')) {
    const url = eventUrl(row.getAttribute('href'));
    if (!url || seen.has(url)) continue;
    const teams = parseTeams(row);
    if (!teams.length) continue;
    const time = text(row.querySelector('.time-badge')?.textContent);
    const league = text(row.querySelector('.col-md-2.ListelemeDuzen')?.textContent) || 'Other';
    const title = teams.map(team => team.name).join(' vs ');
    events.push({ id: url, url, league, title, teams, time, live: /^in progress$/i.test(time) });
    seen.add(url);
  }
  if (!document.querySelector('#searchInput, input[placeholder*="Search"], a.row.MaclariListele')) {
    throw new Error('Sports listings could not be read.');
  }
  return events;
}

export const cachedSportsListing = () => listingCache;

export function loadSportsListing(force = false) {
  if (!force && listingCache && Date.now() - listingCache.updatedAt < 60000) return Promise.resolve(listingCache);
  if (listingRequest) return listingRequest;
  listingRequest = Promise.allSettled([invoke<unknown>('sports_streameast_listing', { force }).then(parseStreamEast), invoke<string>('sports_listing').then(parseSportsListing)])
    .then(results => {
      const primary = results[0].status === 'fulfilled' ? results[0].value : [];
      const fallback = results[1].status === 'fulfilled' ? results[1].value : [];
      if (results.every(result => result.status === 'rejected')) throw new Error('Sports sources are unavailable.');
      const listing = { events: mergeSportsSources(primary, fallback), updatedAt: Date.now() };
      listingCache = listing;
      saveCache('listing-sources-v2', listing);
      return listing;
    })
    .finally(() => { listingRequest = null; });
  return listingRequest;
}

export const openSportsStream = (url: string, sourceIndex: number, title: string, score: string, sources?: string[]) => {
  const index = sources?.length ? sourceIndex % sources.length : 0;
  return invoke<void>('sports_open', { url: sources?.length ? sources[index] + '#luma-plan=' + index : url, sourceIndex: sources?.length ? 0 : sourceIndex, title, score, sources });
};

export const updateSportsScore = (score: string) => invoke<void>('sports_update_score', { score });

export function cachedSportsScores() {
  return { scores: [...scoreCache.values()].flatMap(cached => cached.value), updatedAt: Math.max(0, ...[...scoreCache.values()].map(cached => cached.updatedAt)) };
}

export function loadSportsScores(league: string, force = false): Promise<GameScore[]> {
  const cached = scoreCache.get(league);
  if (!force && cached && Date.now() - cached.updatedAt < 30000) return Promise.resolve(cached.value);
  const pending = scoreRequests.get(league);
  if (pending) return pending;
  const request = invoke<unknown>('sports_scoreboard', { league }).then(data => {
    const value = parseScoreboard(data, league);
    const updatedAt = Date.now();
    scoreCache.set(league, { value, updatedAt });
    saveCache('scores-' + league, value, updatedAt);
    return value;
  }).finally(() => scoreRequests.delete(league));
  scoreRequests.set(league, request);
  return request;
}

export function cachedSportsArtwork(): [string, string][] {
  return Object.entries(artworkCache).filter(([, cached]) => cached && typeof cached.value === 'string' && Date.now() - cached.updatedAt < 86400000).map(([id, cached]) => [id, cached.value]);
}

export function loadSportsArtwork(league: string, eventId: string): Promise<string> {
  const cached = artworkCache[eventId];
  if (cached && Date.now() - cached.updatedAt < 86400000) return Promise.resolve(cached.value);
  const pending = artworkRequests.get(eventId);
  if (pending) return pending;
  const request = invoke<string | null>('sports_artwork', { league, eventId }).then(value => {
    artworkCache[eventId] = { value: value ?? '', updatedAt: Date.now() };
    const oldest = Object.keys(artworkCache).sort((a, b) => artworkCache[a].updatedAt - artworkCache[b].updatedAt);
    for (const id of oldest.slice(0, Math.max(0, oldest.length - 24))) delete artworkCache[id];
    saveCache('artwork', artworkCache);
    return value ?? '';
  }).finally(() => artworkRequests.delete(eventId));
  artworkRequests.set(eventId, request);
  return request;
}

export const onSportsClosed = (callback: () => void) => listen('sports-closed', callback);

const favoriteKey = (league: string, team: string) => `${league.trim().toLowerCase()}\u0000${team.trim().toLowerCase()}`;

function validFavorite(value: unknown): value is SportsFavorite {
  if (!value || typeof value !== 'object') return false;
  const favorite = value as Record<string, unknown>;
  return typeof favorite.league === 'string' && favorite.league.trim() !== ''
    && typeof favorite.team === 'string' && favorite.team.trim() !== '';
}

export function readSportsFavorites(): SportsFavorite[] {
  try {
    const value: unknown = JSON.parse(localStorage.getItem(favoritesKey) ?? '[]');
    if (!Array.isArray(value)) return [];
    return value.filter(validFavorite).map(favorite => ({ league: favorite.league.trim(), team: favorite.team.trim() }));
  } catch {
    return [];
  }
}

export function writeSportsFavorites(favorites: SportsFavorite[]) {
  try {
    localStorage.setItem(favoritesKey, JSON.stringify(favorites));
  } catch {}
}

export function isSportsFavorite(favorites: SportsFavorite[], league: string, team: string) {
  const key = favoriteKey(league, team);
  return favorites.some(favorite => favoriteKey(favorite.league, favorite.team) === key);
}

export function toggleSportsFavorite(favorites: SportsFavorite[], league: string, team: string) {
  const key = favoriteKey(league, team);
  return isSportsFavorite(favorites, league, team)
    ? favorites.filter(favorite => favoriteKey(favorite.league, favorite.team) !== key)
    : [...favorites, { league: league.trim(), team: team.trim() }];
}
