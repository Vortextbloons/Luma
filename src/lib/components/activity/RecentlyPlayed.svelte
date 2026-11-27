<script lang="ts">
  import type { ActivityItem } from '../../types';
  import { isRecentlyPlayed, sortRecent } from '../../services/activity';
  import ActivityCard from './ActivityCard.svelte';
  import EmptyState from '../EmptyState.svelte';

  let { items, trackingEnabled, onopen, onremove, onpreview }: {
    items: ActivityItem[];
    trackingEnabled: boolean;
    onopen: (item: ActivityItem) => void;
    onremove: (item: ActivityItem) => void;
    onpreview?: (item: ActivityItem | null) => void;
  } = $props();

  const played = $derived(sortRecent(items.filter(isRecentlyPlayed)).slice(0, 10));
</script>

<section id="recently-played" class="content-row activity-row" aria-label="Recently played">
  <div class="row-heading">
    <div><h2>Recently played</h2><p>Music and video activity observed on this PC</p></div>
  </div>
  {#if !trackingEnabled}
    <EmptyState title="Activity tracking is off" description="Luma is not recording new activity. Existing history is unchanged until you clear it." />
  {:else if played.length === 0}
    <EmptyState title="No recent playback" description="Play a track in Spotify or a video in Luma's YouTube window while Luma is open." />
  {:else}
    <div class="activity-track">
      {#each played as item (item.id)}
        <ActivityCard {item} {onopen} {onremove} {onpreview} />
      {/each}
    </div>
  {/if}
</section>
