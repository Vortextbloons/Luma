<script lang="ts">
  import { toast } from "../lib/toast.svelte";
</script>

{#if toast.visible}
  <p class="toast" class:is-bad={toast.tone === "bad"} role="status" aria-live="polite">
    {toast.message}
  </p>
{/if}

<style>
  .toast {
    position: fixed;
    top: calc(58px + env(safe-area-inset-top));
    left: 50%;
    z-index: 80;
    width: min(calc(100% - 32px), 398px);
    margin: 0;
    padding: 12px 16px;
    transform: translateX(-50%);
    animation: toast-in .22s var(--ease-remote);
    border: 1px solid var(--line-strong);
    border-radius: 14px;
    background: color-mix(in srgb, var(--surface-3) 97%, transparent);
    box-shadow: 0 18px 44px rgba(0, 0, 0, .45);
    backdrop-filter: blur(18px);
    color: var(--text);
    font-size: 12.5px;
    line-height: 1.45;
    text-align: center;
    pointer-events: none;
  }

  .toast.is-bad {
    border-color: color-mix(in srgb, var(--danger) 32%, transparent);
    color: var(--danger);
  }

  @keyframes toast-in {
    from { opacity: 0; translate: 0 -6px; }
    to { opacity: 1; translate: 0 0; }
  }
</style>
