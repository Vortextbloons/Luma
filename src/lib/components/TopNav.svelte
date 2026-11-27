<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { slideIndicator } from '../services/motion';
  import { connectedControllers, controllerActions, controllerButtonLabel, controllerFamily } from '../services/controller';
  import type { Settings } from '../types';
  let { native, controllerEnabled, binds, sleep, exit, active, navigate, settings, connect, fullscreen, toggle, viewing, switchDisplay, startScreensaver, searchQuery = $bindable(''), onsearch }: { native: boolean; controllerEnabled: boolean; binds: Settings["controllerBinds"]; sleep: () => Promise<void>; exit: () => void; active: string; navigate: (tab: string) => void; settings: () => void; connect: () => void; fullscreen: boolean; toggle: () => void; viewing: () => void; switchDisplay: () => void; startScreensaver: () => void; searchQuery?: string; onsearch: () => void } = $props();
  function search(event: SubmitEvent) {
    event.preventDefault();
    onsearch();
  }
  let menu: HTMLDialogElement;
  let menuButton: HTMLButtonElement;
  let menuOpen = $state(false);
  let view = $state<'quick' | 'bindings' | 'sleep'>('quick');
  let family = $state(controllerFamily(null));
  let sleepError = $state('');
  let sleeping = $state(false);

  async function showView(next: typeof view) {
    if (next === 'bindings') family = controllerFamily(connectedControllers()[0]);
    view = next;
    await tick();
    menu.querySelector<HTMLElement>(next === 'quick' ? '[data-quick-first]' : '[data-quick-back]')?.focus();
  }

  async function sleepNow() {
    sleeping = true;
    sleepError = '';
    try { await sleep(); closeMenu(); }
    catch (error) { sleepError = String(error); }
    finally { sleeping = false; }
  }

  export function closeMenu() {
    if (!menu?.open) return false;
    menu.close();
    menuOpen = false;
    menuButton.focus({ preventScroll: true });
    return true;
  }

  export function backMenu() {
    if (!menu?.open) return false;
    if (view === 'quick') return closeMenu();
    void showView('quick');
    return true;
  }

  function openMenu() {
    view = 'quick';
    sleepError = '';
    menu.style.top = `${menuButton.getBoundingClientRect().bottom + 8}px`;
    menu.showModal();
    menuOpen = true;
  }

  function run(action: () => void) {
    closeMenu();
    action();
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
    <time class="header-time" aria-label="Local time">{clock}</time>
    <button class="header-btn is-icon" title={fullscreen ? 'Exit fullscreen' : 'Enter fullscreen'} aria-label={fullscreen ? 'Exit fullscreen' : 'Enter fullscreen'} onclick={toggle}>
      <svg class="header-icon" viewBox="0 0 24 24" aria-hidden="true">
        {#if fullscreen}<path d="M3 8h5V3m8 0v5h5m0 8h-5v5m-8 0v-5H3" />
        {:else}<path d="M8 3H5a2 2 0 0 0-2 2v3m13-5h3a2 2 0 0 1 2 2v3m0 8v3a2 2 0 0 1-2 2h-3M3 16v3a2 2 0 0 0 2 2h3" />{/if}
      </svg>
    </button>
    <button class="header-btn is-icon" title="Settings" aria-label="Open settings" onclick={settings}>
      <svg class="header-icon" viewBox="0 0 24 24" aria-hidden="true"><circle cx="12" cy="12" r="3" /><path d="M10 3h4l.7 2.6 2 .9 2.4-1.3 2 3.5-1.7 2v2.6l1.7 2-2 3.5-2.4-1.3-2 .9L14 21h-4l-.7-2.6-2-.9-2.4 1.3-2-3.5 1.7-2v-2.6l-1.7-2 2-3.5 2.4 1.3 2-.9z" /></svg>
    </button>
    <button bind:this={menuButton} class="header-btn is-icon" title="Quick menu" aria-label="Quick menu" aria-haspopup="dialog" aria-expanded={menuOpen} onclick={openMenu}>
      <svg class="header-icon dots" viewBox="0 0 24 24" aria-hidden="true"><circle cx="5" cy="12" r="1.7" /><circle cx="12" cy="12" r="1.7" /><circle cx="19" cy="12" r="1.7" /></svg>
    </button>
    <dialog bind:this={menu} class="header-menu" aria-label="More options" oncancel={event => { event.preventDefault(); backMenu(); }} onpointerdown={event => { if (event.target === menu) closeMenu(); }}>
      <div class="menu-content">
        {#if view === 'quick'}
        <button data-quick-first onclick={() => void showView('bindings')}>Gamepad bindings</button>
        <button onclick={() => run(viewing)}>Multi Watch</button>
        {#if native}
          <button onclick={() => run(switchDisplay)}>Switch display</button>
          <button onclick={() => void showView('sleep')}>Sleep PC</button>
          <button onclick={() => run(exit)}>Exit Luma</button>
        {/if}
        <button onclick={() => run(connect)}>Connect a remote</button>
        <button onclick={() => run(startScreensaver)}>Start screensaver</button>
        {:else}
          <button data-quick-back onclick={() => void showView('quick')}>Back to Quick menu</button>
          {#if view === 'bindings'}
            {#if controllerEnabled}
              <dl class="quick-bindings">
                {#each controllerActions as action (action.key)}
                  <div><dt>{action.label}</dt><dd>{controllerButtonLabel(binds[action.key], family)}</dd></div>
                {/each}
              </dl>
            {:else}<p>Gamepad controls are turned off.</p>{/if}
          {:else}
            <button disabled={sleeping} onclick={sleepNow}>{sleeping ? 'Putting PC to sleep…' : 'Confirm Sleep PC'}</button>
            {#if sleepError}<p role="alert">{sleepError}</p>{/if}
          {/if}
        {/if}
      </div>
    </dialog>
  </div>
</header>

<style>
  .header-time { white-space: nowrap; font-size: clamp(12px, .85vw, 17px); font-variant-numeric: tabular-nums; }
  .dots { fill: currentColor; stroke: none; }
  .header-menu { position: fixed; inset: 0 var(--side) auto auto; margin: 0; padding: 0; width: min(16rem, calc(100vw - 2 * var(--side))); border: 1px solid var(--line-strong); border-radius: var(--control-radius); background: var(--surface-2); color: var(--text); box-shadow: 0 16px 36px #0008; }
  .header-menu::backdrop { background: #0003; }
  .quick-bindings { padding: 0 12px; margin: 0; }
  .quick-bindings div { display: flex; justify-content: space-between; gap: 12px; padding: 5px 0; }
  .quick-bindings dd { margin: 0; color: var(--muted); text-align: end; }
  .header-menu { max-height: 75dvh; overflow: auto; }
  .menu-content { display: grid; gap: 4px; padding: 8px; }
  .menu-content button { min-height: 44px; padding: 0 12px; text-align: start; border-radius: 8px; background: transparent; color: inherit; }
  .menu-content button:hover, .menu-content button:focus-visible { background: var(--surface-3); }
  .menu-content button:focus-visible { outline-offset: -2px; }
</style>
