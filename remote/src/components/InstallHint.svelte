<script lang="ts">
  import { onMount } from "svelte";
  import Icon from "./Icon.svelte";
  import { installEnvironment, installSteps, offerInstall } from "../lib/install";

  let visible = $state(false);
  let text = $state("");

  onMount(() => {
    const environment = installEnvironment();
    if (!offerInstall(environment)) return;
    let dismissed = false;
    let seen = false;
    try {
      dismissed = localStorage.getItem("luma-install-dismissed") === "yes";
      seen = localStorage.getItem("luma-install-seen") === "yes";
    } catch (_) {}
    if (dismissed || seen) return;
    text = installSteps(environment);
    const timer = setTimeout(() => {
      visible = true;
      try {
        localStorage.setItem("luma-install-seen", "yes");
      } catch (_) {}
    }, 500);
    return () => clearTimeout(timer);
  });

  function dismiss(): void {
    visible = false;
    try {
      localStorage.setItem("luma-install-dismissed", "yes");
    } catch (_) {}
  }
</script>

{#if visible}
  <div class="install-hint">
    <span class="install-icon" aria-hidden="true"><Icon name="phone" className="size-[16px]" /></span>
    <span class="min-w-0 flex-1">
      <b class="block text-[12px] font-semibold leading-tight text-ink">Keep Luma on your home screen</b>
      <small class="mt-0.5 block text-[11px] leading-snug text-muted">{text}</small>
    </span>
    <button
      type="button"
      class="grid size-8 flex-none place-items-center rounded-full text-faint transition active:bg-white/[.09] active:text-ink"
      aria-label="Dismiss home screen tip"
      onclick={dismiss}
    >
      <Icon name="close" className="size-[14px]" />
    </button>
  </div>
{/if}

<style>
  .install-hint {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 9px 9px 9px 10px;
    animation: fade-in .3s ease;
    border: 1px solid var(--line);
    border-radius: 14px;
    background: rgba(255, 255, 255, .03);
  }

  .install-icon {
    display: grid;
    place-items: center;
    width: 30px;
    height: 30px;
    flex: none;
    border-radius: 10px;
    background: var(--accent-soft);
    color: var(--accent);
  }
</style>
