<script lang="ts">
  import type { SportsEvent } from '../services/sports';
  let { items, onselect }: { items: SportsEvent[]; onselect: (item: SportsEvent) => void } = $props();
</script>
<section class="content-row" aria-label="Sports search results">
  <div class="row-heading"><div><h2>Sports</h2><p>Live and upcoming events</p></div></div>
  <div class="video-track">
    {#each items as item (item.id)}
      <button class="video-card sports-search-card" data-sports-event={item.id} onclick={() => onselect(item)}>
        <span class="match-art">
          <span class="match-status"><span>{item.league}</span><span class={item.live ? 'live' : ''}>{item.live ? 'LIVE' : item.time || 'Upcoming'}</span></span>
          <span class="match-teams">
            {#each item.teams.slice(0, 2) as team (team.name)}
              <span class="match-team">{#if team.logo}<img src={team.logo} alt="" loading="lazy" decoding="async" />{:else}<span class="team-initial">{team.name.slice(0, 1)}</span>{/if}<strong>{team.name}</strong></span>
            {/each}
          </span>
        </span>
        <span class="video-name">{item.title}</span>
        <span class="match-action">View event and sources <span aria-hidden="true">?</span></span>
      </button>
    {/each}
  </div>
</section>
<style>
  .sports-search-card { text-align: left; }
  .match-art { display: flex; flex-direction: column; justify-content: space-between; aspect-ratio: 16/9; padding: clamp(16px,1.4vw,26px); background: radial-gradient(ellipse at top, color-mix(in srgb,var(--accent) 20%,transparent),transparent 80%),var(--surface-2); border-bottom: 1px solid #ffffff14; }
  .match-status { display: flex; justify-content: space-between; gap: 10px; color: var(--dim); font-size: clamp(12px,.8vw,16px); font-weight: 700; }
  .live { color: var(--good, #8be0b0); }
  .match-teams { display: flex; justify-content: space-around; gap: 20px; padding-top: 16px; }
  .match-team { display: flex; flex: 1; min-width: 0; flex-direction: column; align-items: center; gap: 10px; text-align: center; font-size: clamp(13px,.9vw,18px); }
  .match-team img,.team-initial { width: clamp(40px,4vw,72px); height: clamp(40px,4vw,72px); object-fit: contain; }
  .team-initial { display: grid; place-items: center; border-radius: 50%; background: var(--surface-3); font-size: 28px; }
  .match-action { display: flex; justify-content: space-between; padding: 0 16px 16px; color: var(--dim); font-size: clamp(12px,.8vw,16px); }
</style>
