<script lang="ts">
  import type { ActivityItem } from '../../types';
  import { isApplication, sortRecent } from '../../services/activity';
  import ActivityCard from './ActivityCard.svelte';

  let { items, trackingEnabled, onopen, onremove }: {
    items: ActivityItem[];
    trackingEnabled: boolean;
    onopen: (item: ActivityItem) => void;
    onremove: (item: ActivityItem) => void;
  } = $props();

  const apps = $derived(sortRecent(items.filter(isApplication)).slice(0, 8));
</script>

<section id="recent-apps" class="content-row activity-row" aria-label="Recently used apps">
  <div class="row-heading">
    <div><h2>Recently used apps</h2><p>Launches through Luma only. This never reveals what you watched.</p></div>
  </div>
  {#if !trackingEnabled}
    <div class="activity-empty"><strong>Launch history is off</strong><span>Turn on local activity tracking in Settings to record app launches.</span></div>
  {:else if apps.length === 0}
    <div class="activity-empty"><strong>No app launches yet</strong><span>Open Netflix, Disney+, Prime Video, Spotify, YouTube, or Stremio from Luma.</span></div>
  {:else}
    <div class="activity-track app-activity-track">
      {#each apps as item (item.id)}
        <ActivityCard {item} {onopen} {onremove} />
      {/each}
    </div>
  {/if}
</section>
