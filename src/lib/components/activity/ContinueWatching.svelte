<script lang="ts">
  import type { ActivityItem } from '../../types';
  import { isContinueWatching, sortRecent } from '../../services/activity';
  import ActivityCard from './ActivityCard.svelte';

  let { items, trackingEnabled, onopen, onremove }: {
    items: ActivityItem[];
    trackingEnabled: boolean;
    onopen: (item: ActivityItem) => void;
    onremove: (item: ActivityItem) => void;
  } = $props();

  const watching = $derived(sortRecent(items.filter(isContinueWatching)).slice(0, 10));
</script>

<section id="continue-watching" class="content-row activity-row" aria-label="Continue watching">
  <div class="row-heading">
    <div><h2>Continue watching</h2><p>Only verified playback positions from your providers</p></div>
  </div>
  {#if !trackingEnabled}
    <div class="activity-empty"><strong>Activity tracking is off</strong><span>Turn on local activity tracking in Settings to build this row.</span></div>
  {:else if watching.length === 0}
    <div class="activity-empty"><strong>Nothing in progress</strong><span>Connect Plex, Jellyfin, or Trakt, or play a video in Luma's YouTube window.</span></div>
  {:else}
    <div class="activity-track">
      {#each watching as item (item.id)}
        <ActivityCard {item} {onopen} {onremove} />
      {/each}
    </div>
  {/if}
</section>
