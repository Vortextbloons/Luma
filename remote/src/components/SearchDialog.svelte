<script lang="ts">
  import { tick } from 'svelte';
  import Modal from './Modal.svelte';
  import Icon from './Icon.svelte';
  import { api } from '../lib/api';
  import { searchMovies, searchShows } from '../../../src/lib/services/catalog';
  import { parseSportsListing, type SportsEvent } from '../../../src/lib/services/sports';
  import { parseStreamEast, mergeSportsSources } from '../../../src/lib/services/sportsSources';
  import { say } from '../lib/toast.svelte';
  let { open = false, onclose }: { open?: boolean; onclose: () => void } = $props();
  type Result = { key: string; title: string; subtitle: string; image: string; target: unknown };
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
    busy = true; searched = true; feedback = ''; results = [];
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
      for (const item of data.videos.filter(item => matches(item.title))) found.push({ key: 'video' + item.id, title: item.title, subtitle: 'YouTube', image: `https://i.ytimg.com/vi/${item.id}/hqdefault.jpg`, target: { kind: 'video', id: item.id } });
      let sports: SportsEvent[] = [];
      let primary: SportsEvent[] = [];
      try { if (data.sportsHtml) sports = parseSportsListing(data.sportsHtml); } catch { unavailable = true; }
      try { if (data.streamEast) primary = parseStreamEast(data.streamEast); } catch { unavailable = true; }
      for (const item of mergeSportsSources(primary, sports).filter(item => matches(item.title + ' ' + item.league))) found.push({ key: 'sports' + item.id, title: item.title, subtitle: `${item.league} - ${item.live ? 'Live' : item.time}`, image: item.teams[0]?.logo || '', target: { kind: 'sports', url: item.url, title: item.title, sources: item.sources } });
      for (const item of data.history.filter(item => matches(item.title) && item.resumeKind !== 'none')) found.push({ key: 'activity' + item.id, title: item.title, subtitle: item.provider, image: item.artwork || '', target: { kind: 'activity', id: item.id } });
      unavailable ||= data.sportsUnavailable;
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
  <p class="mt-2 text-sm text-muted">Find something to watch. Tap a result to open it on the display.</p>
  <form onsubmit={search} class="mt-5 flex gap-2" role="search">
    <input bind:this={input} bind:value={query} type="search" aria-label="Search titles, teams, or leagues" placeholder="Titles, teams, or leagues" maxlength="200" enterkeyhint="search" autocomplete="off" spellcheck="false" required class="min-w-0 flex-1 rounded-xl border border-line-strong bg-canvas p-3 text-base" />
    <button class="min-h-11 rounded-xl bg-accent px-4 font-semibold text-accent-ink" type="submit">Search</button>
  </form>
  </div>
  <div class="search-body">
  <p class="my-4 min-h-5 text-sm text-muted" role="status">{busy ? 'Searching sources...' : feedback || (!searched ? 'Movies and shows from Stremio, videos, sports, and your sources.' : '')}</p>
  <div class="grid grid-cols-2 gap-3 sm:grid-cols-3" aria-busy={busy}>
    {#each results as item (item.key)}
      <button type="button" class="overflow-hidden rounded-xl border border-line bg-surface text-left disabled:opacity-60" disabled={!!opening} onclick={() => launch(item)}>
        <span class="flex aspect-[4/3] items-center justify-center overflow-hidden bg-canvas">
          {#if item.image}<img src={item.image} alt="" loading="lazy" class="size-full object-contain" />{:else}<Icon name="display" className="size-10 text-faint" />{/if}
        </span>
        <span class="block p-3"><strong class="line-clamp-2 text-sm">{item.title}</strong><span class="mt-1 block text-xs text-muted">{opening === item.key ? 'Opening on display...' : item.subtitle}</span></span>
      </button>
    {/each}
  </div>
  </div>
</Modal>

<style>
  :global(dialog.search-dialog) { margin: auto; width: min(620px, calc(100% - 24px)); height: min(820px, calc(100dvh - 24px)); max-height: calc(100dvh - 24px); padding: 0; overflow: hidden; border: 1px solid var(--line-strong); border-radius: 22px; background: var(--surface-2); }
  .search-header { flex: none; padding: 20px; border-bottom: 1px solid var(--line); }
  .search-body { flex: 1; min-height: 0; overflow-y: auto; overscroll-behavior: contain; padding: 0 20px 20px; }
  @media (max-width: 639px) {
    :global(dialog.search-dialog) { width: 100%; max-width: 100%; height: 100dvh; max-height: 100dvh; margin: 0; border: 0; border-radius: 0; }
    .search-header { padding: max(16px, env(safe-area-inset-top)) max(16px, env(safe-area-inset-right)) 16px max(16px, env(safe-area-inset-left)); }
    .search-body { padding: 0 max(16px, env(safe-area-inset-right)) max(20px, env(safe-area-inset-bottom)) max(16px, env(safe-area-inset-left)); }
  }
</style>
