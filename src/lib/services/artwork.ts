interface Poster { url: string | null; expires: number }
const cache = new Map<string, Poster>();
const pending = new Map<string, Promise<string | null>>();
const waiting: (() => void)[] = [];
let active = 0;

export function showPoster(id: string): Promise<string | null> {
  if (!/^tt[0-9]+$/.test(id)) return Promise.resolve(null);
  const saved = cache.get(id);
  if (saved && saved.expires > Date.now()) return Promise.resolve(saved.url);
  const existing = pending.get(id);
  if (existing) return existing;
  const request = (async () => {
    if (active >= 4) await new Promise<void>(resolve => waiting.push(resolve));
    else active++;
    try {
      const response = await fetch(`https://api.tvmaze.com/lookup/shows?imdb=${id}`, { signal: AbortSignal.timeout(4000) });
      if (!response.ok && response.status !== 404) return null;
      const data = response.ok ? await response.json() as { image?: { original?: string; medium?: string } } : null;
      const candidate = data?.image?.medium ?? data?.image?.original;
      const url = candidate?.startsWith('https://static.tvmaze.com/') ? candidate : null;
      cache.delete(id);
      cache.set(id, { url, expires: Date.now() + (url ? 86400000 : 3600000) });
      while (cache.size > 100) cache.delete(cache.keys().next().value!);
      return url;
    } catch { return null; }
    finally {
      const next = waiting.shift();
      if (next) next();
      else active--;
    }
  })().finally(() => pending.delete(id));
  pending.set(id, request);
  return request;
}
