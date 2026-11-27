<script lang="ts">
  import type { SportsEvent } from '../services/sports';
  import { scoreTeam, type GameScore } from '../services/sportsScores';
  import type { SportsBadge } from '../types';
  let { event, matched, badge, hideScores = true, favorite = false, time = event.time || 'Upcoming', live = event.live, disabled = false, onselect, onpreview }: {
    event: SportsEvent; matched?: GameScore; badge?: SportsBadge; hideScores?: boolean; favorite?: boolean; time?: string; live?: boolean; disabled?: boolean; onselect: (origin: HTMLButtonElement) => void; onpreview?: (event: SportsEvent | null) => void;
  } = $props();
  function logo(name: string) { return scoreTeam(matched, name)?.logo || event.teams.find(team => team.name === name)?.logo || ''; }
  function sideValue(name: string) {
    if (!badge?.sides?.length) return null;
    const norm = (v: string) => v.toLowerCase().replace(/\blos angeles\b/g, 'la').replace(/\bsaint\b/g, 'st').replace(/[^a-z0-9]/g, '');
    const key = norm(name);
    return badge.sides.find(s => { const k = norm(s.team); return k === key || k.includes(key) || key.includes(k); });
  }
</script>
                <button class="sports-card" data-focus-key={event.id} data-sports-event={event.id} style:--team-color={scoreTeam(matched, event.teams[0]?.name)?.color || '#39374f'} disabled={disabled} onclick={(click) => onselect(click.currentTarget)} aria-label={`${event.title}, ${event.league}, ${time}`} onmouseenter={() => onpreview?.(event)} onmouseleave={() => onpreview?.(null)} onfocus={() => onpreview?.(event)} onblur={() => onpreview?.(null)}>
                  {#if logo(event.teams[0]?.name)}<img class="card-watermark" src={logo(event.teams[0].name)} alt="" loading="lazy" />{/if}
                  <span class="game-tags"><span class="league">{event.league}</span>{#if live}<span class="live-badge">● LIVE</span>{/if}{#if favorite}<span class="favorite-mark" aria-label="Favorite team">★</span>{/if}</span>
                  <span class="card-matchup">
                    {#each event.teams as team (team.name)}
                      {@const info = scoreTeam(matched, team.name)}
                      {@const odd = sideValue(team.name)}
                      <span class="team-row">
                        <span class="team-logo">{#if logo(team.name)}<img src={logo(team.name)} alt="" loading="lazy" decoding="async" />{:else}{team.name.slice(0, 1)}{/if}</span>
                        <span class="team-name">{team.name}</span>
                        {#if odd}<span class="team-odds" title={badge?.label}>{odd.value}</span>{/if}
                        {#if matched && matched.state !== 'pre' && !hideScores}<strong class="team-score">{info?.score ?? '–'}</strong>{/if}
                      </span>
                    {/each}
                  </span>
                  {#if badge && badge.sides.length >= 2 && badge.sides[0]?.pct != null}
                    <span class="odds-bar" aria-label={`${badge.label}: ${badge.sides.map(s => `${s.team} ${s.value}`).join(' vs ')}`}>
                      <span class="odds-fill" style:width={`${badge.sides[0].pct}%`}></span>
                    </span>
                  {/if}
                  <span class="card-footer"><span class="watch-label">▶ {live ? 'Watch live' : 'Game details'}</span><span>{time}</span></span>
                </button>
<style>
.sports-card{scroll-snap-align:start}
.sports-card{position:relative;isolation:isolate;display:flex;flex-direction:column;gap:18px;min-height:200px;padding:20px;border:1px solid var(--line-strong);border-radius:17px;background:linear-gradient(115deg,var(--surface-2) 35%,color-mix(in srgb,var(--team-color) 35%,var(--surface-2)));color:var(--text, var(--ink));text-align:left;overflow:hidden;transition:transform var(--motion-fast),box-shadow var(--motion-fast)}
.sports-card:focus{outline:3px solid var(--accent);outline-offset:3px;box-shadow:0 0 25px var(--accent-soft);transform:translateY(-2px)}
.sports-card:hover{border-color:var(--accent)}
.sports-card:is(:hover,:focus) .card-watermark{transform:rotate(-5deg) scale(1.1)}
.card-watermark{position:absolute;right:-30px;bottom:-30px;width:190px;height:190px;object-fit:contain;opacity:.11;z-index:-1;transform:rotate(-12deg);transition:transform var(--motion-base) var(--ease-out)}
.game-tags{display:flex;align-items:center;gap:10px;min-height:24px;font-size:12px;font-weight:700;letter-spacing:.045em}
.league{padding:4px 10px;border:1px solid #ffffff35;border-radius:99px;color:var(--text, var(--ink))}
.live-badge{color:#ffb4be;background:#a12c4338;padding:4px 9px;border-radius:99px}
.favorite-mark{margin-left:auto;color:#ffd28d}
.card-matchup{display:grid;gap:10px;flex:1}
.team-row{display:flex;align-items:center;gap:12px}
.team-logo{display:grid;place-items:center;flex:none;width:42px;height:42px;font-weight:700}
.team-logo img{width:100%;height:100%;object-fit:contain}
.team-name{flex:1;min-width:0;font-size:clamp(17px,1.1vw,21px);font-weight:650;line-height:1.2}
.team-odds{flex:none;padding:2px 8px;border-radius:99px;background:#ffffff12;border:1px solid #ffffff1c;color:var(--text-dim,var(--muted));font-size:12px;font-weight:700;font-variant-numeric:tabular-nums}
.odds-bar{display:block;height:3px;border-radius:99px;background:#ffffff1a;overflow:hidden}
.odds-fill{display:block;height:100%;border-radius:99px;background:var(--accent)}
.team-score{font-size:27px;font-weight:750;font-variant-numeric:tabular-nums}
.card-footer{display:flex;align-items:center;justify-content:space-between;gap:10px;font-size:13px;color:var(--text-dim, var(--muted))}
.watch-label{padding:7px 12px;border-radius:99px;background:var(--accent);color:var(--accent-ink);font-weight:700}

 .sports-card { width: 100%; min-width: 0; }
  .sports-card:disabled { opacity: .6; }
  @media (max-width: 639px) { .sports-card { padding: 16px; } .team-name { font-size: 16px; } }
</style>
