<script module lang="ts">
  export interface SportsHeroTeam {
    name: string;
    abbreviation?: string;
    logo?: string;
    score?: string;
  }
</script>

<script lang="ts">
  import { badgeSideValue } from '../services/sportsScores';
  import type { SportsBadge } from '../types';

  let {
    heroKey,
    league,
    title,
    subtitle,
    artwork = '',
    isLive = false,
    badge,
    teams,
    showScores = false,
    clock,
    primaryLabel,
    onprimary,
    secondaryLabel,
    onsecondary,
    onartworkerror,
    focusKeyPrefix,
    framed = false,
    inset = false,
  }: {
    heroKey: string;
    league: string;
    title: string;
    subtitle: string;
    artwork?: string;
    isLive?: boolean;
    badge?: SportsBadge;
    teams: SportsHeroTeam[];
    showScores?: boolean;
    clock: string;
    primaryLabel: string;
    onprimary: (origin: HTMLElement) => void;
    secondaryLabel?: string;
    onsecondary?: () => void;
    onartworkerror?: () => void;
    focusKeyPrefix?: string;
    framed?: boolean;
    inset?: boolean;
  } = $props();
</script>

<section class="sports-hero" class:is-framed={framed} class:is-inset={inset} aria-label="Featured game">
  {#key artwork}
    {#if artwork}<img class="hero-art" src={artwork} alt="" onerror={() => onartworkerror?.()} />{/if}
  {/key}
  <div class="hero-wash"></div>
  <div class="hero-copy">
    {#key heroKey}
      <div class="hero-copy-inner">
        <div class="game-tags">
          <span class="league">{league}</span>
          <span class={{ 'live-badge': isLive }}>{isLive ? '● LIVE' : 'FEATURED MATCHUP'}</span>
          {#if badge}<span class="hero-odds-label">{badge.label}</span>{/if}
        </div>
        <h1>{title}</h1>
        <p>{subtitle}</p>
        <div class="hero-actions">
          <button
            data-focus-key={focusKeyPrefix ? `${focusKeyPrefix}:primary` : undefined}
            class="btn btn-primary"
            onclick={(event) => onprimary(event.currentTarget)}
          >{primaryLabel}</button>
          {#if secondaryLabel && onsecondary}
            <button
              data-focus-key={focusKeyPrefix ? `${focusKeyPrefix}:details` : undefined}
              class="btn btn-glass"
              onclick={onsecondary}
            >{secondaryLabel}</button>
          {/if}
        </div>
      </div>
    {/key}
  </div>
  {#key heroKey}
    <div class="hero-matchup" aria-label={showScores ? 'Featured scoreboard' : 'Featured teams'}>
      {#each teams as team (team.name)}
        {@const odd = badgeSideValue(badge, team.name)}
        <div class="hero-team">
          {#if team.logo}<img src={team.logo} alt="" />{/if}
          <span>{team.abbreviation || team.name}</span>
          {#if odd}<span class="hero-odds" title={badge?.label}>{odd.value}</span>{/if}
          {#if showScores}<strong>{team.score ?? '–'}</strong>{/if}
        </div>
      {/each}
      <span class="hero-clock">{clock}</span>
    </div>
  {/key}
</section>

<style>
  .sports-hero {
    position: relative;
    display: grid;
    grid-template-columns: minmax(0, 1.35fr) minmax(280px, .8fr);
    align-items: center;
    gap: clamp(16px, 2vw, 30px);
    height: clamp(330px, 23vw, 400px);
    overflow: hidden;
    border: 1px solid var(--line-strong);
    border-radius: 22px;
    background: linear-gradient(115deg, var(--surface-3), var(--bg));
    isolation: isolate;
  }
  .sports-hero.is-framed {
    margin: 0 var(--side) clamp(16px, 1.8vw, 24px);
  }
  .sports-hero.is-inset {
    margin: 0 0 clamp(16px, 1.8vw, 24px);
  }
  .hero-art {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    object-fit: cover;
    z-index: -3;
    opacity: .7;
    animation: sports-hero-art-in .22s var(--ease-out);
  }
  @keyframes sports-hero-art-in {
    from { opacity: 0; }
    to { opacity: .7; }
  }
  .hero-wash {
    position: absolute;
    inset: 0;
    z-index: -2;
    background: linear-gradient(90deg, #0b0d12f5 10%, #111320a8 55%, #0b0d1244), linear-gradient(0deg, #0b0d12b8, transparent 80%);
  }
  .hero-copy {
    min-width: 0;
    padding: clamp(26px, 2.7vw, 50px);
  }
  .hero-copy-inner {
    animation: sports-hero-swap .18s var(--ease-out);
  }
  .game-tags {
    display: flex;
    align-items: center;
    gap: 10px;
    min-height: 24px;
    font-size: 12px;
    font-weight: 700;
    letter-spacing: .045em;
  }
  .league {
    padding: 4px 10px;
    border: 1px solid #ffffff35;
    border-radius: 99px;
    color: var(--text);
  }
  .live-badge {
    color: #ffb4be;
    background: #a12c4338;
    padding: 4px 9px;
    border-radius: 99px;
  }
  .hero-copy h1 {
    max-width: 26ch;
    min-height: 2.1em;
    margin: 16px 0 12px;
    font-size: clamp(30px, 3.3vw, 58px);
    line-height: 1.05;
    font-weight: 750;
    letter-spacing: -.045em;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
    text-wrap: balance;
  }
  .hero-copy p {
    max-width: 100%;
    margin: 0;
    color: #d5d8e2;
    font-size: clamp(15px, 1.05vw, 20px);
    line-height: 1.4;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .hero-actions {
    display: flex;
    flex-wrap: wrap;
    gap: 12px;
    margin-top: 24px;
  }
  .hero-actions .btn {
    min-height: 52px;
    padding: 0 24px;
    font-size: clamp(16px, 1.1vw, 20px);
  }
  .hero-matchup {
    display: flex;
    flex-wrap: wrap;
    justify-content: center;
    align-items: center;
    gap: clamp(12px, 1.4vw, 20px) clamp(16px, 2vw, 28px);
    min-width: 0;
    padding: clamp(20px, 2.5vw, 34px) clamp(14px, 2.5vw, 36px);
    animation: sports-hero-swap .18s var(--ease-out);
  }
  .hero-team {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 12px;
    min-width: 0;
    max-width: 100%;
  }
  .hero-team img {
    width: clamp(56px, 5vw, 94px);
    height: clamp(56px, 5vw, 94px);
    object-fit: contain;
    filter: drop-shadow(0 6px 14px #0008);
  }
  .hero-team span {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: clamp(16px, 1.3vw, 24px);
    font-weight: 700;
  }
  .hero-team .hero-odds {
    flex: none;
    padding: 3px 10px;
    border-radius: 99px;
    background: #ffffff14;
    border: 1px solid #ffffff24;
    color: #e5eaf6;
    font-size: clamp(12px, .9vw, 15px);
    font-weight: 700;
    font-variant-numeric: tabular-nums;
  }
  .hero-odds-label {
    padding: 4px 10px;
    border-radius: 99px;
    background: #ffffff12;
    color: #d5d8e2;
    font-size: 11px;
    font-weight: 700;
    letter-spacing: .04em;
  }
  .hero-team strong {
    min-width: 2ch;
    text-align: center;
    font-size: clamp(36px, 3vw, 54px);
    font-variant-numeric: tabular-nums;
  }
  .hero-clock {
    flex-basis: 100%;
    text-align: center;
    font-size: clamp(15px, 1.1vw, 20px);
    color: #d5d8e2;
  }
  @keyframes sports-hero-swap {
    from { opacity: .6; transform: translateY(4px); }
    to { opacity: 1; transform: none; }
  }
  @media (max-width: 1050px) {
    .hero-matchup { gap: 12px; padding: 20px 22px 20px 12px; }
    .hero-team { gap: 8px; }
    .hero-team span { display: none; }
  }
  @media (max-width: 760px) {
    .sports-hero { grid-template-columns: 1fr; height: auto; gap: 0; }
    .hero-copy { max-width: 100%; padding: 26px; }
    .hero-matchup { max-width: 100%; padding: 0 26px 26px; justify-content: flex-start; }
    .hero-clock { text-align: left; }
    .hero-actions { flex-wrap: wrap; }
  }
</style>
