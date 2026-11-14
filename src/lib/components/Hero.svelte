<script lang="ts">
  import type { ActivityItem, MediaItem } from '../types';
  import { actionLabel, progressPercent, providerName } from '../services/activity';
  let { wallpaper, item, activity, launch, resume, details }: {
    wallpaper: string; item: MediaItem | null; activity: ActivityItem | null;
    launch: () => void; resume: () => void; details: () => void;
  } = $props();
  const percent = $derived(activity ? Math.round(progressPercent(activity) ?? 0) : 0);
  const artwork = $derived(activity?.artwork || wallpaper || item?.backdrop || '');
  const label = $derived(activity ? providerName(activity.provider) : 'Popular on Stremio');
  const eyebrow = $derived(activity ? (activity.type === 'track' ? 'LAST PLAYED' : 'CONTINUE WATCHING') : 'TRENDING NOW');
  const action = $derived(activity ? actionLabel(activity) : 'Open Stremio');
</script>

<section class="hero" aria-labelledby={activity || item ? 'hero-title' : undefined}>
  <div class="hero-art" style:background-image={artwork ? `url("${artwork}")` : undefined}></div>
  <div class="hero-vignette"></div>
  {#if activity}
  <div class="hero-copy">
    <div class="eyebrow"><span class="spotlight-dot"></span> {eyebrow}</div>
    <h1 id="hero-title">{activity.title}</h1>
    <div class="metadata">
      <span class="meta-chip">{activity.type === 'track' ? 'TRACK' : activity.type.toUpperCase()}</span>
      {#if activity.subtitle}<span>{activity.subtitle}</span>{/if}
      {#if percent > 0}<span class="meta-divider"></span><span>{percent}% watched</span>{/if}
    </div>
    <p class="hero-desc">{activity.type === 'track' ? 'Your most recent listen on Spotify.' : 'Pick up right where you left off.'}</p>
    <div class="hero-actions"><button class="btn btn-primary" onclick={resume}><span aria-hidden="true">▶</span> {action}</button></div>
  </div>
  {:else if item}
  <div class="hero-copy">
    <div class="eyebrow"><span class="spotlight-dot"></span> TRENDING NOW</div>
    <h1 id="hero-title">{item.title}</h1>
    <div class="metadata"><span class="meta-chip">MOVIE</span><span>{item.year}</span><span class="meta-divider"></span><span>{item.runtime}</span>{#if item.rating}<span class="meta-quality">★ {item.rating}</span>{/if}</div>
    <p class="hero-desc">{item.description}</p>
    <div class="hero-actions"><button class="btn btn-primary" onclick={launch}><span aria-hidden="true">▶</span> Open Stremio</button><button class="btn btn-glass" onclick={details}><span class="circle-info" aria-hidden="true">i</span> More info</button></div>
  </div>
  {/if}
  <div class="hero-footer"><span class="hero-hint">Your entertainment, together</span><span class="hero-hint">{label}</span></div>
</section>
