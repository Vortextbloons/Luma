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

const favoritesKey = 'luma-sports-favorites-v1';
let listingCache: SportsListing | null = null;
let listingRequest: Promise<SportsListing> | null = null;

const text = (value: string | null | undefined) => (value ?? '').replace(/\s+/g, ' ').trim();

function eventUrl(value: string | null) {
  if (!value) return '';
  try {
    const url = new URL(value, 'https://isportsurge.ws/');
    if (url.protocol !== 'https:' || url.host !== 'isportsurge.ws') return '';
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
  listingRequest = invoke<string>('sports_listing')
    .then(html => {
      const listing = { events: parseSportsListing(html), updatedAt: Date.now() };
      listingCache = listing;
      return listing;
    })
    .finally(() => { listingRequest = null; });
  return listingRequest;
}

export const openSportsStream = (url: string, sourceIndex: number, title: string, score: string) =>
  invoke<void>('sports_open', { url, sourceIndex, title, score });

export const updateSportsScore = (score: string) => invoke<void>('sports_update_score', { score });

export async function loadSportsScores(league: string): Promise<GameScore[]> {
  return parseScoreboard(await invoke<unknown>('sports_scoreboard', { league }), league);
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
