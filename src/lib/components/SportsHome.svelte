<script lang="ts">
  import SportsEventCard from './SportsEventCard.svelte';
  import { onMount, tick, untrack } from 'svelte';
  import { SvelteMap } from 'svelte/reactivity';
  import { loadSportsTeams, isSportsFavorite, majorSportsLeagues, readSportsFavorites, sportsScoresHidden, toggleSportsFavorite, writeSportsFavorites, type SportsTeam, type SportsEvent, type SportsFavorite } from '../services/sports';
  import { matchEvent, matchScore, scoreTeam } from '../services/sportsScores';
  import { slideIndicator } from '../services/motion';
  import type { SportsState } from '../state/sports.svelte';
  import nbaIcon from '../../../src-tauri/resources/icons/sports-nba.png';
  import nflIcon from '../../../src-tauri/resources/icons/sports-nfl.png';
  import mlbIcon from '../../../src-tauri/resources/icons/sports-mlb.png';
  import nhlIcon from '../../../src-tauri/resources/icons/sports-nhl.png';

  let { native, sports, ondetailclose }: { native: boolean; sports: SportsState; ondetailclose?: () => void } = $props();
  const listing = $derived(sports.listing);
  const scores = $derived(sports.scores);
  const loading = $derived(sports.loading);
  const error = $derived(sports.error);
  const scoreError = $derived(sports.scoreError);
  const scoreUpdated = $derived(sports.updatedAt);
  const opening = $derived(sports.opening);
  let filter = $state('All');
  let managingTeams = $state(false);
  let teamLeague = $state("NBA");
  let teamOptions = $state.raw<SportsTeam[]>([]);
  const teamLogos = new SvelteMap<string, string>();
  let teamsLoading = $state(false);
  let teamsError = $state('');
  let teamRequest = 0;
  let selected = $state<SportsEvent | null>(null);
  let detailError = $state('');
  let favorites = $state<SportsFavorite[]>(readSportsFavorites());
  let hideScores = $state(sportsScoresHidden());
  let sourceIndexes: Record<string, number> = {};
  let returnFocus: HTMLElement | null = null;
  let active = false;
  const events = $derived(sports.events);
  const filters = $derived(['All', 'Your teams', ...majorSportsLeagues, ...(events.some(event => !majorSportsLeagues.includes(event.league)) ? ['More'] : [])]);
  const leagueIcons: Record<string, string> = { NBA: nbaIcon, NFL: nflIcon, MLB: mlbIcon, NHL: nhlIcon };
  const visible = $derived(events.filter(event => filter === 'All' || (filter === 'Your teams' ? eventFavorite(event) : filter === 'More' ? !majorSportsLeagues.includes(event.league) : event.league === filter)));
  const liveEvents = $derived(visible.filter(event => live(event)));
  const upcoming = $derived(visible.filter(event => !live(event) && game(event)?.state !== 'post'));
  const finished = $derived(visible.filter(event => game(event)?.state === 'post'));
  const featured = $derived(liveEvents.find(event => eventFavorite(event)) ?? liveEvents.find(event => game(event)) ?? liveEvents[0] ?? upcoming.find(event => game(event)) ?? upcoming[0] ?? visible[0]);
  const featuredGame = $derived(featured ? game(featured) : undefined);
  const selectedGame = $derived(selected ? game(selected) : undefined);
  const artwork = $derived(featuredGame ? sports.artworkFor(featuredGame.id) : '');
  const sections = $derived([{ title: 'Live Now', events: liveEvents }, { title: 'Upcoming', events: upcoming }, { title: 'Final scores', events: finished }].filter(section => section.events.length));

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
    if (native && matched) {
      const timer = setTimeout(() => { void sports.loadArtwork(matched.league, matched.id); }, 180);
      return () => clearTimeout(timer);
    }
  });

  let closedAt = untrack(() => sports.closedAt);
  $effect(() => {
    if (sports.closedAt === closedAt) return;
    closedAt = sports.closedAt;
    closeDetail();
  });

  async function refreshScores(force = false) {
    if (!native || !active) return;
    const leagues = [...new Set((sports.watching && selected ? [selected] : events).map(event => event.league))].filter(league => majorSportsLeagues.includes(league));
    await sports.refreshScores(leagues, force);
  }

  async function load(force = false) {
    await sports.load(force);
  }
  export async function showLeague(league: string) {
    closeDetail();
    filter = filters.includes(league) ? league : 'All';
    await tick();
    document.querySelector<HTMLElement>('.sports-filters button[aria-pressed="true"]')?.focus({ preventScroll: true });
  }
  function selectEvent(event: SportsEvent, origin: HTMLElement) { returnFocus = origin; selected = event; detailError = ''; }
  export function closeDetail() {
    if (sports.opening) return true;
    if (!selected && !managingTeams) return false;
    const wasDetail = !!selected;
    managingTeams = false;
    teamRequest++;
    selected = null;
    detailError = '';
    const target = returnFocus;
    returnFocus = null;
    void tick().then(() => {
      const focus = target?.isConnected ? target : document.querySelector<HTMLElement>('.sports-filters button[aria-pressed="true"]');
      focus?.focus({ preventScroll: true });
      if (wasDetail) ondetailclose?.();
    });
    return true;
  }
  async function openStream(nextSource = false) {
    const event = selected;
    if (!event || sports.opening || !native || !live(event) || !event.url) return;
    const current = sourceIndexes[event.id] ?? 0;
    const index = nextSource ? current + 1 : current;
    detailError = '';
    if (await sports.openEvent(event, game(event), index)) sourceIndexes[event.id] = index;
    else detailError = 'The stream could not open. Try another source.';
  }
  export async function showEvent(event: SportsEvent) {
    selected = event;
    detailError = '';
    if (!native || event.url) return;
    try {
      const latest = await sports.refreshListing();
      if (!active || selected?.id !== event.id) return;
      const score = game(event);
      const matched = score ? matchEvent(score, latest.events) : undefined;
      if (matched) selected = matched;
    } catch { if (active && selected?.id === event.id) detailError = 'Streams could not load. Close game details and refresh to try again.'; }
  }
  async function chooseTeamLeague(league: string) {
    teamLeague = league;
    const request = ++teamRequest;
    teamsLoading = true;
    teamsError = '';
    teamOptions = [];
    try {
      const names = await loadSportsTeams(league);
      if (request === teamRequest) {
        teamOptions = names;
        for (const team of names) teamLogos.set(league + ':' + team.name, team.logo);
      }
    } catch { if (request === teamRequest) teamsError = 'Teams could not load. Try again.'; }
    finally { if (request === teamRequest) teamsLoading = false; }
  }
  function manageTeams(origin: HTMLElement) {
    returnFocus = origin;
    managingTeams = true;
    void chooseTeamLeague(teamLeague);
  }
  function favorite(league: string, team: string) {
    const origin = document.activeElement as HTMLElement | null;
    favorites = toggleSportsFavorite(favorites, league, team);
    writeSportsFavorites(favorites);
    void tick().then(() => {
      if (managingTeams && !origin?.isConnected) document.querySelector<HTMLElement>('.team-leagues button[aria-pressed="true"]')?.focus({ preventScroll: true });
    });
  }
  function setFilter(name: string) { if (filter === name) return; filter = name; }
  function toggleScores() {
    hideScores = !hideScores;
    try { localStorage.setItem('luma-sports-hide-scores', String(hideScores)); } catch {}
    if (sports.watching) sports.refreshOverlay();
  }
  function mountDialog(dialog: HTMLDialogElement) { dialog.showModal(); requestAnimationFrame(() => dialog.querySelector<HTMLElement>('[data-sports-action="watch"]:not(:disabled), .close-btn')?.focus({ preventScroll: true })); }
  onMount(() => {
    active = true;
    let timer: ReturnType<typeof setTimeout>;
    const schedule = () => { timer = setTimeout(() => {
      if (document.visibilityState === 'visible' || sports.watching) void refreshScores().finally(() => { if (active) schedule(); });
      else schedule();
    }, sports.watching || events.some(event => live(event)) ? 30000 : 120000); };
    if (native) {
      void load();
      schedule();
    }
    return () => { active = false; clearTimeout(timer); };
  });
