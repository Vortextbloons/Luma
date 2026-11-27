<script lang="ts">
  import type { Snippet } from 'svelte';
  let { eyebrow, title, description, actions, titleId, flush = false, divided = false }: {
    eyebrow?: string;
    title: string;
    description?: string;
    actions?: Snippet;
    titleId?: string;
    flush?: boolean;
    divided?: boolean;
  } = $props();
</script>

<header class={['page-head', { 'is-flush': flush, 'is-divided': divided }]}>
  <div class="page-head-copy">
    {#if eyebrow}<span class="eyebrow">{eyebrow}</span>{/if}
    <h1 id={titleId}>{title}</h1>
    {#if description}<p>{description}</p>{/if}
  </div>
  {#if actions}<div class="page-head-actions">{@render actions()}</div>{/if}
</header>
