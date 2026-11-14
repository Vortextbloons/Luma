import type { MediaItem } from '../types';

const endpoint = 'https://v3-cinemeta.strem.io/catalog/movie';

interface CinemetaMeta {
  id: string; name: string; description?: string; genres?: string[]; genre?: string[];
  year?: string; runtime?: string; imdbRating?: string; poster?: string; background?: string;
}

async function load(id: string): Promise<MediaItem[]> {
  const response = await fetch(`${endpoint}/${id}.json`);
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
  const [popular, featured] = await Promise.all([load('top'), load('imdbRating')]);
  return { popular, featured, hero: popular[0] ?? featured[0] };
}

export const searchMovies = (query: string) => load(`top/search=${encodeURIComponent(query.trim())}`);
