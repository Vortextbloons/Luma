<script lang="ts">
  import { controllerBrandPaths, type ControllerFamily } from '../services/controller';

  let { family, label = '' }: { family: ControllerFamily; label?: string } = $props();
</script>

<svg class="controller-brand-logo" viewBox="0 0 24 24" role="img" aria-label={label || family}>
  <path d={controllerBrandPaths[family]} fill="currentColor" />
</svg>

<style>
  .controller-brand-logo { width: 22px; height: 22px; flex: none; }
</style>
