<script lang="ts">
  import type { ActivityItem } from '../../types';
  import { isApplication, sortRecent } from '../../services/activity';
  import ActivityCard from './ActivityCard.svelte';
  import EmptyState from '../EmptyState.svelte';

  let { items, trackingEnabled, onopen, onremove, onpreview }: {
    items: ActivityItem[];
    trackingEnabled: boolean;
    onopen: (item: ActivityItem) => void;
    onremove: (item: ActivityItem) => void;
    onpreview?: (item: ActivityItem | null) => void;
  } = $props();

  const apps = $derived(sortRecent(items.filter(isApplication)).slice(0, 8));
</script>

<section id="recent-apps" class="content-row activity-row" aria-label="Recently used apps">
  <div class="row-heading">
    <div><h2>Recently used apps</h2><p>Launches through Luma only. This never reveals what you watched.</p></div>
  </div>
  {#if !trackingEnabled}
    <EmptyState title="Launch history is off" description="Turn on local activity tracking in Settings to record app launches." />
  {:else if apps.length === 0}
    <EmptyState title="No app launches yet" description="Open Steam, Netflix, Disney+, Prime Video, Apple TV+, Hulu, Max, Peacock, Spotify, YouTube, or Stremio from Luma." />
  {:else}
    <div class="activity-track app-activity-track">
      {#each apps as item (item.id)}
        <ActivityCard {item} {onopen} {onremove} {onpreview} />
      {/each}
    </div>
  {/if}
</section>
