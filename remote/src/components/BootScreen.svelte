<script lang="ts">
  import BrandMark from "./BrandMark.svelte";
  import { retry, session } from "../lib/session.svelte";
</script>

<main class="screen-enter grid min-h-dvh place-items-center px-6 py-10">
  <div class="w-full max-w-[340px]">
    <BrandMark className="size-10" />

    {#if session.bootError}
      <p class="mt-9 text-[11px] font-semibold text-warn">Connection lost</p>
      <h1 class="mt-2 text-balance text-[30px] font-semibold leading-[1.05] tracking-[-0.045em] text-ink">
        Can’t reach Luma
      </h1>
      <p class="mt-3 text-[13.5px] leading-relaxed text-muted">
        {session.phoneOffline
          ? "Reconnect this phone to Wi-Fi, then try again."
          : "Check that Luma is open on your PC and that both devices are on the same Wi-Fi."}
      </p>
      <button
        type="button"
        class="mt-7 flex h-13 w-full items-center justify-center rounded-2xl bg-accent text-[14px] font-semibold text-accent-ink transition active:scale-[.985]"
        onclick={retry}
      >
        Try again
      </button>
    {:else}
      <h1 class="mt-9 text-balance text-[30px] font-semibold leading-[1.05] tracking-[-0.045em] text-ink">
        Connecting to Luma
      </h1>
      <p class="mt-3 text-[13.5px] leading-relaxed text-muted">
        Finding your PC on the local network.
      </p>
      <div class="mt-8 flex items-center gap-3 text-[12px] font-medium text-faint">
        <span class="spinner" aria-hidden="true"></span>
        <span>This should only take a moment</span>
      </div>
    {/if}
  </div>
</main>

<style>
  .spinner {
    width: 16px;
    height: 16px;
    flex: none;
    border: 2px solid color-mix(in srgb, var(--accent) 24%, transparent);
    border-top-color: var(--accent);
    border-radius: 50%;
    animation: spin .8s linear infinite;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }
</style>
