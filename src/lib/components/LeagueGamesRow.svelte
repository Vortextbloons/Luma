<script lang="ts">
  import { badgeSideValue, matchSportsBadge, type GameScore } from '../services/sportsScores';
  import type { SportsBadge } from '../types';
  import EmptyState from './EmptyState.svelte';

  let { league, games, badges = [], loading = false, error = '', retry, watch, onpreview }: {
    league: string;
    games: GameScore[];
    badges?: SportsBadge[];
    loading?: boolean;
    error?: string;
    retry?: () => void;
    watch: (game: GameScore) => void;
    onpreview?: (game: GameScore | null) => void;
  } = $props();

  const live = $derived(games.filter(game => game.state === 'in').length);
  const heading = $derived(live ? `Live on ${league}` : `${league} games`);
  const summary = $derived(live ? `${live} game${live === 1 ? '' : 's'} in progress` : 'Live and upcoming matchups');
  function status(game: GameScore) {
    if (game.status) return game.status;
    const date = new Date(game.date);
    return Number.isNaN(date.getTime()) ? game.league : date.toLocaleString([], { weekday: 'short', hour: 'numeric', minute: '2-digit' });
  }
  function label(game: GameScore) {
    return `${game.teams.map(team => team.name).join(' vs ')}, ${game.state === 'in' ? 'live now' : status(game)}. Watch ${game.league} game.`;
  }
</script>

<section id="continue-watching" class="content-row activity-row" aria-label={heading}>
  <div class="row-heading">
    <div><h2>{heading}</h2><p>{summary}</p></div>
  </div>
  <div class="activity-track is-cinematic">
    {#if games.length}
      {#each games as game (game.id)}
        {@const badge = matchSportsBadge(game, badges)}
        <button data-focus-key={`league:${league}:${game.id}`} class="league-game" style:--team-color={game.teams[0]?.color || '#39374f'} onclick={() => watch(game)} aria-label={label(game)} onmouseenter={() => onpreview?.(game)} onmouseleave={() => onpreview?.(null)} onfocus={() => onpreview?.(game)} onblur={() => onpreview?.(null)}>
          <span class="league-game-art">
            {#each game.teams as team (team.name)}
              {@const odd = badgeSideValue(badge, team.name)}
              <span class="league-team">
                <span class="league-team-logo">
                  {#if team.logo}<img src={team.logo} alt="" loading="lazy" decoding="async" />{:else}{team.name.slice(0, 1)}{/if}
                </span>
                <span class="league-team-name">{team.name}</span>
                {#if odd}<span class="league-team-odds" title={badge?.label}>{odd.value}</span>{/if}
                {#if game.state !== 'pre'}<strong class="league-team-score">{team.score}</strong>{/if}
              </span>
            {/each}
            {#if badge && badge.sides.length >= 2 && badge.sides[0]?.pct != null}
              <span class="league-odds-bar" aria-label={`${badge.label}: ${badge.sides.map(s => `${s.team} ${s.value}`).join(' vs ')}`}>
                <span class="league-odds-fill" style:width={`${badge.sides[0].pct}%`}></span>
              </span>
            {/if}
            <span class="league-game-foot">
              {#if game.state === 'in'}<span class="live-badge">● LIVE</span>{/if}
              <span class="league-game-status">{status(game)}</span>
            </span>
          </span>
        </button>
      {/each}
    {:else if loading}
      <EmptyState title={`Looking up ${league} games`} description="Scores refresh in a moment." />
    {:else if error}
      <EmptyState title={`${league} games could not load`} description={error}>
        {#snippet actions()}<button class="btn btn-glass" onclick={retry}>Retry</button>{/snippet}
      </EmptyState>
    {:else}
      <EmptyState title={`No ${league} games listed right now`} description="Open Sports to browse every league and what is on." />
    {/if}
  </div>
</section>

<style>
  .activity-track > :global(.empty-state) { min-width: 0; }
  .league-game { min-width: 0; display: block; padding: 0; text-align: left; background: transparent; border-radius: calc(var(--card-radius) + 2px); transition: transform .18s; }
  .league-game:hover { transform: translateY(-2px); }
  .league-game:focus-visible { outline: 3px solid var(--accent); outline-offset: 4px; box-shadow: var(--focus-glow); transform: translateY(-3px); }
  .league-game:focus-visible .league-game-art { border-color: color-mix(in srgb, var(--accent) 55%, transparent); }
  .league-game:active { transform: translateY(-1px) scale(.995); transition-duration: var(--motion-press); }
  .league-game-art { display: flex; flex-direction: column; justify-content: center; gap: 9px; aspect-ratio: 16 / 9; padding: 14px 16px; overflow: hidden; border: 1px solid #ffffff14; border-radius: calc(var(--card-radius) + 2px); background: linear-gradient(115deg, var(--surface-2) 35%, color-mix(in srgb, var(--team-color) 35%, var(--surface-2))); box-shadow: 0 10px 26px #0004; transition: border-color .18s, box-shadow .18s; }
  .league-game:hover .league-game-art, .league-game:focus-visible .league-game-art { border-color: #ffffff33; box-shadow: 0 16px 36px #0007; }
  .league-game-foot { display: flex; align-items: center; gap: 8px; font-size: clamp(10px, .68vw, 12px); font-weight: 600; color: #b9c2d3; }
  .live-badge { padding: 3px 8px; border-radius: 99px; color: #ffb4be; background: #a12c4338; letter-spacing: .045em; }
  .league-game-status { min-width: 0; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .league-team { display: flex; align-items: center; gap: 9px; min-width: 0; }
  .league-team-logo { display: grid; place-items: center; flex: none; width: clamp(20px, 1.5vw, 26px); height: clamp(20px, 1.5vw, 26px); font-weight: 700; font-size: 13px; }
  .league-team-logo img { width: 100%; height: 100%; object-fit: contain; }
  .league-team-name { flex: 1; min-width: 0; font-size: clamp(13px, .86vw, 16px); font-weight: 650; line-height: 1.2; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .league-team-odds { flex: none; padding: 2px 7px; border-radius: 99px; background: #ffffff14; border: 1px solid #ffffff20; color: #dbe2f0; font-size: 11px; font-weight: 700; font-variant-numeric: tabular-nums; }
  .league-odds-bar { display: block; height: 2px; border-radius: 99px; background: #ffffff1c; overflow: hidden; }
  .league-odds-fill { display: block; height: 100%; border-radius: 99px; background: var(--accent); }
  .league-team-score { flex: none; font-size: clamp(15px, 1vw, 19px); font-variant-numeric: tabular-nums; }
</style>
