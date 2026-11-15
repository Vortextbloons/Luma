<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { SvelteMap } from 'svelte/reactivity';
  import { cachedSportsArtwork, cachedSportsScores, loadSportsArtwork, cachedSportsListing, isSportsFavorite, loadSportsListing, loadSportsScores, majorSportsLeagues, onSportsClosed, openSportsStream, readSportsFavorites, toggleSportsFavorite, updateSportsScore, writeSportsFavorites, type SportsEvent, type SportsFavorite, type SportsListing } from '../services/sports';
  import { matchScore, scoreSummary, scoreTeam, type GameScore } from '../services/sportsScores';
  import { slideIndicator } from '../services/motion';
  import nbaIcon from '../../../src-tauri/resources/icons/sports-nba.png';
  import nflIcon from '../../../src-tauri/resources/icons/sports-nfl.png';
  import mlbIcon from '../../../src-tauri/resources/icons/sports-mlb.png';
  import nhlIcon from '../../../src-tauri/resources/icons/sports-nhl.png';

  let { native }: { native: boolean } = $props();
  let listing = $state.raw<SportsListing | null>(cachedSportsListing());
  const cachedScores = cachedSportsScores();
  let scores = $state.raw<GameScore[]>(cachedScores.scores);
  let loading = $state(false);
  let error = $state('');
  let scoreError = $state('');
  let scoreUpdated = $state(cachedScores.updatedAt);
  let filter = $state('All');
  let previewId = $state<string | null>(null);
  let selected = $state<SportsEvent | null>(null);
  let detailError = $state('');
  let opening = $state(false);
  let watching = $state(false);
  let favorites = $state<SportsFavorite[]>(readSportsFavorites());
  let hideScores = $state(readHideScores());
  const artworkCache = new SvelteMap<string, string>(cachedSportsArtwork());
  let sourceIndexes: Record<string, number> = {};
  let returnFocus: HTMLElement | null = null;
  let active = false;
  let scoreBusy = false;
  const events = $derived(listing?.events ?? []);
  const filters = $derived(['All', 'Your teams', ...majorSportsLeagues, ...(events.some(event => !majorSportsLeagues.includes(event.league)) ? ['More'] : [])]);
  const leagueIcons: Record<string, string> = { NBA: nbaIcon, NFL: nflIcon, MLB: mlbIcon, NHL: nhlIcon };
  const visible = $derived(events.filter(event => filter === 'All' || (filter === 'Your teams' ? eventFavorite(event) : filter === 'More' ? !majorSportsLeagues.includes(event.league) : event.league === filter)));
  const liveEvents = $derived(visible.filter(event => live(event)));
  const upcoming = $derived(visible.filter(event => !live(event) && game(event)?.state !== 'post'));
  const finished = $derived(visible.filter(event => game(event)?.state === 'post'));
  const featured = $derived(visible.find(event => event.id === previewId) ?? liveEvents.find(event => eventFavorite(event)) ?? liveEvents.find(event => game(event)) ?? liveEvents[0] ?? upcoming.find(event => game(event)) ?? upcoming[0] ?? visible[0]);
  const featuredGame = $derived(featured ? game(featured) : undefined);
  const selectedGame = $derived(selected ? game(selected) : undefined);
  const artwork = $derived(featuredGame ? artworkCache.get(featuredGame.id) ?? '' : '');
  const sections = $derived([{ title: 'Live Now', events: liveEvents }, { title: 'Upcoming', events: upcoming }, { title: 'Final scores', events: finished }].filter(section => section.events.length));

  function readHideScores() { try { return localStorage.getItem('luma-sports-hide-scores') === 'true'; } catch { return false; } }
  function game(event: SportsEvent) { return matchScore(event, scores); }
  function live(event: SportsEvent) { const matched = game(event); return matched ? matched.state === 'in' : event.live; }
  function eventFavorite(event: SportsEvent) { return event.teams.some(team => isSportsFavorite(favorites, event.league, team.name)); }
  function teamLogo(event: SportsEvent, name: string) { return scoreTeam(game(event), name)?.logo || event.teams.find(team => team.name === name)?.logo || ''; }
  function gameTime(event: SportsEvent) {
    const matched = game(event);
    if (!matched) return event.time;
    if (hideScores && matched.state !== 'pre') return matched.state === 'in' ? 'Live now' : 'Completed';
    if (matched.state !== 'pre') return matched.status;
    const date = new Date(matched.date);
    return Number.isNaN(date.getTime()) ? event.time : date.toLocaleString([], { weekday: 'short', hour: 'numeric', minute: '2-digit' });
  }

  $effect(() => {
    const matched = featuredGame;
    if (native && matched && !artworkCache.has(matched.id)) {
      const timer = setTimeout(() => {
        void loadSportsArtwork(matched.league, matched.id)
          .then(url => artworkCache.set(matched.id, url)).catch(() => artworkCache.set(matched.id, ''));
      }, 180);
      return () => clearTimeout(timer);
    }
  });

  async function refreshScores(force = false) {
    if (!native || scoreBusy || !active) return;
    const leagues = [...new Set((watching && selected ? [selected] : events).map(event => event.league))].filter(league => majorSportsLeagues.includes(league));
    if (!leagues.length) return;
    scoreBusy = true;
    const results = await Promise.allSettled(leagues.map(league => loadSportsScores(league, force)));
    if (active) {
      const succeeded = leagues.filter((_, index) => results[index].status === 'fulfilled');
      scores = [...scores.filter(score => !succeeded.includes(score.league)), ...results.flatMap(result => result.status === 'fulfilled' ? result.value : [])];
      scoreError = results.some(result => result.status === 'rejected') ? 'Some scores could not refresh. Game streams are still available.' : '';
      if (succeeded.length) scoreUpdated = cachedSportsScores().updatedAt;
      if (watching && selected) void updateSportsScore(scoreSummary(game(selected), hideScores)).catch(() => {});
    }
    scoreBusy = false;
  }

  async function load(force = false) {
    if (!native || loading) return;
    loading = true;
    error = '';
    try { listing = await loadSportsListing(force); await refreshScores(force); }
    catch { error = listing ? 'Games could not refresh. Showing the last loaded list.' : 'Games could not load. Check your connection and try again.'; }
    finally { loading = false; }
  }
  export async function showLeague(league: string) {
    opening = false;
    watching = false;
    closeDetail();
    filter = filters.includes(league) ? league : 'All';
    previewId = null;
    await tick();
    document.querySelector<HTMLElement>('.sports-filters button[aria-pressed="true"]')?.focus({ preventScroll: true });
  }
  function selectEvent(event: SportsEvent, origin: HTMLElement) { returnFocus = origin; selected = event; detailError = ''; }
  export function closeDetail() {
    if (!selected || opening) return false;
    selected = null;
    detailError = '';
    const target = returnFocus;
    returnFocus = null;
    void tick().then(() => target?.focus({ preventScroll: true }));
    return true;
  }
  async function openStream(nextSource = false) {
    const event = selected;
    if (!event || opening) return;
    const current = sourceIndexes[event.id] ?? 0;
    const index = nextSource ? current + 1 : current;
    opening = true;
    detailError = '';
    try {
      await openSportsStream(event.url, index, event.title, scoreSummary(game(event), hideScores), event.sources);
      sourceIndexes[event.id] = index;
      watching = true;
    } catch { detailError = 'The stream could not open. Try another source.'; }
    finally { opening = false; }
  }
  function watchEvent(event: SportsEvent, origin: HTMLElement) { selectEvent(event, origin); void openStream(); }
  function favorite(league: string, team: string) { favorites = toggleSportsFavorite(favorites, league, team); writeSportsFavorites(favorites); }
  function setFilter(name: string) { if (filter === name) return; filter = name; previewId = null; }
  function toggleScores() {
    hideScores = !hideScores;
    try { localStorage.setItem('luma-sports-hide-scores', String(hideScores)); } catch {}
    if (watching && selected) void updateSportsScore(scoreSummary(game(selected), hideScores)).catch(() => {});
  }
  function mountDialog(dialog: HTMLDialogElement) { dialog.showModal(); requestAnimationFrame(() => dialog.querySelector<HTMLElement>('[data-sports-action="watch"]')?.focus({ preventScroll: true })); }
  onMount(() => {
    active = true;
    let stop: (() => void) | undefined;
    let timer: ReturnType<typeof setTimeout>;
    const schedule = () => { timer = setTimeout(() => {
      if (document.visibilityState === 'visible' || watching) void refreshScores().finally(() => { if (active) schedule(); });
      else schedule();
    }, watching || events.some(event => live(event)) ? 30000 : 120000); };
    if (native) {
      void onSportsClosed(() => { if (active) { watching = false; opening = false; closeDetail(); } }).then(unlisten => { if (active) stop = unlisten; else unlisten(); }).catch(() => {});
      void load();
      schedule();
    }
    return () => { active = false; stop?.(); clearTimeout(timer); };
  });
