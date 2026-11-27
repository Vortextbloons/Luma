<script lang="ts">
  import type { StremioStatus } from '../types';
  import PageHeader from './PageHeader.svelte';
  import EmptyState from './EmptyState.svelte';
  import PosterImage from './PosterImage.svelte';
  let { status, native, open, connect, sync }: {
    status: StremioStatus | null; native: boolean;
    open: (id: string) => void; connect: () => void; sync: () => Promise<void>;
  } = $props();
  let busy = $state(false);
  let error = $state('');
  let limit = $state(60);
  const visible = $derived(status?.items.slice(0, limit) ?? []);
  const kind = (value: string) => value === 'series' ? 'Series' : value === 'movie' ? 'Movie' : 'Video';
  const when = (value: number | null) => value ? new Date(value).toLocaleString([], { dateStyle: 'medium', timeStyle: 'short' }) : 'Never';

  async function synchronize() {
    if (busy) return;
    busy = true; error = '';
    try { await sync(); } catch (cause) { error = String(cause); }
    finally { busy = false; }
  }
</script>

<section class="library-page" aria-labelledby="library-title">
  <PageHeader flush eyebrow="STREMIO ACCOUNT" title="Your library" titleId="library-title"
    description={status?.connected ? `${status.items.length} titles · Last synchronized ${when(status.lastSync)}` : 'Connect your Stremio account to bring your library into Luma.'}>
    {#snippet actions()}
      {#if native && status?.connected}<button class="text-link" disabled={busy} onclick={synchronize}>{busy ? 'Synchronizing…' : 'Sync Now'}</button>{/if}
    {/snippet}
  </PageHeader>
  {#if !native}
    <EmptyState variant="panel" title="Desktop app required" description="Stremio account integration is available in the Luma desktop application." />
  {:else if !status?.connected}
    <EmptyState variant="panel" title="Connect Stremio" description="Link your account once, then your Continue Watching and library appear here. Luma never asks for your Stremio password.">
      {#snippet actions()}<button class="btn btn-primary" onclick={connect}>Connect Stremio</button>{/snippet}
    </EmptyState>
  {:else if status.items.length === 0}
    <EmptyState variant="panel" title="No library titles yet" description={status.lastSync ? 'Your synchronized Stremio library is empty.' : 'Synchronize your Stremio library to load it here.'}>
      {#snippet actions()}<button class="btn btn-primary" disabled={busy} onclick={synchronize}>{busy ? 'Synchronizing…' : 'Sync Now'}</button>{/snippet}
    </EmptyState>
  {:else}
    <div class="library-grid">
      {#each visible as item (item.id)}
        <button class="media-card" aria-label={`${item.name}, ${item.progress > 0 ? `${Math.round(item.progress)} percent watched` : kind(item.kind)}`} onclick={() => open(item.id)}>
          <span class="poster">
            <PosterImage src={item.poster} show={item.kind === 'series'} />
            {#if item.progress > 0}<span class="watch-progress" aria-hidden="true"><span style:width={`${Math.min(item.progress, 100)}%`}></span></span>{/if}
          </span>
          <span class="card-name">{item.name}</span>
          <span class="card-meta">{kind(item.kind)}{item.progress > 0 ? ` · ${Math.round(item.progress)}%` : ''}</span>
        </button>
      {/each}
    </div>
    {#if status.items.length > limit}<button class="library-more-titles" onclick={() => limit += 60}>Show more titles</button>{/if}
  {/if}
  {#if error}<p class="stremio-error library-error" role="alert">{error}</p>{/if}
</section>

<style>
  .library-more-titles { display: block; margin: 26px auto 0; padding: 12px 22px; border: 1px solid var(--line-strong); border-radius: 99px; background: #ffffff08; color: var(--text-dim); font-size: clamp(13px, .9vw, 16px); font-weight: 600; }
  .library-more-titles:focus-visible { outline: 3px solid var(--accent); outline-offset: 4px; }
</style>
