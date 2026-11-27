<script lang="ts">
  import type { SportsEvent } from '../services/sports';
  import { matchSportsBadge } from '../services/sportsScores';
  import type { SportsBadge } from '../types';
  import SportsEventCard from './SportsEventCard.svelte';
  let { items, badges = [], onselect }: { items: SportsEvent[]; badges?: SportsBadge[]; onselect: (item: SportsEvent) => void } = $props();
</script>
<section class="content-row" aria-label="Sports search results">
  <div class="row-heading"><div><h2>Sports</h2><p>Live and upcoming events</p></div></div>
  <div class="activity-track is-cinematic">
    {#each items as item (item.id)}
      <SportsEventCard event={item} badge={matchSportsBadge(item, badges)} onselect={() => onselect(item)} />
    {/each}
  </div>
</section>
