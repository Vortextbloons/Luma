<script lang="ts">
  import type { ActivityItem } from '../../types';
  import { isRecentlyPlayed, sortRecent } from '../../services/activity';
  import ActivityCard from './ActivityCard.svelte';

  let { items, trackingEnabled, onopen, onremove }: {
    items: ActivityItem[];
    trackingEnabled: boolean;
    onopen: (item: ActivityItem) => void;
    onremove: (item: ActivityItem) => void;
  } = $props();

  const played = $derived(sortRecent(items.filter(isRecentlyPlayed)).slice(0, 10));
</script>

<section id="recently-played" class="content-row activity-row" aria-label="Recently played">
  <div class="row-heading">
    <div><h2>Recently played</h2><p>Music and video activity observed on this PC</p></div>
  </div>
  {#if !trackingEnabled}
    <div class="activity-empty"><strong>Activity tracking is off</strong><span>Luma is not recording new activity. Existing history is unchanged until you clear it.</span></div>
  {:else if played.length === 0}
    <div class="activity-empty"><strong>No recent playback</strong><span>Play a track in Spotify or a video in Luma's YouTube window while Luma is open.</span></div>
  {:else}
    <div class="activity-track">
      {#each played as item (item.id)}
        <ActivityCard {item} {onopen} {onremove} />
      {/each}
    </div>
  {/if}
</section>
