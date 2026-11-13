<script lang="ts">
  import type { AppId } from '../types';
  import spotify from '../../assets/spotify.svg';
  import stremio from '../../assets/stremio.svg';
  import youtube from '../../assets/youtube.png';
  let { order, launch, customize }: { order: AppId[]; launch: (id: AppId) => void; customize: () => void } = $props();
</script>

<section id="apps" class="content-row apps-row">
  <div class="row-heading"><div><h2>Your apps</h2><p>Running on your Windows PC</p></div><button class="text-link" onclick={customize}>Rearrange apps <span>→</span></button></div>
  <div class="app-track">
    {#each order as id (id)}
      <button class={`app-card ${id.toLowerCase()}`} onclick={() => launch(id)}>
        {#if id === 'YouTube'}
          <img class="app-icon" src={youtube} alt="" />
        {:else if id !== 'Custom'}<img class="app-icon" src={id === 'Spotify' ? spotify : stremio} alt="" />{:else}<span class="plus-logo" aria-hidden="true">＋</span>{/if}
        <span>{id === 'Custom' ? 'Custom app' : id}</span>
      </button>
    {/each}
  </div>
</section>