</script>

<section class="sports-home" aria-label="Sports" aria-busy={loading}>
  {#if featured}
    <section class="sports-hero" aria-label="Featured game">
      {#key artwork}
        {#if artwork}<img class="hero-art" src={artwork} alt="" onerror={() => { if (featuredGame) artworkCache.set(featuredGame.id, ''); }} />{/if}
      {/key}
      <div class="hero-wash"></div>
      <div class="hero-copy">
        {#key featured.id}
          <div class="hero-copy-inner">
            <div class="game-tags"><span class="league">{featured.league}</span><span class={{ 'live-badge': live(featured) }}>{live(featured) ? '● LIVE' : 'FEATURED MATCHUP'}</span></div>
            <h1>{featured.title}</h1>
            <p>{featuredGame?.venue || 'Your front row to the game.'}{featuredGame?.venue ? ` · ${gameTime(featured)}` : ''}</p>
            <div class="hero-actions">
              <button class="btn btn-primary" onclick={(event) => watchEvent(featured, event.currentTarget)}>▶ {live(featured) ? 'Watch live' : 'Watch game'}</button>
              <button class="btn btn-glass" onclick={(event) => selectEvent(featured, event.currentTarget)}>Game details</button>
            </div>
          </div>
        {/key}
      </div>
      {#key featured.id}
        <div class="hero-matchup" aria-label={hideScores ? 'Featured teams' : 'Featured scoreboard'}>
          {#each featured.teams as team (team.name)}
            {@const info = scoreTeam(featuredGame, team.name)}
            <div class="hero-team">
              {#if teamLogo(featured, team.name)}<img src={teamLogo(featured, team.name)} alt="" />{/if}
              <span>{info?.abbreviation || team.name}</span>
              {#if !hideScores && featuredGame && featuredGame.state !== 'pre'}<strong>{info?.score ?? '–'}</strong>{/if}
            </div>
          {/each}
          <span class="hero-clock">{gameTime(featured)}</span>
        </div>
      {/key}
    </section>
  {:else}
    <header class="sports-heading"><span class="eyebrow">GAME DAY, TOGETHER</span><h1>Sports</h1><p>Your teams. Every matchup. One place.</p></header>
  {/if}

  <div class="sports-toolbar">
    <nav class="sports-filters" aria-label="Filter sports" use:slideIndicator={{ key: filter }}>
      <span class="filter-pill" aria-hidden="true"></span>
      {#each filters as name (name)}<button class={{ active: filter === name }} aria-pressed={filter === name} onclick={() => setFilter(name)}>{#if leagueIcons[name]}<img src={leagueIcons[name]} alt="" />{/if}{name === 'Your teams' ? '☆ Your teams' : name}</button>{/each}
    </nav>
    <div class="toolbar-actions">
      <button aria-pressed={hideScores} onclick={toggleScores}>{hideScores ? 'Show scores' : 'Hide scores'}</button>
      {#if native}<button disabled={loading} onclick={() => void load(true)}>{loading ? 'Refreshing…' : 'Refresh'}</button>{/if}
    </div>
  </div>
  {#if error || scoreError}<p class="sports-feedback" role="status">{error || scoreError}</p>{/if}
  {#if !native}<div class="sports-state"><h2>Open Luma on your PC</h2><p>Live sports are available in the desktop app.</p></div>
  {:else if !listing && loading}<div class="sports-state" role="status"><h2>Getting ready for game time</h2><p>Loading live and upcoming matchups…</p></div>
  {:else if !listing}<div class="sports-state"><h2>Games could not load</h2><p>{error}</p><button class="btn btn-glass" onclick={() => void load(true)}>Try again</button></div>
  {:else if !visible.length}<div class="sports-state"><h2>{filter === 'Your teams' ? 'Your teams play here' : `No ${filter === 'All' ? '' : filter + ' '}games listed right now`}</h2><p>{filter === 'Your teams' ? 'Open a game and favorite a team to find their matchups here.' : 'Choose another league or refresh for the latest games.'}</p><button class="btn btn-glass" onclick={() => setFilter('All')}>Show all games</button></div>
  {:else}
    {#key filter}
      <div class="sports-results">
        {#each sections as section (section.title)}
          <section class="game-section" aria-label={section.title}>
            <header class="section-heading"><h2>{section.title} <span>{section.events.length}</span></h2></header>
            <div class="sports-grid">
              {#each section.events as event (event.id)}
                {@const matched = game(event)}
                <button class="sports-card" data-focus-key={event.id} onmouseenter={() => previewId = event.id} onfocus={() => previewId = event.id} style:--team-color={scoreTeam(matched, event.teams[0]?.name)?.color || '#39374f'} onclick={(click) => selectEvent(event, click.currentTarget)} aria-label={`${event.title}, ${event.league}, ${gameTime(event)}`}>
                  {#if teamLogo(event, event.teams[0]?.name)}<img class="card-watermark" src={teamLogo(event, event.teams[0].name)} alt="" loading="lazy" />{/if}
                  <span class="game-tags"><span class="league">{event.league}</span>{#if live(event)}<span class="live-badge">● LIVE</span>{/if}{#if eventFavorite(event)}<span class="favorite-mark" aria-label="Favorite team">★</span>{/if}</span>
                  <span class="card-matchup">
                    {#each event.teams as team (team.name)}
                      {@const info = scoreTeam(matched, team.name)}
                      <span class="team-row">
                        <span class="team-logo">{#if teamLogo(event, team.name)}<img src={teamLogo(event, team.name)} alt="" loading="lazy" decoding="async" />{:else}{team.name.slice(0, 1)}{/if}</span>
                        <span class="team-name">{team.name}</span>
                        {#if matched && matched.state !== 'pre' && !hideScores}<strong class="team-score">{info?.score ?? '–'}</strong>{/if}
                      </span>
                    {/each}
                  </span>
                  <span class="card-footer"><span class="watch-label">▶ {live(event) ? 'Watch live' : 'Game details'}</span><span>{gameTime(event)}</span></span>
                </button>
              {/each}
            </div>
          </section>
        {/each}
      </div>
    {/key}
  {/if}
  {#if scoreUpdated}<p class="scores-credit">Scores: ESPN · Updated {new Date(scoreUpdated).toLocaleTimeString([], { hour: 'numeric', minute: '2-digit' })}{hideScores ? ' · Scores hidden' : ''}</p>{/if}
</section>

{#if selected}
  <dialog {@attach mountDialog} class="sports-dialog" aria-labelledby="sports-detail-title" oncancel={(event) => { event.preventDefault(); closeDetail(); }}>
    <div class="detail-top"><div class="game-tags"><span class="league">{selected.league}</span>{#if live(selected)}<span class="live-badge">● LIVE</span>{/if}</div><button class="close-btn" aria-label="Close game details" onclick={closeDetail}>×</button></div>
    <h2 id="sports-detail-title">{selected.title}</h2>
    <p class="detail-status">{gameTime(selected)}{selectedGame?.venue ? ` · ${selectedGame.venue}` : ''}</p>
    <div class="detail-teams">
      {#each selected.teams as team (team.name)}
        {@const info = scoreTeam(selectedGame, team.name)}
        <div class="detail-team"><span class="team-logo">{#if teamLogo(selected, team.name)}<img src={teamLogo(selected, team.name)} alt="" />{:else}{team.name.slice(0, 1)}{/if}</span><span class="detail-team-name">{team.name}{#if info?.record}<small>{info.record}</small>{/if}</span>{#if selectedGame && selectedGame.state !== 'pre' && !hideScores}<strong class="detail-score">{info?.score ?? '–'}</strong>{/if}<button class="favorite-button" aria-pressed={isSportsFavorite(favorites, selected.league, team.name)} aria-label={`Favorite ${team.name}`} onclick={() => favorite(selected!.league, team.name)}>{isSportsFavorite(favorites, selected.league, team.name) ? '★' : '☆'}</button></div>
      {/each}
    </div>
    {#if detailError}<p class="sports-feedback" role="alert">{detailError}</p>{/if}
    <div class="detail-actions"><button class="btn btn-primary" data-sports-action="watch" disabled={opening} onclick={() => void openStream()}>{opening ? 'Starting stream…' : '▶ Watch fullscreen'}</button><button class="btn btn-glass" disabled={opening} onclick={() => void openStream(true)}>Try another source</button></div>
    <p class="detail-hint">{selected.provider === 'StreamEast' ? 'StreamEast first. Available Sportsurge sources are tried if needed.' : 'Sportsurge fallback.'} Use your remote to play, pause, and change sources. Back returns to Sports.</p>
  </dialog>
{/if}

<style>
  .sports-home{padding:0 var(--side) 40px}.sports-heading{padding:20px 0}.sports-heading h1{font-size:48px;margin:8px 0}.sports-heading p{color:var(--muted)}.sports-results{animation:sports-results-in var(--motion-base) var(--ease-out)}@keyframes sports-results-in{from{opacity:0;transform:translateY(6px)}to{opacity:1;transform:none}}
  .sports-hero{position:relative;display:grid;grid-template-columns:minmax(0,1.35fr) minmax(280px,.8fr);align-items:center;gap:clamp(16px,2vw,30px);height:clamp(330px,23vw,400px);overflow:hidden;border:1px solid var(--line-strong);border-radius:22px;background:linear-gradient(115deg,var(--surface-3),var(--bg));isolation:isolate}.hero-art{position:absolute;inset:0;width:100%;height:100%;object-fit:cover;z-index:-3;opacity:.7;animation:sports-hero-art-in .22s var(--ease-out)}@keyframes sports-hero-art-in{from{opacity:0}to{opacity:.7}}.hero-wash{position:absolute;inset:0;z-index:-2;background:linear-gradient(90deg,#0b0d12f5 10%,#111320a8 55%,#0b0d1244),linear-gradient(0deg,#0b0d12b8,transparent 80%)}.hero-copy{min-width:0;padding:clamp(26px,2.7vw,50px)}.hero-copy-inner{animation:sports-hero-swap .18s var(--ease-out)}.hero-copy h1{max-width:26ch;min-height:2.1em;margin:16px 0 12px;font-size:clamp(30px,3.3vw,58px);line-height:1.05;font-weight:750;letter-spacing:-.045em;display:-webkit-box;-webkit-line-clamp:2;line-clamp:2;-webkit-box-orient:vertical;overflow:hidden;text-wrap:balance}.hero-copy p{max-width:100%;margin:0;color:#d5d8e2;font-size:clamp(15px,1.05vw,20px);line-height:1.4;white-space:nowrap;overflow:hidden;text-overflow:ellipsis}.hero-actions{display:flex;flex-wrap:wrap;gap:12px;margin-top:24px}.hero-actions .btn{min-height:52px;padding:0 24px;font-size:clamp(16px,1.1vw,20px)}.hero-matchup{display:flex;flex-wrap:wrap;justify-content:center;align-items:center;gap:clamp(12px,1.4vw,20px) clamp(16px,2vw,28px);min-width:0;padding:clamp(20px,2.5vw,34px) clamp(14px,2.5vw,36px);animation:sports-hero-swap .18s var(--ease-out)}.hero-team{display:flex;align-items:center;justify-content:center;gap:12px;min-width:0;max-width:100%}.hero-team img{width:clamp(56px,5vw,94px);height:clamp(56px,5vw,94px);object-fit:contain;filter:drop-shadow(0 6px 14px #0008)}.hero-team span{min-width:0;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;font-size:clamp(16px,1.3vw,24px);font-weight:700}.hero-team strong{min-width:2ch;text-align:center;font-size:clamp(36px,3vw,54px);font-variant-numeric:tabular-nums}.hero-clock{flex-basis:100%;text-align:center;font-size:clamp(15px,1.1vw,20px);color:#d5d8e2}@keyframes sports-hero-swap{from{opacity:.6;transform:translateY(4px)}to{opacity:1;transform:none}}
  .sports-toolbar{display:flex;align-items:center;justify-content:space-between;gap:18px;margin:22px 0 26px}.sports-filters{position:relative;display:flex;flex-wrap:wrap;gap:8px}.filter-pill{position:absolute;left:0;top:0;width:var(--indicator-w,0);height:var(--indicator-h,0);border-radius:99px;background:var(--text);transform:translate(var(--indicator-x,0),var(--indicator-y,0));opacity:0;pointer-events:none}.sports-filters:global([data-indicator=ready]) .filter-pill{opacity:1;transition:transform var(--motion-base) var(--ease-out),width var(--motion-base) var(--ease-out),height var(--motion-base) var(--ease-out)}.sports-filters button,.toolbar-actions button{min-height:46px;padding:0 20px;border-radius:99px;border:1px solid var(--line-strong);background:var(--surface-2);color:var(--text-dim);font-size:clamp(14px,.95vw,18px);font-weight:600;white-space:nowrap}.sports-filters button{position:relative;z-index:1;display:inline-flex;align-items:center;gap:8px}.sports-filters img{width:24px;height:24px;object-fit:contain}.sports-filters button.active{background:transparent;color:var(--bg);border-color:transparent}.toolbar-actions{display:flex;gap:8px}.toolbar-actions button{padding:0 14px;font-size:14px;background:transparent}.section-heading{display:flex;justify-content:space-between;align-items:baseline;margin:0 0 14px}.section-heading h2{margin:0;font-size:clamp(23px,1.65vw,31px);font-weight:700;letter-spacing:-.025em}.game-section{margin-bottom:28px}.sports-grid{display:grid;grid-auto-flow:column;grid-auto-columns:calc((100% - 54px)/4);gap:18px;overflow-x:auto;scroll-snap-type:x proximity;padding:7px;margin:-7px;scrollbar-width:none}.sports-grid::-webkit-scrollbar{display:none}.sports-card{scroll-snap-align:start}.section-heading h2>span{font-size:14px;font-weight:500;color:var(--muted);margin-left:8px}.sports-card{position:relative;isolation:isolate;display:flex;flex-direction:column;gap:18px;min-height:200px;padding:20px;border:1px solid var(--line-strong);border-radius:17px;background:linear-gradient(115deg,var(--surface-2) 35%,color-mix(in srgb,var(--team-color) 35%,var(--surface-2)));color:var(--text);text-align:left;overflow:hidden;transition:transform var(--motion-fast),box-shadow var(--motion-fast)}.sports-card:focus{outline:3px solid var(--accent);outline-offset:3px;box-shadow:0 0 25px var(--accent-soft);transform:translateY(-2px)}.sports-card:hover{border-color:var(--accent)}.sports-card:is(:hover,:focus) .card-watermark{transform:rotate(-5deg) scale(1.1)}.card-watermark{position:absolute;right:-30px;bottom:-30px;width:190px;height:190px;object-fit:contain;opacity:.11;z-index:-1;transform:rotate(-12deg);transition:transform var(--motion-base) var(--ease-out)}.game-tags{display:flex;align-items:center;gap:10px;min-height:24px;font-size:12px;font-weight:700;letter-spacing:.045em}.league{padding:4px 10px;border:1px solid #ffffff35;border-radius:99px;color:var(--text)}.live-badge{color:#ffb4be;background:#a12c4338;padding:4px 9px;border-radius:99px}.favorite-mark{margin-left:auto;color:#ffd28d}.card-matchup{display:grid;gap:10px;flex:1}.team-row{display:flex;align-items:center;gap:12px}.team-logo{display:grid;place-items:center;flex:none;width:42px;height:42px;font-weight:700}.team-logo img{width:100%;height:100%;object-fit:contain}.team-name{flex:1;min-width:0;font-size:clamp(17px,1.1vw,21px);font-weight:650;line-height:1.2}.team-score{font-size:27px;font-weight:750;font-variant-numeric:tabular-nums}.card-footer{display:flex;align-items:center;justify-content:space-between;gap:10px;font-size:13px;color:var(--text-dim)}.watch-label{padding:7px 12px;border-radius:99px;background:var(--accent-soft);color:var(--text);font-weight:650}.sports-state{padding:40px;border:1px solid var(--line);border-radius:18px;background:var(--surface-2)}.sports-state h2{font-size:28px;margin:0 0 12px}.sports-state p{color:var(--muted);margin:0 0 24px}.sports-feedback{color:var(--warn);font-size:15px;line-height:1.5}.scores-credit{font-size:12px;color:var(--dim);margin:14px 0 0}
  .sports-dialog{width:min(760px,calc(100vw - 48px));max-height:calc(100dvh - 48px);margin:auto;padding:32px;overflow-y:auto;border:1px solid var(--line-strong);border-radius:24px;background:var(--surface-2);color:var(--text);box-shadow:0 24px 90px #0008}.sports-dialog::backdrop{background:#05060acb;backdrop-filter:blur(8px)}.detail-top{display:flex;align-items:center;justify-content:space-between;gap:20px}.sports-dialog h2{margin:20px 0 10px;font-size:clamp(28px,2.5vw,42px);line-height:1.1;letter-spacing:-.035em}.detail-status{color:var(--muted);font-size:16px;line-height:1.5}.detail-teams{display:grid;gap:12px;margin:28px 0}.detail-team{display:flex;align-items:center;gap:16px;padding:16px;border:1px solid var(--line);border-radius:14px;background:#ffffff04}.detail-team .team-logo{width:56px;height:56px}.detail-team-name{flex:1;font-size:23px;font-weight:650}.detail-team-name small{display:block;color:var(--muted);font-size:14px;font-weight:500;margin-top:5px}.detail-score{font-size:36px;font-variant-numeric:tabular-nums}.favorite-button{width:48px;height:48px;border:1px solid var(--line-strong);border-radius:50%;background:var(--surface-3);color:var(--text-dim);font-size:26px}.favorite-button[aria-pressed=true]{color:#ffd28d}.detail-actions{display:flex;gap:12px}.detail-actions .btn{flex:1;min-height:58px;font-size:18px}.detail-hint{margin:20px 0 0;color:var(--muted);font-size:14px;line-height:1.5}
  @media(max-width:1050px){.sports-grid{grid-auto-columns:calc((100% - 18px)/2)}.sports-toolbar{align-items:flex-start;flex-direction:column}.hero-matchup{gap:12px;padding:20px 22px 20px 12px}.hero-team{gap:8px}.hero-team span{display:none}}@media(max-width:760px){.sports-hero{grid-template-columns:1fr;height:auto;gap:0}.hero-copy{max-width:100%;padding:26px}.hero-matchup{max-width:100%;padding:0 26px 26px;justify-content:flex-start}.hero-clock{text-align:left}.sports-grid{grid-auto-columns:85%}.detail-actions{flex-direction:column}.detail-team-name{font-size:18px}.hero-actions{flex-wrap:wrap}}
</style>
