<script lang="ts">
  import { themes } from '../services/theme';
  import type { Settings } from '../types';
  let { settings }: { settings: Settings } = $props();
</script>

<section class="settings-group">
  <h3>Theme</h3>
  <p class="panel-description">Sets the background, surfaces, and accent together. Every screen follows the theme, including the web remote.</p>
  <div class="theme-grid" role="group" aria-label="Theme">
    {#each themes as theme (theme.id)}
      <button
        class={['theme-card', { 'is-active': settings.theme === theme.id }]}
        aria-pressed={settings.theme === theme.id}
        onclick={() => settings.theme = theme.id}
      >
        <span
          class="theme-screen"
          aria-hidden="true"
          style:--preview-bg={theme.tokens['--bg']}
          style:--preview-soft={theme.tokens['--bg-soft']}
          style:--preview-surface={theme.tokens['--surface']}
          style:--preview-surface-2={theme.tokens['--surface-2']}
          style:--preview-text={theme.tokens['--text']}
          style:--preview-muted={theme.tokens['--muted']}
          style:--preview-accent={theme.tokens['--accent']}
        >
          <span class="theme-screen-top"></span>
          <span class="theme-screen-hero"></span>
          <span class="theme-screen-row"><i></i><i></i><i></i></span>
          <span class="theme-screen-row is-small"><i></i><i></i></span>
          <span class="theme-screen-accent"></span>
        </span>
        <span class="theme-copy"><strong>{theme.name}</strong><small>{theme.description}</small></span>
      </button>
    {/each}
  </div>
</section>

<style>
  .theme-grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(200px, 1fr)); gap: 14px; }
  .theme-card { display: grid; gap: 12px; min-width: 0; padding: 12px; text-align: left; border: 1px solid var(--line); border-radius: 16px; background: color-mix(in srgb, var(--surface) 55%, transparent); color: var(--text); transition: border-color .18s, background-color .18s, transform .18s; }
  .theme-card:hover { border-color: var(--line-strong); background: color-mix(in srgb, var(--surface) 75%, transparent); }
  .theme-card.is-active { border-color: var(--accent); background: var(--accent-soft); }
  .theme-card:focus-visible { outline: 3px solid var(--accent); outline-offset: 4px; }
  .theme-screen { position: relative; display: block; aspect-ratio: 16 / 9; overflow: hidden; border: 1px solid var(--line); border-radius: 10px; background: linear-gradient(var(--preview-soft), var(--preview-bg) 68%); }
  .theme-screen-top { position: absolute; inset: 0 0 auto; height: 13%; background: var(--preview-surface-2); opacity: .85; }
  .theme-screen-hero { position: absolute; left: 8%; right: 8%; top: 24%; height: 31%; border-radius: 6px; background: linear-gradient(120deg, var(--preview-surface), var(--preview-accent)); opacity: .78; }
  .theme-screen-row { position: absolute; left: 8%; right: 30%; top: 62%; display: flex; gap: 4%; }
  .theme-screen-row.is-small { right: 48%; top: 79%; opacity: .7; }
  .theme-screen-row i { display: block; width: 22%; aspect-ratio: 2 / 3; border-radius: 3px; background: var(--preview-surface); border: 1px solid var(--line); }
  .theme-screen-row.is-small i { aspect-ratio: 16 / 9; }
  .theme-screen-accent { position: absolute; right: 8%; top: 62%; width: 14%; aspect-ratio: 1; border-radius: 50%; background: var(--preview-accent); box-shadow: 0 0 18px var(--preview-accent); }
  .theme-copy { display: grid; gap: 3px; min-width: 0; }
  .theme-copy strong { font-size: 15px; letter-spacing: -.01em; }
  .theme-copy small { color: var(--muted); font-size: 12px; line-height: 1.35; }
  @media (max-width: 640px) { .theme-grid { grid-template-columns: 1fr 1fr; } .theme-copy small { display: none; } }
</style>
