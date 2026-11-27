<script lang="ts">
  import type { Snippet } from 'svelte';
  let { title, description, actions, variant = 'inline', role }: {
    title: string;
    description?: string;
    actions?: Snippet;
    variant?: 'inline' | 'panel';
    role?: 'status' | 'alert';
  } = $props();
</script>

<div class={['empty-state', `is-${variant}`]} {role}>
  <div class="empty-state-copy">
    <strong>{title}</strong>
    {#if description}<span>{description}</span>{/if}
  </div>
  {#if actions}<div class="empty-state-actions">{@render actions()}</div>{/if}
</div>
