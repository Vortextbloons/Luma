<script lang="ts">
  import PosterImage from '../../../src/lib/components/PosterImage.svelte';
  import { tick } from 'svelte';
  import SportsEventCard from '../../../src/lib/components/SportsEventCard.svelte';
  import Modal from './Modal.svelte';
  import Icon from './Icon.svelte';
  import { api } from '../lib/api';
  import { searchMovies, searchShows } from '../../../src/lib/services/catalog';
  import { parseSportsListing, type SportsEvent } from '../../../src/lib/services/sports';
  import { parseStreamEast, mergeSportsSources } from '../../../src/lib/services/sportsSources';
  import { say } from '../lib/toast.svelte';
  let { open = false, onclose }: { open?: boolean; onclose: () => void } = $props();
  type Result = { key: string; title: string; subtitle: string; image: string; target: unknown; sports?: SportsEvent };
  const sources = ['All', 'Movies', 'Shows', 'YouTube', 'Sports', 'History'];
  let source = $state('All');
  const group = (item: Result) => item.key.startsWith('movie') ? 'Movies' : item.key.startsWith('series') ? 'Shows' : item.key.startsWith('video') ? 'YouTube' : item.key.startsWith('sports') ? 'Sports' : 'History';
  const groups = $derived(sources.slice(1).map(name => ({ name, items: results.filter(item => group(item) === name) })).filter(section => section.items.length && (source === 'All' || source === section.name)));
  let query = $state('');
  let results = $state.raw<Result[]>([]);
  let busy = $state(false);
  let opening = $state('');
  let feedback = $state('');
  let searched = $state(false);
  let input = $state<HTMLInputElement>();
  let request = 0;
  $effect(() => { if (open) void tick().then(() => input?.focus()); });
  async function search(event: SubmitEvent) {
    event.preventDefault();
    const text = query.trim();
    if (!text) { ++request; results = []; feedback = ''; searched = false; busy = false; return; }
    input?.blur();
    const current = ++request;
    busy = true; searched = true; feedback = ''; results = []; source = 'All';
    const matches = (title: string) => title.toLocaleLowerCase().includes(text.toLocaleLowerCase());
    const catalog = (kind: 'movie' | 'series') => (kind === 'movie' ? searchMovies(text) : searchShows(text)).then(items => {
      if (current === request) {
        const group = items.map(item => ({ key: kind + item.id, title: item.title, subtitle: `Stremio - ${kind === 'movie' ? 'Movie' : 'Show'} ${item.year}`, image: item.poster, target: { kind, id: item.id } }));
        results = [...results, ...group].sort((a, b) => Number(a.key.startsWith('series')) - Number(b.key.startsWith('series')));
      }
      return items;
    });
    const sources = await Promise.allSettled([catalog('movie'), catalog('series'), api.search(text)]);
    if (current !== request) return;
    const found: Result[] = [];
    for (const [index, kind] of ['movie', 'series'].entries()) {
      const source = sources[index];
      if (source.status !== 'fulfilled' || !Array.isArray(source.value)) continue;
      for (const item of source.value) found.push({ key: kind + item.id, title: item.title, subtitle: `Stremio - ${kind === 'movie' ? 'Movie' : 'Show'} ${item.year}`, image: item.poster, target: { kind, id: item.id } });
    }
    const context = sources[2];
    let unavailable = sources.some(source => source.status === 'rejected');
    if (context.status === 'fulfilled' && !Array.isArray(context.value) && context.value.response.ok && context.value.data) {
      const data = context.value.data;
      for (const item of data.videos) found.push({ key: 'video' + item.id, title: item.title, subtitle: 'YouTube', image: `https://i.ytimg.com/vi/${item.id}/hqdefault.jpg`, target: { kind: 'video', id: item.id } });
      let sports: SportsEvent[] = [];
      let primary: SportsEvent[] = [];
      try { if (data.sportsHtml) sports = parseSportsListing(data.sportsHtml); } catch { unavailable = true; }
      try { if (data.streamEast) primary = parseStreamEast(data.streamEast); } catch { unavailable = true; }
      for (const item of mergeSportsSources(primary, sports).filter(item => matches(item.title + ' ' + item.league))) found.push({ sports: item, key: 'sports' + item.id, title: item.title, subtitle: `${item.league} - ${item.live ? 'Live' : item.time}`, image: item.teams[0]?.logo || '', target: { kind: 'sports', url: item.url, title: item.title, sources: item.sources } });
      for (const item of data.history.filter(item => matches(item.title) && item.resumeKind !== 'none')) found.push({ key: 'activity' + item.id, title: item.title, subtitle: item.provider, image: item.artwork || '', target: { kind: 'activity', id: item.id } });
      unavailable ||= data.sportsUnavailable || data.youtubeUnavailable;
    } else unavailable = true;
    results = found;
    feedback = unavailable ? 'Some sources could not load. Search again to retry.' : found.length ? `${found.length} results` : 'No results found. Try another title, team, or league.';
    busy = false;
  }
  async function launch(item: Result) {
    if (opening) return;
    opening = item.key; feedback = '';
    try {
      const { response, data } = await api.openSearch(item.target);
      if (!response.ok || !data?.accepted) throw new Error(data?.error || 'Could not open this result. Try again.');
      say(`Opening ${item.title} on the display.`);
    } catch (error) { feedback = error instanceof Error ? error.message : 'Could not open this result.'; }
    finally { opening = ''; }
  }
