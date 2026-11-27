<script lang="ts">
  import { tick } from 'svelte';
  import FramePairing from './FramePairing.svelte';
  import StremioIntegration from './StremioIntegration.svelte';
  import SpotifyIntegration from './spotify/SpotifyIntegration.svelte';
  import ThemeSettings from './ThemeSettings.svelte';
  import { isSportsFavorite, loadSportsTeams, majorSportsLeagues, readSportsFavorites, toggleSportsFavorite, writeSportsFavorites } from '../services/sports';
  import type { Settings } from '../types';

  let { settings, native, ondone }: { settings: Settings; native: boolean; ondone: () => void } = $props();
  const steps = ['Choose a theme', 'Pair your phone', 'Connect your services', 'Follow leagues and teams'];
  let step = $state(0);
  let heading: HTMLHeadingElement;
  let league = $state('');
  let teams = $state.raw<Awaited<ReturnType<typeof loadSportsTeams>>>([]);
  let loading = $state(false);
  let error = $state('');
  let favorites = $state(readSportsFavorites());
  let request = 0;

  async function change(next: number) {
    step = next;
    await tick();
    heading.focus();
  }

  export function back() {
    if (!step) ondone();
    else void change(step - 1);
    return true;
  }

  async function chooseLeague(next: string) {
    league = next;
    const current = ++request;
    loading = true;
    error = '';
    teams = [];
    try {
      const result = await loadSportsTeams(next);
      if (current === request) teams = result;
    } catch {
      if (current === request) error = 'Teams could not load.';
    } finally {
      if (current === request) loading = false;
    }
  }

  function follow(team: string) {
    favorites = toggleSportsFavorite(favorites, league, team);
    writeSportsFavorites(favorites);
  }
</script>

<dialog class="setup-dialog" aria-labelledby="setup-title" {@attach element => { element.showModal(); }} oncancel={event => { event.preventDefault(); back(); }}>
  <header>
    <p class="eyebrow">LUMA SETUP · {step + 1} OF {steps.length}</p>
    <h2 id="setup-title" bind:this={heading} tabindex="-1">{steps[step]}</h2>
  </header>
  <div class="setup-actions">
    {#if step}<button class="panel-secondary" onclick={back}>Back</button>{/if}
    <button class="panel-secondary" onclick={ondone}>Skip setup</button>
    <button class="panel-secondary is-primary" onclick={() => step === steps.length - 1 ? ondone() : void change(step + 1)}>{step === steps.length - 1 ? 'Go to Home' : 'Next'}</button>
  </div>
  {#if step === 0}
    <ThemeSettings {settings} />
  {:else if step === 1}
    <FramePairing {native} autoConnect pairingOnly />
  {:else if step === 2}
    <StremioIntegration {native} />
    <SpotifyIntegration {native} />
  {:else}
    <p>Follow a whole league or individual teams to see their games under Your teams.</p>
    <div class="setup-options" aria-label="Leagues" role="group">
      {#each majorSportsLeagues as option (option)}
        <button class="panel-secondary" aria-pressed={league === option} disabled={!native} onclick={() => void chooseLeague(option)}>{option}</button>
      {/each}
    </div>
    {#if league}
      <button class="panel-secondary" aria-pressed={isSportsFavorite(favorites, league, '*')} onclick={() => follow('*')}>Follow all {league} teams</button>
      {#if loading}<p role="status">Loading teams…</p>{/if}
      {#if error}<p role="alert">{error}</p><button class="panel-secondary" onclick={() => void chooseLeague(league)}>Retry</button>{/if}
      <div class="setup-options" aria-label="Teams" role="group">
        {#each teams as team (team.name)}
          <button class="panel-secondary" aria-pressed={isSportsFavorite(favorites, league, team.name)} onclick={() => follow(team.name)}>{team.name}</button>
        {/each}
      </div>
    {/if}
  {/if}
</dialog>

<style>
  .setup-dialog { width: min(880px, calc(100vw - 48px)); max-height: calc(100dvh - 48px); margin: auto; padding: clamp(24px, 3vw, 44px); overflow-y: auto; border: 1px solid var(--line-strong); border-radius: 24px; background: var(--surface-2); color: var(--text); }
  .setup-dialog::backdrop { background: #05060ad9; }
  h2 { margin: 8px 0 16px; font-size: clamp(28px, 3vw, 44px); }
  .setup-actions, .setup-options { display: flex; flex-wrap: wrap; gap: 12px; margin-block: 16px; }
  button { min-height: 48px; }
  button[aria-pressed=true] { border-color: var(--accent); }
</style>