</script>

<section class="sports-home" aria-label="Sports" aria-busy={loading}>
  {#if featured}
    <section class="sports-hero" aria-label="Featured game">
      {#key artwork}
        {#if artwork}<img class="hero-art" src={artwork} alt="" onerror={() => { if (featuredGame) sports.clearArtwork(featuredGame.id); }} />{/if}
      {/key}
      <div class="hero-wash"></div>
      <div class="hero-copy">
        {#key featured.id}
          <div class="hero-copy-inner">
            <div class="game-tags"><span class="league">{featured.league}</span><span class={{ 'live-badge': live(featured) }}>{live(featured) ? '● LIVE' : 'FEATURED MATCHUP'}</span></div>
            <h1>{featured.title}</h1>
            <p>{featuredGame?.venue || 'Your front row to the game.'}{featuredGame?.venue ? ` · ${gameTime(featured)}` : ''}</p>
            <div class="hero-actions">
              <button class="btn btn-primary" onclick={(event) => selectEvent(featured, event.currentTarget)}>{featuredGame?.state === 'post' && !hideScores ? 'Final score' : 'Game details'}</button>
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
      {#if native}<button onclick={(event) => manageTeams(event.currentTarget)}>Manage teams</button>{/if}
      <button aria-pressed={hideScores} onclick={toggleScores}>{hideScores ? 'Show scores' : 'Hide scores'}</button>
      {#if native}<button disabled={loading} onclick={() => void load(true)}>{loading ? 'Refreshing…' : 'Refresh'}</button>{/if}
    </div>
  </div>
  {#if error || scoreError}<p class="sports-feedback" role="status">{error || scoreError}</p>{/if}
  {#if !native}<div class="sports-state"><h2>Open Luma on your PC</h2><p>Live sports are available in the desktop app.</p></div>
  {:else if !listing && loading}<div class="sports-state" role="status"><h2>Getting ready for game time</h2><p>Loading live and upcoming matchups…</p></div>
  {:else if !listing}<div class="sports-state"><h2>Games could not load</h2><p>{error}</p><button class="btn btn-glass" onclick={() => void load(true)}>Try again</button></div>
  {:else if !visible.length}<div class="sports-state"><h2>{filter === 'Your teams' ? 'Your teams play here' : `No ${filter === 'All' ? '' : filter + ' '}games listed right now`}</h2><p>{filter === 'Your teams' ? favorites.length ? 'None of your teams have a listed game right now.' : 'Choose your teams to find their matchups here.' : 'Choose another league or refresh for the latest games.'}</p><button class="btn btn-glass" onclick={() => setFilter('All')}>Show all games</button>{#if filter === 'Your teams'} <button class="btn btn-glass" onclick={(event) => manageTeams(event.currentTarget)}>Manage teams</button>{/if}</div>
  {:else}
    {#key filter}
      <div class="sports-results">
        {#each sections as section (section.title)}
          <section class="game-section" aria-label={section.title}>
            <header class="section-heading"><h2>{section.title} <span>{section.events.length}</span></h2></header>
            <div class="sports-grid">
              {#each section.events as event (event.id)}
                {@const matched = game(event)}
                <SportsEventCard {event} {matched} {hideScores} favorite={eventFavorite(event)} time={gameTime(event)} live={live(event)} onselect={origin => selectEvent(event, origin)} />
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
    <div class="detail-actions"><button class="btn btn-primary" data-sports-action="watch" disabled={opening || !native || !live(selected) || !selected.url} onclick={() => void openStream()}>{opening ? 'Starting stream…' : '▶ Watch fullscreen'}</button><button class="btn btn-glass" disabled={opening || !native || !live(selected) || !selected.url} onclick={() => void openStream(true)}>Try another source</button></div>
    <p class="detail-hint">{live(selected) ? 'Use your remote to play, pause, and change sources. Back returns to game details.' : selectedGame?.state === 'post' ? 'This game has ended. Replay availability is not provided.' : 'Live playback will be available when the game starts and a stream is listed.'}</p>
  </dialog>
{/if}


{#if managingTeams}
  <dialog {@attach mountDialog} class="sports-dialog" aria-labelledby="sports-teams-title" oncancel={(event) => { event.preventDefault(); closeDetail(); }}>
    <div class="detail-top"><h2 id="sports-teams-title">Your teams</h2><button class="close-btn" aria-label="Close team management" onclick={closeDetail}>&#215;</button></div>
    <p class="detail-status">Follow teams to find their games in Your teams. Changes save automatically.</p>
    <div class="team-leagues" aria-label="Team league">
      {#each majorSportsLeagues as league (league)}<button class="btn btn-glass" aria-pressed={teamLeague === league} onclick={() => void chooseTeamLeague(league)}><img class="picker-league-logo" src={leagueIcons[league]} alt="" />{league}</button>{/each}
    </div>
    {#if favorites.length}<h3>Following</h3><div class="team-options">{#each favorites as item (item.league + item.team)}<button class="btn btn-glass" aria-label={`Unfollow ${item.team}`} onclick={() => favorite(item.league, item.team)}><span class="team-logo">{#if teamLogos.get(item.league + ':' + item.team)}<img src={teamLogos.get(item.league + ':' + item.team)} alt="" loading="lazy" decoding="async" />{:else}<img src={leagueIcons[item.league]} alt="" />{/if}</span><span class="picker-team-name">{item.team}<small>{item.league}</small></span><span class="picker-followed" aria-hidden="true">&#9733;</span></button>{/each}</div>{/if}
    <h3>{teamLeague} teams</h3>
    {#if teamsLoading}<p role="status">Loading teams...</p>
    {:else if teamsError}<p class="sports-feedback" role="status">{teamsError}</p><button class="btn btn-glass" onclick={() => void chooseTeamLeague(teamLeague)}>Try again</button>
    {:else if !teamOptions.length}<p>No teams are available for this league.</p>
    {:else}<div class="team-options">{#each teamOptions as team (team.name)}<button class="btn btn-glass" aria-pressed={isSportsFavorite(favorites, teamLeague, team.name)} onclick={() => favorite(teamLeague, team.name)}><span class="team-logo">{#if team.logo}<img src={team.logo} alt="" loading="lazy" decoding="async" />{:else}{team.name.slice(0, 1)}{/if}</span><span class="picker-team-name">{team.name}</span><span class="picker-followed" aria-hidden="true">{isSportsFavorite(favorites, teamLeague, team.name) ? '\u2605' : '\u2606'}</span></button>{/each}</div>{/if}
  </dialog>
{/if}

<style>
  .picker-league-logo{width:28px;height:28px;object-fit:contain}.team-options .btn{display:flex;align-items:center;gap:14px;padding:14px 18px}.picker-team-name{flex:1;min-width:0;font-size:16px;line-height:1.3}.picker-team-name small{display:block;margin-top:4px;color:var(--muted);font-size:13px}.picker-followed{font-size:24px;color:var(--muted)}[aria-pressed=true] .picker-followed,.team-options button[aria-label] .picker-followed{color:#ffd28d}

  .team-leagues{display:flex;flex-wrap:wrap;gap:12px}.team-options{display:grid;grid-template-columns:repeat(2,minmax(0,1fr));gap:12px}.team-options .btn{min-height:54px;white-space:normal;text-align:left;justify-content:flex-start}.team-leagues [aria-pressed=true],.team-options [aria-pressed=true]{border-color:var(--accent);background:var(--accent-soft)}.toolbar-actions{flex-wrap:wrap}button:focus-visible{outline:3px solid var(--accent);outline-offset:3px}@media(max-width:760px){.team-options{grid-template-columns:1fr}}

  .sports-home{padding:0 var(--side) 40px}.sports-heading{padding:20px 0}.sports-heading h1{font-size:48px;margin:8px 0}.sports-heading p{color:var(--muted)}.sports-results{animation:sports-results-in var(--motion-base) var(--ease-out)}@keyframes sports-results-in{from{opacity:0;transform:translateY(6px)}to{opacity:1;transform:none}}
  .sports-hero{position:relative;display:grid;grid-template-columns:minmax(0,1.35fr) minmax(280px,.8fr);align-items:center;gap:clamp(16px,2vw,30px);height:clamp(330px,23vw,400px);overflow:hidden;border:1px solid var(--line-strong);border-radius:22px;background:linear-gradient(115deg,var(--surface-3),var(--bg));isolation:isolate}.hero-art{position:absolute;inset:0;width:100%;height:100%;object-fit:cover;z-index:-3;opacity:.7;animation:sports-hero-art-in .22s var(--ease-out)}@keyframes sports-hero-art-in{from{opacity:0}to{opacity:.7}}.hero-wash{position:absolute;inset:0;z-index:-2;background:linear-gradient(90deg,#0b0d12f5 10%,#111320a8 55%,#0b0d1244),linear-gradient(0deg,#0b0d12b8,transparent 80%)}.hero-copy{min-width:0;padding:clamp(26px,2.7vw,50px)}.hero-copy-inner{animation:sports-hero-swap .18s var(--ease-out)}.hero-copy h1{max-width:26ch;min-height:2.1em;margin:16px 0 12px;font-size:clamp(30px,3.3vw,58px);line-height:1.05;font-weight:750;letter-spacing:-.045em;display:-webkit-box;-webkit-line-clamp:2;line-clamp:2;-webkit-box-orient:vertical;overflow:hidden;text-wrap:balance}.hero-copy p{max-width:100%;margin:0;color:#d5d8e2;font-size:clamp(15px,1.05vw,20px);line-height:1.4;white-space:nowrap;overflow:hidden;text-overflow:ellipsis}.hero-actions{display:flex;flex-wrap:wrap;gap:12px;margin-top:24px}.hero-actions .btn{min-height:52px;padding:0 24px;font-size:clamp(16px,1.1vw,20px)}.hero-matchup{display:flex;flex-wrap:wrap;justify-content:center;align-items:center;gap:clamp(12px,1.4vw,20px) clamp(16px,2vw,28px);min-width:0;padding:clamp(20px,2.5vw,34px) clamp(14px,2.5vw,36px);animation:sports-hero-swap .18s var(--ease-out)}.hero-team{display:flex;align-items:center;justify-content:center;gap:12px;min-width:0;max-width:100%}.hero-team img{width:clamp(56px,5vw,94px);height:clamp(56px,5vw,94px);object-fit:contain;filter:drop-shadow(0 6px 14px #0008)}.hero-team span{min-width:0;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;font-size:clamp(16px,1.3vw,24px);font-weight:700}.hero-team strong{min-width:2ch;text-align:center;font-size:clamp(36px,3vw,54px);font-variant-numeric:tabular-nums}.hero-clock{flex-basis:100%;text-align:center;font-size:clamp(15px,1.1vw,20px);color:#d5d8e2}@keyframes sports-hero-swap{from{opacity:.6;transform:translateY(4px)}to{opacity:1;transform:none}}
  .sports-toolbar{display:flex;align-items:center;justify-content:space-between;gap:18px;margin:22px 0 26px}.sports-filters{position:relative;display:flex;flex-wrap:wrap;gap:8px}.filter-pill{position:absolute;left:0;top:0;width:var(--indicator-w,0);height:var(--indicator-h,0);border-radius:99px;background:var(--text);transform:translate(var(--indicator-x,0),var(--indicator-y,0));opacity:0;pointer-events:none}.sports-filters:global([data-indicator=ready]) .filter-pill{opacity:1;transition:transform var(--motion-base) var(--ease-out),width var(--motion-base) var(--ease-out),height var(--motion-base) var(--ease-out)}.sports-filters button,.toolbar-actions button{min-height:46px;padding:0 20px;border-radius:99px;border:1px solid var(--line-strong);background:var(--surface-2);color:var(--text-dim);font-size:clamp(14px,.95vw,18px);font-weight:600;white-space:nowrap}.sports-filters button{position:relative;z-index:1;display:inline-flex;align-items:center;gap:8px}.sports-filters img{width:24px;height:24px;object-fit:contain}.sports-filters button.active{background:transparent;color:var(--bg);border-color:transparent}.toolbar-actions{display:flex;gap:8px}.toolbar-actions button{padding:0 14px;font-size:14px;background:transparent}.section-heading{display:flex;justify-content:space-between;align-items:baseline;margin:0 0 14px}.section-heading h2{margin:0;font-size:clamp(23px,1.65vw,31px);font-weight:700;letter-spacing:-.025em}.game-section{margin-bottom:28px}.sports-grid{display:grid;grid-auto-flow:column;grid-auto-columns:calc((100% - 54px)/4);gap:18px;overflow-x:auto;scroll-snap-type:x proximity;padding:7px;margin:-7px;scrollbar-width:none}.sports-grid::-webkit-scrollbar{display:none}.section-heading h2>span{font-size:14px;font-weight:500;color:var(--muted);margin-left:8px}.game-tags{display:flex;align-items:center;gap:10px;min-height:24px;font-size:12px;font-weight:700;letter-spacing:.045em}.league{padding:4px 10px;border:1px solid #ffffff35;border-radius:99px;color:var(--text)}.live-badge{color:#ffb4be;background:#a12c4338;padding:4px 9px;border-radius:99px}.team-logo{display:grid;place-items:center;flex:none;width:42px;height:42px;font-weight:700}.team-logo img{width:100%;height:100%;object-fit:contain}.sports-state{padding:40px;border:1px solid var(--line);border-radius:18px;background:var(--surface-2)}.sports-state h2{font-size:28px;margin:0 0 12px}.sports-state p{color:var(--muted);margin:0 0 24px}.sports-feedback{color:var(--warn);font-size:15px;line-height:1.5}.scores-credit{font-size:12px;color:var(--dim);margin:14px 0 0}
  .sports-dialog{width:min(760px,calc(100vw - 48px));max-height:calc(100dvh - 48px);margin:auto;padding:32px;overflow-y:auto;border:1px solid var(--line-strong);border-radius:24px;background:var(--surface-2);color:var(--text);box-shadow:0 24px 90px #0008}.sports-dialog::backdrop{background:#05060acb;backdrop-filter:blur(8px)}.detail-top{display:flex;align-items:center;justify-content:space-between;gap:20px}.sports-dialog h2{margin:20px 0 10px;font-size:clamp(28px,2.5vw,42px);line-height:1.1;letter-spacing:-.035em}.detail-status{color:var(--muted);font-size:16px;line-height:1.5}.detail-teams{display:grid;gap:12px;margin:28px 0}.detail-team{display:flex;align-items:center;gap:16px;padding:16px;border:1px solid var(--line);border-radius:14px;background:#ffffff04}.detail-team .team-logo{width:56px;height:56px}.detail-team-name{flex:1;font-size:23px;font-weight:650}.detail-team-name small{display:block;color:var(--muted);font-size:14px;font-weight:500;margin-top:5px}.detail-score{font-size:36px;font-variant-numeric:tabular-nums}.favorite-button{width:48px;height:48px;border:1px solid var(--line-strong);border-radius:50%;background:var(--surface-3);color:var(--text-dim);font-size:26px}.favorite-button[aria-pressed=true]{color:#ffd28d}.detail-actions{display:flex;gap:12px}.detail-actions .btn{flex:1;min-height:58px;font-size:18px}.detail-hint{margin:20px 0 0;color:var(--muted);font-size:14px;line-height:1.5}
  @media(max-width:1050px){.sports-grid{grid-auto-columns:calc((100% - 18px)/2)}.sports-toolbar{align-items:flex-start;flex-direction:column}.hero-matchup{gap:12px;padding:20px 22px 20px 12px}.hero-team{gap:8px}.hero-team span{display:none}}@media(max-width:760px){.sports-hero{grid-template-columns:1fr;height:auto;gap:0}.hero-copy{max-width:100%;padding:26px}.hero-matchup{max-width:100%;padding:0 26px 26px;justify-content:flex-start}.hero-clock{text-align:left}.sports-grid{grid-auto-columns:85%}.detail-actions{flex-direction:column}.detail-team-name{font-size:18px}.hero-actions{flex-wrap:wrap}}
</style>
