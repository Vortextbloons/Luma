<script lang="ts">
  import { onDestroy } from 'svelte';
  import type { AppId } from '../types';
  import { appProviders } from '../services/activity';
  import { slideIndicator } from '../services/motion';
  import ProviderIcon from './activity/ProviderIcon.svelte';
  let { order, launch, preview, launching = null, active = null, smoothHover = true, grid = false }: {
    order: AppId[];
    launch: (id: AppId) => void;
    preview?: (id: AppId) => void;
    launching?: AppId | null;
    active?: AppId | null;
    smoothHover?: boolean;
    grid?: boolean;
  } = $props();
  const className = (id: AppId) => id.toLowerCase().replace(/\+/g, 'plus').replace(/\s+/g, '-');
  let hoverTimer: ReturnType<typeof setTimeout> | undefined;
  function clearHoverTimer() {
    if (hoverTimer !== undefined) {
      clearTimeout(hoverTimer);
      hoverTimer = undefined;
    }
  }
  function handleEnter(id: AppId) {
    clearHoverTimer();
    if (!smoothHover) {
      preview?.(id);
      return;
    }
    hoverTimer = setTimeout(() => {
      hoverTimer = undefined;
      preview?.(id);
    }, 135);
  }
  function handleFocus(id: AppId) {
    clearHoverTimer();
    preview?.(id);
  }
  function handleClick(id: AppId) {
    clearHoverTimer();
    launch(id);
  }
  onDestroy(clearHoverTimer);
</script>

<section id="apps" class="content-row apps-row">
  <div class={['app-track', { 'has-smooth-hover': smoothHover, 'is-grid': grid }]} use:slideIndicator={{ key: active }}>
    {#if smoothHover}<span class="app-indicator" aria-hidden="true"></span>{/if}
    {#each order as id (id)}
      <button data-focus-key={id} class={`app-card ${className(id)}`} class:is-launching={launching === id} class:is-active={active === id} aria-busy={launching === id} onclick={() => handleClick(id)} onmouseenter={() => handleEnter(id)} onmouseleave={clearHoverTimer} onfocus={() => handleFocus(id)}>
        {#if id === 'Custom'}
          <span class="plus-logo" aria-hidden="true">＋</span>
        {:else}
          <span class="app-icon"><ProviderIcon provider={appProviders[id]} /></span>
        {/if}
        <span>{id === 'Custom' ? 'Custom' : id}</span>
      </button>
    {/each}
  </div>
</section>
