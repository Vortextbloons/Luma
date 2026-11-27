<script lang="ts">
  import SportsEventCard from './SportsEventCard.svelte';
  import SportsHero from './SportsHero.svelte';
  import PageHeader from './PageHeader.svelte';
  import EmptyState from './EmptyState.svelte';
  import { onMount, tick, untrack } from 'svelte';
  import { SvelteMap } from 'svelte/reactivity';
  import { loadSportsTeams, isSportsFavorite, majorSportsLeagues, readSportsFavorites, sportsScoresHidden, toggleSportsFavorite, writeSportsFavorites, type SportsTeam, type SportsEvent, type SportsFavorite } from '../services/sports';
  import { badgeSideValue, matchSportsBadge, scoreTeam } from '../services/sportsScores';
  import { appVisible } from '../services/visibility';
  import type { SportsState } from '../state/sports.svelte';
  import nbaIcon from '../../../src-tauri/resources/icons/sports-nba.png';
  import nflIcon from '../../../src-tauri/resources/icons/sports-nfl.png';
  import mlbIcon from '../../../src-tauri/resources/icons/sports-mlb.png';
  import nhlIcon from '../../../src-tauri/resources/icons/sports-nhl.png';
  import wnbaIcon from '../../../src-tauri/resources/icons/sports-wnba.png';
  import cfbIcon from '../../../src-tauri/resources/icons/sports-cfb.png';
  import ncaaMenIcon from '../../../src-tauri/resources/icons/sports-ncaa-men.png';
  import ncaaWomenIcon from '../../../src-tauri/resources/icons/sports-ncaa-women.png';

  let { native, sports, ondetailclose }: { native: boolean; sports: SportsState; ondetailclose?: () => void } = $props();
  const listing = $derived(sports.listing);
  const scores = $derived(sports.scores);
  const loading = $derived(sports.loading);
  const error = $derived(sports.error);
  const scoreError = $derived(sports.scoreError);
  const scoreUpdated = $derived(sports.updatedAt);
  const opening = $derived(sports.opening);
  let filter = $state('All');
  let leaguesOpen = $state(false);
  let managingTeams = $state(false);
  let teamLeague = $state("NBA");
  let teamOptions = $state.raw<SportsTeam[]>([]);
  const teamLogos = new SvelteMap<string, string>();
  let teamsLoading = $state(false);
  let teamsError = $state('');
  let teamRequest = 0;
  let selected = $state<SportsEvent | null>(null);
  let hovered = $state.raw<SportsEvent | null>(null);
  let detailError = $state('');
  let favorites = $state<SportsFavorite[]>(readSportsFavorites());
  let hideScores = $state(sportsScoresHidden());
  const badges = $derived(sports.badges);
  let sourceIndexes: Record<string, number> = {};
  let returnFocus: HTMLElement | null = null;
  let active = false;
  const events = $derived.by(() => {
    const listed = sports.events;
    const now = Date.now();
    const horizon = now + 7 * 24 * 60 * 60 * 1000;
    const scheduled: SportsEvent[] = scores
      .filter(score => {
        if (score.state !== 'pre' || score.teams.length !== 2) return false;
        const when = Date.parse(score.date);
        if (!Number.isNaN(when) && (when < now - 3_600_000 || when > horizon)) return false;
        return !sports.streamFor(score);
      })
      .map(score => ({
        id: `espn:${score.league}:${score.id}`,
        title: `${score.teams[0].name} vs ${score.teams[1].name}`,
        league: score.league,
        time: score.status,
        live: false,
        url: '',
        sources: [],
        teams: score.teams.map(team => ({ name: team.name, logo: team.logo })),
      }));
    return [...listed, ...scheduled];
  });
  const extraLeagues = $derived([...new Set(events.map(event => event.league).filter(league => !majorSportsLeagues.includes(league)))].sort((a, b) => a.localeCompare(b)));
  const filters = $derived(['All', 'Your teams', ...majorSportsLeagues, ...extraLeagues]);
  const leagueIcons: Record<string, string> = { NBA: nbaIcon, NFL: nflIcon, MLB: mlbIcon, NHL: nhlIcon, WNBA: wnbaIcon, CFB: cfbIcon, 'NCAA Men': ncaaMenIcon, 'NCAA Women': ncaaWomenIcon };
  const visible = $derived(events.filter(event => filter === 'All' || (filter === 'Your teams' ? eventFavorite(event) : event.league === filter)));
  const liveEvents = $derived(visible.filter(event => live(event)));
  const upcoming = $derived.by(() => {
    const schedule: { event: SportsEvent; start: number }[] = [];
    for (const event of visible) {
      const matched = game(event);
      if (matched ? matched.state === 'in' || matched.state === 'post' : event.live) continue;
      schedule.push({ event, start: Date.parse(matched?.date ?? '') || Number.MAX_SAFE_INTEGER });
    }
    return schedule.sort((a, b) => a.start - b.start).map(item => item.event);
  });
  const finished = $derived(visible.filter(event => game(event)?.state === 'post'));
  const featured = $derived.by(() => {
    const preview = hovered;
    if (preview && visible.some(event => event.id === preview.id)) return preview;
    return liveEvents.find(event => eventFavorite(event)) ?? liveEvents.find(event => game(event)) ?? liveEvents[0] ?? upcoming.find(event => game(event)) ?? upcoming[0];
  });
  const featuredGame = $derived(featured ? game(featured) : undefined);
  const featuredBadge = $derived(featured ? matchSportsBadge(featured, badges) : undefined);
  const selectedGame = $derived(selected ? game(selected) : undefined);
  const selectedBadge = $derived(selected ? matchSportsBadge(selected, badges) : undefined);
  const artwork = $derived(featuredGame ? sports.artworkFor(featuredGame.id) : '');
  const sections = $derived([{ title: 'Live Now', events: liveEvents }, { title: 'Upcoming', events: upcoming }, { title: 'Final scores', events: finished }].filter(section => section.events.length));

  function game(event: SportsEvent) { return sports.scoreFor(event); }
  function live(event: SportsEvent) { const matched = game(event); return matched ? matched.state === 'in' : event.live; }
  function eventFavorite(event: SportsEvent) { return isSportsFavorite(favorites, event.league, '*') || event.teams.some(team => isSportsFavorite(favorites, event.league, team.name)); }
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
    hovered = null;
    await tick();
    document.querySelector<HTMLElement>('.sports-filters button[aria-pressed="true"]')?.focus({ preventScroll: true });
  }
  function selectEvent(event: SportsEvent, origin: HTMLElement) { returnFocus = origin; selected = event; detailError = ''; }
  export function closeDetail() {
    if (leaguesOpen) {
      leaguesOpen = false;
      document.getElementById('extra-leagues-toggle')?.focus();
      return true;
    }
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
      await sports.refreshListing();
      if (!active || selected?.id !== event.id) return;
      const score = game(event);
      const matched = score ? sports.streamFor(score) : undefined;
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
  function setFilter(name: string) { if (filter === name) return; filter = name; hovered = null; }
  function previewEvent(event: SportsEvent | null) { hovered = event; }
  export function toggleScores() {
    hideScores = !hideScores;
    try { localStorage.setItem('luma-sports-hide-scores', String(hideScores)); } catch {}
    if (sports.watching) sports.refreshOverlay();
  }
  function mountDialog(dialog: HTMLDialogElement) { dialog.showModal(); requestAnimationFrame(() => dialog.querySelector<HTMLElement>('[data-sports-action="watch"]:not(:disabled), .close-btn')?.focus({ preventScroll: true })); }
  onMount(() => {
    active = true;
    let timer: ReturnType<typeof setTimeout>;
    const schedule = () => { timer = setTimeout(() => {
      if (appVisible() || sports.watching) void refreshScores().finally(() => { if (active) schedule(); });
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
    <SportsHero
      heroKey={featured.id}
      league={featured.league}
      title={featured.title}
      subtitle={`${featuredGame?.venue || 'Your front row to the game.'}${featuredGame?.venue ? ` · ${gameTime(featured)}` : ''}`}
      {artwork}
      isLive={live(featured)}
      badge={featuredBadge}
      teams={featured.teams.map(team => {
        const info = scoreTeam(featuredGame, team.name);
        return { name: team.name, abbreviation: info?.abbreviation || team.name, logo: teamLogo(featured, team.name), score: info?.score ?? '–' };
      })}
      showScores={!hideScores && !!featuredGame && featuredGame.state !== 'pre'}
      clock={gameTime(featured)}
      primaryLabel={featuredGame?.state === 'post' && !hideScores ? 'Final score' : 'Game details'}
      onprimary={(origin) => selectEvent(featured, origin)}
      onartworkerror={() => { if (featuredGame) sports.clearArtwork(featuredGame.id); }}
    />
  {:else}
    <PageHeader flush eyebrow={filter === 'All' || filter === 'Your teams' ? 'GAME DAY, TOGETHER' : filter}
      title={filter === 'Your teams' ? 'Your teams play here' : filter === 'All' || error || scoreError ? filter === 'All' ? 'Sports' : filter : `No ${filter} games right now`}
      description={error || scoreError ? 'Browse leagues or retry to refresh games.' : filter === 'Your teams' ? (favorites.length ? 'None of your teams have a live or upcoming game right now.' : 'Choose your teams to find their matchups here.') : filter === 'All' ? 'Your teams. Every matchup. One place.' : 'There are no live or upcoming games listed for this league.'} />
  {/if}

  <div class="sports-toolbar">
    <nav class="sports-filters" aria-label="Filter sports">
      {#each filters.filter(name => !extraLeagues.includes(name)) as name (name)}<button class={{ active: filter === name }} aria-pressed={filter === name} onclick={() => setFilter(name)}>{#if leagueIcons[name]}<img src={leagueIcons[name]} alt="" />{/if}{name === 'Your teams' ? '☆ Your teams' : name}</button>{/each}
      {#if extraLeagues.length}<button id="extra-leagues-toggle" class={{ active: extraLeagues.includes(filter) }} aria-expanded={leaguesOpen} aria-controls="extra-leagues" onclick={() => { leaguesOpen = !leaguesOpen; if (leaguesOpen) void tick().then(() => document.querySelector<HTMLElement>('#extra-leagues .sports-filters button')?.focus()); }}><span aria-hidden="true">&#183;&#183;&#183;</span> {extraLeagues.includes(filter) ? filter : 'More leagues'}</button>{/if}
    </nav>
    <div class="toolbar-actions">
      {#if native}<button onclick={(event) => manageTeams(event.currentTarget)}>Manage teams</button>{/if}
      <button aria-pressed={hideScores} onclick={toggleScores}>{hideScores ? 'Show scores' : 'Hide scores'}</button>
      {#if native}<button disabled={loading} onclick={() => void load(true)}>{loading ? 'Refreshing…' : 'Refresh'}</button>{/if}
    </div>
  </div>
  {#if leaguesOpen}
    <section id="extra-leagues" class="extra-leagues" aria-label="More sports leagues">
      <header><h2>More leagues</h2><button aria-label="Close league choices" onclick={() => { leaguesOpen = false; document.getElementById('extra-leagues-toggle')?.focus(); }}>Close</button></header>
      <div class="sports-filters">
        {#each extraLeagues as name (name)}
          <button class={{ active: filter === name }} aria-pressed={filter === name} onclick={() => { setFilter(name); leaguesOpen = false; void tick().then(() => document.getElementById('extra-leagues-toggle')?.focus()); }}>{name}<span class="league-count">{events.filter(event => event.league === name).length}</span></button>
        {/each}
      </div>
    </section>
  {/if}
  {#if error || scoreError}<p class="sports-feedback" role="status">{error || scoreError}</p>{/if}
  {#if !native}<EmptyState variant="panel" title="Open Luma on your PC" description="Live sports are available in the desktop app." />
  {:else if !listing && loading}<EmptyState variant="panel" role="status" title="Getting ready for game time" description="Loading live and upcoming matchups…" />
  {:else if !listing || (!visible.length && (error || scoreError))}<EmptyState variant="panel" title="Games could not load" description={error || scoreError}>
    {#snippet actions()}<button class="btn btn-glass" onclick={() => { void load(true); void refreshScores(true); }}>Retry</button>{/snippet}
  </EmptyState>
  {:else if !visible.length}<EmptyState variant="panel" title={filter === 'Your teams' ? 'Your teams play here' : `No ${filter === 'All' ? '' : filter + ' '}games listed right now`} description={filter === 'Your teams' ? favorites.length ? 'None of your teams have a listed game right now.' : 'Choose your teams to find their matchups here.' : 'Choose another league or refresh for the latest games.'}>
    {#snippet actions()}
      <button class="btn btn-glass" onclick={() => setFilter('All')}>Show all games</button>
      {#if filter === 'Your teams'} <button class="btn btn-glass" onclick={(event) => manageTeams(event.currentTarget)}>Manage teams</button>{/if}
    {/snippet}
  </EmptyState>
  {:else}
    {#key filter}
      <div class="sports-results">
        {#each sections as section (section.title)}
          <section class="game-section" aria-label={section.title}>
            <header class="section-heading"><h2>{section.title} <span>{section.events.length}</span></h2></header>
            <div class="sports-grid">
              {#each section.events as event (event.id)}
                {@const matched = game(event)}
                <SportsEventCard {event} {matched} badge={matchSportsBadge(event, badges)} {hideScores} favorite={eventFavorite(event)} time={gameTime(event)} live={live(event)} onselect={origin => selectEvent(event, origin)} onpreview={previewEvent} />
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
        {@const odd = badgeSideValue(selectedBadge, team.name)}
        <div class="detail-team"><span class="team-logo">{#if teamLogo(selected, team.name)}<img src={teamLogo(selected, team.name)} alt="" />{:else}{team.name.slice(0, 1)}{/if}</span><span class="detail-team-name">{team.name}{#if info?.record}<small>{info.record}</small>{/if}</span>{#if odd}<span class="detail-odds" title={selectedBadge?.label}>{odd.value}</span>{/if}{#if selectedGame && selectedGame.state !== 'pre' && !hideScores}<strong class="detail-score">{info?.score ?? '–'}</strong>{/if}<button class="favorite-button" aria-pressed={isSportsFavorite(favorites, selected.league, team.name)} aria-label={`Favorite ${team.name}`} onclick={() => favorite(selected!.league, team.name)}>{isSportsFavorite(favorites, selected.league, team.name) ? '★' : '☆'}</button></div>
      {/each}
    </div>
    {#if selectedBadge && selectedBadge.sides.length >= 2 && selectedBadge.sides[0]?.pct != null}
      <div class="detail-odds-summary">
        <span>{selectedBadge.label}</span>
        <span class="detail-odds-bar"><span class="detail-odds-fill" style:width={`${selectedBadge.sides[0].pct}%`}></span></span>
      </div>
    {/if}
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
    {#if favorites.length}<h3>Following</h3><div class="team-options">{#each favorites as item (item.league + item.team)}<button class="btn btn-glass" aria-label={`Unfollow ${item.team === '*' ? item.league : item.team}`} onclick={() => favorite(item.league, item.team)}><span class="team-logo">{#if teamLogos.get(item.league + ':' + item.team)}<img src={teamLogos.get(item.league + ':' + item.team)} alt="" loading="lazy" decoding="async" />{:else}<img src={leagueIcons[item.league]} alt="" />{/if}</span><span class="picker-team-name">{item.team === '*' ? 'All teams' : item.team}<small>{item.league}</small></span><span class="picker-followed" aria-hidden="true">&#9733;</span></button>{/each}</div>{/if}
    <h3>{teamLeague} teams</h3>
    <button class="btn btn-glass" aria-pressed={isSportsFavorite(favorites, teamLeague, '*')} onclick={() => favorite(teamLeague, '*')}>Follow all {teamLeague} teams</button>
    {#if teamsLoading}<p role="status">Loading teams...</p>
    {:else if teamsError}<p class="sports-feedback" role="status">{teamsError}</p><button class="btn btn-glass" onclick={() => void chooseTeamLeague(teamLeague)}>Try again</button>
    {:else if !teamOptions.length}<p>No teams are available for this league.</p>
    {:else}<div class="team-options">{#each teamOptions as team (team.name)}<button class="btn btn-glass" aria-pressed={isSportsFavorite(favorites, teamLeague, team.name)} onclick={() => favorite(teamLeague, team.name)}><span class="team-logo">{#if team.logo}<img src={team.logo} alt="" loading="lazy" decoding="async" />{:else}{team.name.slice(0, 1)}{/if}</span><span class="picker-team-name">{team.name}</span><span class="picker-followed" aria-hidden="true">{isSportsFavorite(favorites, teamLeague, team.name) ? '\u2605' : '\u2606'}</span></button>{/each}</div>{/if}
  </dialog>
{/if}

<style>
  .picker-league-logo{width:28px;height:28px;object-fit:contain}.team-options .btn{display:flex;align-items:center;gap:14px;padding:14px 18px}.picker-team-name{flex:1;min-width:0;font-size:16px;line-height:1.3}.picker-team-name small{display:block;margin-top:4px;color:var(--muted);font-size:13px}.picker-followed{font-size:24px;color:var(--muted)}[aria-pressed=true] .picker-followed,.team-options button[aria-label] .picker-followed{color:#ffd28d}

  .team-leagues{display:flex;flex-wrap:wrap;gap:12px}.team-options{display:grid;grid-template-columns:repeat(2,minmax(0,1fr));gap:12px}.team-options .btn{min-height:54px;white-space:normal;text-align:left;justify-content:flex-start}.team-leagues [aria-pressed=true],.team-options [aria-pressed=true]{border-color:var(--accent);background:var(--accent-soft)}button:focus-visible{outline:3px solid var(--accent);outline-offset:3px}@media(max-width:760px){.team-options{grid-template-columns:1fr}}

  .sports-home{padding:0 var(--side) 40px}.sports-results{animation:sports-results-in var(--motion-base) var(--ease-out)}@keyframes sports-results-in{from{opacity:0;transform:translateY(6px)}to{opacity:1;transform:none}}
  .sports-toolbar { display: flex; align-items: center; justify-content: space-between; flex-wrap: wrap; gap: 14px clamp(18px, 2vw, 36px); margin: clamp(18px, 1.4vw, 28px) 0 clamp(18px, 1.5vw, 30px); }
  .sports-filters, .toolbar-actions { display: flex; flex-wrap: wrap; gap: clamp(8px, .6vw, 14px); }
  .sports-filters { position: relative; }
  .sports-filters button, .toolbar-actions button {
    min-height: clamp(48px, 3vw, 64px);
    padding: 0 clamp(14px, 1.2vw, 28px);
    border-radius: 99px;
    border: 1px solid var(--line-strong);
    background: var(--surface-2);
    color: var(--text-dim);
    font-size: clamp(15px, 1.03vw, 21px);
    font-weight: 600;
    white-space: nowrap;
  }
  .sports-filters button { position: relative; z-index: 1; display: inline-flex; align-items: center; gap: clamp(7px, .55vw, 12px); }
  .sports-filters img { width: clamp(20px, 1.45vw, 30px); height: clamp(20px, 1.45vw, 30px); object-fit: contain; }
  .sports-filters button.active, .toolbar-actions button[aria-pressed=true] { background: var(--accent-soft); color: var(--text); border-color: var(--accent); }
  .sports-filters button.active { box-shadow: inset 0 -3px 0 var(--accent); }
  .toolbar-actions button { padding-inline: clamp(12px, 1vw, 24px); font-size: clamp(14px, .92vw, 19px); background: transparent; }
  .section-heading { display: flex; justify-content: space-between; align-items: baseline; margin: 0 0 clamp(12px, 1vw, 20px); }
  .section-heading h2 { margin: 0; font-size: clamp(25px, 1.8vw, 38px); font-weight: 700; letter-spacing: -.025em; }
  .game-section{margin-bottom:28px}.sports-grid{display:grid;grid-auto-flow:column;grid-auto-columns:calc((100% - 54px)/4);gap:18px;overflow-x:auto;scroll-snap-type:x proximity;padding:7px;margin:-7px;scrollbar-width:none}.sports-grid::-webkit-scrollbar{display:none}.section-heading h2>span{font-size:clamp(14px,.86vw,18px);font-weight:500;color:var(--muted);margin-left:10px}.game-tags{display:flex;align-items:center;gap:10px;min-height:24px;font-size:12px;font-weight:700;letter-spacing:.045em}.league{padding:4px 10px;border:1px solid #ffffff35;border-radius:99px;color:var(--text)}.live-badge{color:#ffb4be;background:#a12c4338;padding:4px 9px;border-radius:99px}.team-logo{display:grid;place-items:center;flex:none;width:42px;height:42px;font-weight:700}.team-logo img{width:100%;height:100%;object-fit:contain}.sports-feedback{color:var(--warn);font-size:clamp(14px,.86vw,18px);line-height:1.5}.sports-home>.sports-feedback{margin:0 0 12px}.scores-credit{font-size:12px;color:var(--dim);margin:14px 0 0}
  .sports-dialog{width:min(760px,calc(100vw - 48px));max-height:calc(100dvh - 48px);margin:auto;padding:32px;overflow-y:auto;border:1px solid var(--line-strong);border-radius:24px;background:var(--surface-2);color:var(--text);box-shadow:0 24px 90px #0008}.sports-dialog::backdrop{background:#05060acb;backdrop-filter:blur(8px)}.detail-top{display:flex;align-items:center;justify-content:space-between;gap:20px}.sports-dialog h2{margin:20px 0 10px;font-size:clamp(28px,2.5vw,42px);line-height:1.1;letter-spacing:-.035em}.detail-status{color:var(--muted);font-size:16px;line-height:1.5}.detail-teams{display:grid;gap:12px;margin:28px 0}.detail-team{display:flex;align-items:center;gap:16px;padding:16px;border:1px solid var(--line);border-radius:14px;background:#ffffff04}.detail-team .team-logo{width:56px;height:56px}.detail-team-name{flex:1;font-size:23px;font-weight:650}.detail-team-name small{display:block;color:var(--muted);font-size:14px;font-weight:500;margin-top:5px}.detail-score{font-size:36px;font-variant-numeric:tabular-nums}.detail-odds{flex:none;padding:4px 11px;border-radius:99px;background:#ffffff12;border:1px solid #ffffff20;color:var(--text-dim);font-size:14px;font-weight:700;font-variant-numeric:tabular-nums}.detail-odds-summary{display:grid;gap:8px;margin:-12px 0 22px;color:var(--muted);font-size:13px;font-weight:600}.detail-odds-bar{display:block;height:4px;border-radius:99px;background:#ffffff1a;overflow:hidden}.detail-odds-fill{display:block;height:100%;border-radius:99px;background:var(--accent)}.favorite-button{width:48px;height:48px;border:1px solid var(--line-strong);border-radius:50%;background:var(--surface-3);color:var(--text-dim);font-size:26px}.favorite-button[aria-pressed=true]{color:#ffd28d}.detail-actions{display:flex;gap:12px}.detail-actions .btn{flex:1;min-height:58px;font-size:18px}.detail-hint{margin:20px 0 0;color:var(--muted);font-size:14px;line-height:1.5}
  @media(max-width:1050px){.sports-grid{grid-auto-columns:calc((100% - 18px)/2)}.sports-toolbar{align-items:flex-start;flex-direction:column}}@media(max-width:760px){.sports-grid{grid-auto-columns:85%}.detail-actions{flex-direction:column}.detail-team-name{font-size:18px}}
  .sports-filters button:hover { background: var(--accent-soft); color: var(--text); border-color: var(--accent); }
  .sports-filters button:focus-visible { background: var(--accent-soft); color: var(--text); outline: 3px solid var(--accent); outline-offset: 4px; }
  .league-count { font-size: 12px; opacity: .7; margin-left: 6px; }
  .extra-leagues { margin: -10px 0 26px; padding: 20px; border: 1px solid var(--line-strong); border-radius: 18px; background: var(--surface-2); }
  .extra-leagues header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 16px; }
  .extra-leagues h2 { margin: 0; font-size: 20px; }
  .extra-leagues header button { min-height: 44px; padding: 0 16px; border-radius: 99px; border: 1px solid var(--line-strong); }
  @media (prefers-reduced-motion: no-preference) { .sports-filters button { transition: background .16s, border-color .16s, box-shadow .16s; } }
</style>
