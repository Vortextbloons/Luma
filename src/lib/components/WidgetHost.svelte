<script lang="ts">
  import { widgetDefinitions } from '../services/widgets';
  import type { Settings } from '../types';
  let { widgets, onopen }: { widgets: Settings['widgets']; onopen: () => void } = $props();
  const active = $derived(widgetDefinitions.filter(widget => widgets[widget.id]?.enabled));
</script>

{#each active as widget (widget.id)}
  {@const Component = widget.component}
  <Component placement={widgets[widget.id].placement} {onopen} />
{/each}