</script>

<Modal id="search-dialog" titleId="search-title" {open} onrequestclose={onclose} className="search-dialog">
  <div class="search-header">
  <div class="flex items-center justify-between gap-3">
    <h2 id="search-title" class="text-[23px] font-semibold tracking-tight">Search Luma</h2>
    <button type="button" class="grid size-11 place-items-center rounded-full border border-line" aria-label="Back to remote" onclick={onclose}><Icon name="back" /></button>
  </div>
  <p class="search-intro">One search. Everything you watch.</p>
  <form onsubmit={search} class="mt-5 flex gap-2" role="search">
    <input bind:this={input} bind:value={query} type="search" aria-label="Search titles, teams, or leagues" placeholder="Titles, teams, or leagues" maxlength="200" enterkeyhint="search" autocomplete="off" spellcheck="false" required class="min-w-0 flex-1 rounded-xl border border-line-strong bg-canvas p-3 text-base" />
    <button class="min-h-11 rounded-xl bg-accent px-4 font-semibold text-accent-ink" type="submit">Search</button>
  </form>
  </div>
  <div class="search-body">
  <p class="my-4 min-h-5 text-sm text-muted" role="status">{busy ? 'Searching sources...' : feedback || (!searched ? 'Movies and shows from Stremio, videos, sports, and your sources.' : '')}</p>
  <nav class="source-tabs" aria-label="Search sources">
    {#each sources as name (name)}
      <button type="button" class:active={source === name} aria-pressed={source === name} onclick={() => source = name}>{name}<span>{name === 'All' ? results.length : results.filter(item => group(item) === name).length}</span></button>
    {/each}
  </nav>
  <div class="source-results" aria-busy={busy}>
    {#each groups as section (section.name)}
      <section class="source-section" aria-label={section.name}>
        <header><h3>{section.name}</h3>{#if section.name === 'Shows'}<a class="art-credit" href="https://www.tvmaze.com" target="_blank" rel="noreferrer">Artwork: TVmaze</a>{/if}<span>{section.items.length} results</span></header>
        <div class="result-grid">
          {#each section.items as item (item.key)}
            {#if item.sports}
              <div class="sports-result"><SportsEventCard event={item.sports} disabled={!!opening} onselect={() => launch(item)} /></div>
            {:else}
              <button type="button" class="result-card" disabled={!!opening} onclick={() => launch(item)}>
                <span class="result-art" class:poster={section.name === 'Movies' || section.name === 'Shows'}>
                  <PosterImage src={item.image} imdb={item.key.replace(/^(movie|series)/, '')} show={section.name === 'Shows'} />
                </span>
                <span class="result-copy"><strong>{item.title}</strong><span>{opening === item.key ? 'Opening on display?' : item.subtitle}</span></span>
              </button>
            {/if}
          {/each}
        </div>
      </section>
    {/each}
    {#if searched && !busy && !groups.length && results.length}<p class="empty-source">No {source.toLowerCase()} results. Choose another source.</p>{/if}
    {#if !searched}<div class="search-welcome"><Icon name="search" className="size-8 text-faint" /><h3>Find your next watch</h3><p>Search movies, shows, YouTube, live sports, and your history. Choose a result to play it on your display.</p></div>{/if}
  </div>
  </div>
</Modal>

<style>
  :global(dialog.search-dialog) { margin: auto; width: min(620px, calc(100% - 24px)); height: min(820px, calc(100dvh - 24px)); max-height: calc(100dvh - 24px); padding: 0; overflow: hidden; border: 1px solid var(--line-strong); border-radius: 22px; background: var(--surface-2); }
  .search-intro { margin: 8px 0 0; color: var(--muted); font-size: 14px; }
  .search-header { flex: none; padding: 20px; border-bottom: 1px solid var(--line); }
  .search-body { flex: 1; min-height: 0; overflow-y: auto; overscroll-behavior: contain; padding: 0 20px 20px; }
  @media (max-width: 639px) {
    :global(dialog.search-dialog) { width: 100%; max-width: 100%; height: 100dvh; max-height: 100dvh; margin: 0; border: 0; border-radius: 0; }
    .search-intro { margin: 8px 0 0; color: var(--muted); font-size: 14px; }
  .search-header { padding: max(16px, env(safe-area-inset-top)) max(16px, env(safe-area-inset-right)) 16px max(16px, env(safe-area-inset-left)); }
    .search-body { padding: 0 max(16px, env(safe-area-inset-right)) max(20px, env(safe-area-inset-bottom)) max(16px, env(safe-area-inset-left)); }
  }
  .source-tabs { display: flex; gap: 8px; overflow-x: auto; position: sticky; top: 0; z-index: 2; padding: 12px 0; background: var(--surface-2); scrollbar-width: none; }
  .source-tabs button { display: flex; align-items: center; gap: 7px; flex: none; min-height: 44px; padding: 0 15px; border: 1px solid var(--line); border-radius: 99px; color: var(--muted); font-size: 13px; font-weight: 650; }
  .source-tabs button.active { background: var(--accent); color: var(--accent-ink); border-color: transparent; }
  .source-tabs button span { font-size: 11px; opacity: .7; }
  .source-section { margin: 12px 0 28px; }
  .source-section header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 12px; }
  .source-section h3 { font-size: 19px; font-weight: 700; }
  .art-credit { color: var(--muted); font-size: 11px; margin-left: auto; margin-right: 12px; }
  .source-section header > span { font-size: 12px; color: var(--muted); }
  .result-grid { display: grid; grid-template-columns: repeat(2,minmax(0,1fr)); gap: 14px; }
  .result-card { min-width: 0; text-align: left; border-radius: 14px; overflow: hidden; border: 1px solid var(--line); background: var(--surface); }
  .result-card:disabled { opacity: .6; }
  .result-art { position: relative; display: grid; place-items: center; aspect-ratio: 16/9; background: var(--bg); overflow: hidden; }
  .result-art.poster { aspect-ratio: 2/3; }
  .result-copy { display: grid; gap: 6px; padding: 12px; }
  .result-copy strong { font-size: 14px; line-height: 1.35; display: -webkit-box; -webkit-line-clamp: 2; line-clamp: 2; -webkit-box-orient: vertical; overflow: hidden; }
  .result-copy > span { color: var(--muted); font-size: 11px; }
  .sports-result { grid-column: 1/-1; }
  .search-welcome { display: grid; justify-items: center; gap: 14px; padding: 50px 20px; text-align: center; }
  .search-welcome h3 { font-size: 22px; font-weight: 700; }
  .search-welcome p,.empty-source { color: var(--muted); font-size: 14px; line-height: 1.6; }
  @media (prefers-reduced-motion: no-preference) { .result-card { transition: transform .15s; } .result-card:active { transform: scale(.97); } }
</style>
