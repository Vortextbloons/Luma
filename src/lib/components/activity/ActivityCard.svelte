<script lang="ts">
  import type { ActivityItem } from '../../types';
  import { actionLabel, formatRelative, progressLabel, progressPercent, providerName, typeLabel } from '../../services/activity';
  import ProviderIcon from './ProviderIcon.svelte';

  let { item, onopen, onremove }: {
    item: ActivityItem;
    onopen: (item: ActivityItem) => void;
    onremove?: (item: ActivityItem) => void;
  } = $props();

  const percent = $derived(progressPercent(item));
  const finished = $derived(item.completed === true);
  const isApp = $derived(item.type === 'application');
  const isTrack = $derived(item.type === 'track');
  const launches = $derived(item.launchCount && item.launchCount > 1 ? `${item.launchCount} opens` : 'Opened');
  const meta = $derived(isApp
    ? `${launches} · ${formatRelative(item.lastActivityAt)}`
    : finished ? `Finished · ${formatRelative(item.lastActivityAt)}` : progressLabel(item) || formatRelative(item.lastActivityAt));
</script>

<article class={['activity-card', { 'is-app': isApp, 'is-track': isTrack }]}>
  <button class="activity-main" onclick={() => onopen(item)} aria-label={`${actionLabel(item)} ${item.title}`}>
    {#if isApp}
      <span class="activity-app-art"><ProviderIcon provider={item.provider} /></span>
    {:else}
      <span class="activity-art">
        {#if item.artwork}
          <img src={item.artwork} alt="" loading="lazy" decoding="async" />
        {:else}
          <span class="activity-art-fallback"><ProviderIcon provider={item.provider} /></span>
        {/if}
        <span class="activity-provider"><ProviderIcon provider={item.provider} /><span>{providerName(item.provider)}</span></span>
        {#if percent !== null && !finished}
          <span class="progress" aria-label={progressLabel(item)}><i style:width={`${percent}%`}></i></span>
        {/if}
      </span>
    {/if}
    <span class="activity-copy">
      <span class="activity-title">{item.title}</span>
      {#if item.subtitle}<span class="activity-subtitle">{item.subtitle}</span>{/if}
      <span class="activity-meta">
        {#if !isApp}<span class="activity-type">{typeLabel(item)}</span>{/if}
        <span>{meta}</span>
      </span>
    </span>
  </button>
  <div class="activity-actions">
    <button class="text-link" onclick={() => onopen(item)}>{actionLabel(item)} <span aria-hidden="true">→</span></button>
    {#if onremove}<button class="activity-remove" aria-label={`Remove ${item.title} from activity history`} title="Remove from history" onclick={() => onremove(item)}>Remove</button>{/if}
  </div>
</article>
