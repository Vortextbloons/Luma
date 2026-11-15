<script lang="ts">
  import type { PluginsState } from '../services/plugins';
  import { pluginFieldLabel } from '../services/plugins';

  let { plugins, fields, onmanage }: { plugins: PluginsState | null; fields: string[]; onmanage?: () => void } = $props();

  const owners = $derived((plugins?.plugins ?? []).filter(plugin => plugin.enabled && plugin.controls.some(field => fields.includes(field))));
  const labels = $derived([...new Set(owners.flatMap(plugin => plugin.controls.filter(field => fields.includes(field))))].map(pluginFieldLabel));
  const labelText = $derived(labels.length < 2 ? labels[0] ?? '' : `${labels.slice(0, -1).join(', ')} and ${labels.at(-1)}`);
</script>

{#if owners.length}
  <p class="plugin-ownership" role="status">
    {#each owners as plugin, index (plugin.id)}{index ? ', ' : ''}{plugin.name}{/each} controls {labelText}. Changing it here detaches that control from the plugin and keeps your choice.
    {#if onmanage}<button class="text-link" onclick={onmanage}>Manage plugins</button>{/if}
  </p>
{/if}

<style>
  .plugin-ownership { margin: 0 0 14px; padding: 10px 14px; border: 1px solid color-mix(in srgb, var(--accent) 25%, transparent); border-radius: 10px; background: color-mix(in srgb, var(--accent) 6%, transparent); color: var(--text-dim); font-size: clamp(12px, .84vw, 15px); line-height: 1.5; }
  .plugin-ownership button { margin-left: 8px; }
</style>
