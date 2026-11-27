<script lang="ts">
  import { onMount } from 'svelte';
  import NetworkStatus from './NetworkStatus.svelte';
  import { slideIndicator } from '../services/motion';
  let { active, navigate, settings, connect, fullscreen, toggle, viewing, switchDisplay, searchQuery = $bindable(''), onsearch }: { active: string; navigate: (tab: string) => void; settings: () => void; connect: () => void; fullscreen: boolean; toggle: () => void; viewing: () => void; switchDisplay: () => void; searchQuery?: string; onsearch: () => void } = $props();
  function search(event: SubmitEvent) {
    event.preventDefault();
    onsearch();
  }
  let clock = $state('');
  onMount(() => {
    const update = () => clock = new Date().toLocaleTimeString([], { hour: 'numeric', minute: '2-digit' });
    update();
    const timer = setInterval(update, 60000);
    return () => clearInterval(timer);
  });
</script>

{#snippet navIcon(tab: string)}
  <svg class="nav-icon" viewBox="0 0 24 24" aria-hidden="true">
    {#if tab === 'Home'}
      <path d="M4 11.2 12 4l8 7.2V20a1 1 0 0 1-1 1h-4v-6H9v6H5a1 1 0 0 1-1-1z" />
    {:else if tab === 'Sports'}
      <path d="M8 4h8v5a4 4 0 0 1-8 0z" />
      <path d="M8 5H5v2a3 3 0 0 0 3 3" />
      <path d="M16 5h3v2a3 3 0 0 1-3 3" />
      <path d="M12 13v4" />
      <path d="M9 20h6" />
    {:else if tab === 'Library'}
      <path d="M4 5v14" />
      <path d="M8 5v14" />
      <rect x="11" y="5" width="9" height="14" rx="2" />
    {:else if tab === 'Music'}
      <path d="M9 17.5V6l10-2v11.5" />
      <circle cx="6.5" cy="17.5" r="2.5" />
      <circle cx="16.5" cy="15.5" r="2.5" />
    {:else if tab === 'Apps'}
      <rect x="4" y="4" width="6" height="6" rx="1.6" />
      <rect x="14" y="4" width="6" height="6" rx="1.6" />
      <rect x="4" y="14" width="6" height="6" rx="1.6" />
      <rect x="14" y="14" width="6" height="6" rx="1.6" />
    {/if}
  </svg>
{/snippet}

<header class="topbar">
  <form class="nav-search" role="search" onsubmit={search}>
    <svg class="nav-search-icon" viewBox="0 0 24 24" aria-hidden="true"><circle cx="11" cy="11" r="7" /><path d="m20 20-3.5-3.5" /></svg>
    <input id="movie-search" type="search" bind:value={searchQuery} placeholder="Search titles, teams, or leagues" aria-label="Search all sources" autocomplete="off" spellcheck="false" />
  </form>
  <nav class="nav-pill" aria-label="Main navigation" use:slideIndicator={{ key: active }}>
    <span class="nav-indicator" aria-hidden="true"></span>
    {#each ['Home', 'Sports', 'Music', 'Library', 'Apps'] as tab (tab)}
      <button id={tab === 'Home' ? 'home-button' : undefined} class={['nav-btn', { 'is-active': active === tab }]} aria-current={active === tab ? 'page' : undefined} onclick={() => navigate(tab)}>{@render navIcon(tab)}{tab}</button>
    {/each}
  </nav>
  <div class="header-right">
    <NetworkStatus />
    <time class="clock" aria-label="Local time">{clock}</time>
    <button class="header-btn" title="Multi Watch and floating video" onclick={viewing}>Multi Watch</button>
    <button class="header-btn is-icon" title="Switch display (Ctrl+Alt+M)" aria-label="Switch display" onclick={switchDisplay}><svg class="header-icon" viewBox="0 0 24 24" aria-hidden="true"><rect x="2" y="4" width="14" height="10" rx="2" /><path d="M7 18h4M9 14v4M18 10h4v10h-9v-3" /></svg></button>
    {#if !fullscreen}
      <button class="header-btn is-icon" title="Enter fullscreen" aria-label="Enter fullscreen" onclick={toggle}>
        <svg class="header-icon" viewBox="0 0 24 24" aria-hidden="true"><path d="M8 3H5a2 2 0 0 0-2 2v3" /><path d="M16 3h3a2 2 0 0 1 2 2v3" /><path d="M21 16v3a2 2 0 0 1-2 2h-3" /><path d="M3 16v3a2 2 0 0 0 2 2h3" /></svg>
      </button>
    {/if}
    <button class="header-btn is-icon" title="Connect a remote" aria-label="Connect a remote" onclick={connect}>
      <svg class="header-icon" viewBox="0 0 24 24" aria-hidden="true"><rect x="8" y="3" width="8" height="18" rx="2.5" /><path d="M11 18h2" /></svg>
    </button>
    <button class="header-btn is-icon" title="Settings" aria-label="Open settings" onclick={settings}>
      <svg class="header-icon" viewBox="0 0 24 24" aria-hidden="true"><circle cx="12" cy="12" r="3" /><path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 2.83-2.83l.06.06A1.65 1.65 0 0 0 9 4.6h.08A1.65 1.65 0 0 0 10 3.09V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9v.08a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z" /></svg>
    </button>
  </div>
</header>
