<script lang="ts">
  import type { AppId } from '../types';
  import { appProviders } from '../services/activity';
  import ProviderIcon from './activity/ProviderIcon.svelte';
  let { order, launch, customize, preview, launching = null, showHeading = true, active = null }: {
    order: AppId[];
    launch: (id: AppId) => void;
    customize?: () => void;
    preview?: (id: AppId) => void;
    launching?: AppId | null;
    showHeading?: boolean;
    active?: AppId | null;
  } = $props();
  const className = (id: AppId) => id.toLowerCase().replace(/\+/g, 'plus').replace(/\s+/g, '-');
</script>

<section id="apps" class="content-row apps-row">
  {#if showHeading}<div class="row-heading"><div><h2>Your apps</h2><p>Running on your Windows PC</p></div>{#if customize}<button class="text-link" onclick={customize}>Edit apps <span>→</span></button>{/if}</div>{/if}
  <div class="app-track">
    {#each order as id (id)}
      <button class={`app-card ${className(id)}`} class:is-launching={launching === id} class:is-active={active === id} aria-busy={launching === id} onclick={() => launch(id)} onmouseenter={() => preview?.(id)} onfocus={() => preview?.(id)}>
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
