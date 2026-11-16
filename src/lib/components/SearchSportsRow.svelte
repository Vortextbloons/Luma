<script lang="ts">
  import type { SportsEvent } from '../services/sports';
  import SportsEventCard from './SportsEventCard.svelte';
  let { items, onselect }: { items: SportsEvent[]; onselect: (item: SportsEvent) => void } = $props();
</script>
<section class="content-row" aria-label="Sports search results">
  <div class="row-heading"><div><h2>Sports</h2><p>Live and upcoming events</p></div></div>
  <div class="sports-grid">
    {#each items as item (item.id)}
      <SportsEventCard event={item} onselect={() => onselect(item)} />
    {/each}
  </div>
</section>
<style>
  .sports-grid { display: grid; grid-auto-flow: column; grid-auto-columns: minmax(260px, calc((100% - 54px)/4)); gap: 18px; overflow-x: auto; padding: 7px; margin: -7px; }
</style>
