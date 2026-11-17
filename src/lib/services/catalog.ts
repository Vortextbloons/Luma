import type { MediaItem } from '../types';

const endpoint = 'https://v3-cinemeta.strem.io/catalog';

interface CinemetaMeta {
  id: string; name: string; description?: string; genres?: string[]; genre?: string[];
  year?: string; runtime?: string; imdbRating?: string; poster?: string; background?: string;
}

async function load(id: string, kind = 'movie'): Promise<MediaItem[]> {
  const response = await fetch(`${endpoint}/${kind}/${id}.json`, { signal: AbortSignal.timeout(15_000) });
  if (!response.ok) throw new Error(`Movie catalog request failed (${response.status}).`);
  const data = await response.json() as { metas?: CinemetaMeta[] };
  return (data.metas ?? []).slice(0, 10).map(meta => ({
    id: meta.id,
    title: meta.name,
    subtitle: [...(meta.genres ?? meta.genre ?? []).slice(0, 2), meta.year].filter(Boolean).join(' · '),
    poster: (meta.poster ?? meta.background ?? '').replace('/poster/small/', '/poster/medium/'),
    backdrop: meta.background ?? meta.poster ?? '',    description: meta.description ?? '',
    year: meta.year ?? '',
    runtime: meta.runtime ?? '',
    rating: meta.imdbRating ?? '',
  }));
}

export async function loadCatalog() {
  const results = await Promise.allSettled([load('top'), load('imdbRating'), load('top', 'series')]);
  const names = ['Popular movies', 'Top-rated movies', 'Popular shows'];
  return {
    popular: results[0].status === 'fulfilled' ? results[0].value : undefined,
    featured: results[1].status === 'fulfilled' ? results[1].value : undefined,
    shows: results[2].status === 'fulfilled' ? results[2].value : undefined,
    unavailable: results.flatMap((result, index) => result.status === 'rejected' ? [names[index]] : []),
  };
}

export const searchMovies = (query: string) => load(`top/search=${encodeURIComponent(query.trim())}`);

export const searchShows = (query: string) => load(`top/search=${encodeURIComponent(query.trim())}`, 'series');
