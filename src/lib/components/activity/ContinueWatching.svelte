<script lang="ts">
  import type { ActivityItem } from '../../types';
  import { isContinueWatching, providerName, sortRecent } from '../../services/activity';
  import ActivityCard from './ActivityCard.svelte';
  import EmptyState from '../EmptyState.svelte';

  let { items, trackingEnabled, onopen, onpreview, focusProvider = null }: {
    items: ActivityItem[];
    trackingEnabled: boolean;
    onopen: (item: ActivityItem) => void;
    onpreview?: (item: ActivityItem | null) => void;
    focusProvider?: string | null;
  } = $props();

  function scoped(provider: string): ActivityItem[] {
    const own = sortRecent(items.filter(item => item.provider === provider && item.type !== 'application'));
    const resumable = own.filter(isContinueWatching);
    return resumable.length ? resumable : own.filter(item => item.type === 'track');
  }

  const all = $derived(sortRecent(items.filter(isContinueWatching)));
  const focused = $derived(focusProvider ? scoped(focusProvider) : []);
  const watching = $derived((focused.length ? focused : all).slice(0, 4));
  const heading = $derived(focused.length && focusProvider
    ? `${focused[0].type === 'track' ? 'Continue listening' : 'Continue watching'} on ${providerName(focusProvider)}`
    : 'Continue watching');
</script>

<section id="continue-watching" class="content-row activity-row" aria-label={heading}>
  {#key heading}
  <div class="provider-preview">
  <div class="row-heading">
    <div><h2>{heading}</h2><p>Only verified playback positions from your providers</p></div>
  </div>
  {#if !trackingEnabled}
    <EmptyState title="Activity tracking is off" description="Turn on local activity tracking in Settings to build this row." />
  {:else if watching.length === 0}
    <EmptyState title="Nothing in progress" description="Connect Plex, Jellyfin, or Trakt, or play a video in Luma's YouTube window." />
  {:else}
    <div class="activity-track is-cinematic">
      {#each watching as item (item.id)}
        <ActivityCard {item} {onopen} {onpreview} cinematic />
      {/each}
    </div>
  {/if}
  </div>
  {/key}
</section>

<style>
  .provider-preview { animation: provider-preview-in 220ms cubic-bezier(.2, .7, .2, 1); }
  @keyframes provider-preview-in {
    from { opacity: 0; transform: translateY(6px); }
    to { opacity: 1; transform: translateY(0); }
  }
  @media (prefers-reduced-motion: reduce) {
    .provider-preview { animation: none; }
  }
</style>
