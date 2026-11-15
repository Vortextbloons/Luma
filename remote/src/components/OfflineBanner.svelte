<script lang="ts">
  import Icon from "./Icon.svelte";
  import { retry, session } from "../lib/session.svelte";

  let phoneOffline = $derived(session.phoneOffline);
  let title = $derived(phoneOffline ? "This phone is offline" : `Can’t reach ${session.pcName}`);
  let detail = $derived(
    phoneOffline
      ? "Reconnect to Wi-Fi, then try again."
      : "Check that Luma is open and both devices are on the same Wi-Fi.",
  );
</script>

<div class="offline" role="status" aria-live="polite">
  <span class="offline-icon" aria-hidden="true"><Icon name="wifi-off" className="size-[17px]" /></span>
  <span class="min-w-0 flex-1">
    <b class="block text-[12.5px] font-semibold leading-tight text-ink">{title}</b>
    <small class="mt-0.5 block text-[11.5px] leading-snug text-muted">{detail}</small>
  </span>
  <button
    type="button"
    class="min-h-9 flex-none rounded-full border border-line-strong bg-white/[0.06] px-3.5 text-[11.5px] font-semibold text-ink transition active:scale-[.97]"
    onclick={retry}
  >
    Retry
  </button>
</div>

<style>
  .offline {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 9px 10px;
    border: 1px solid color-mix(in srgb, var(--warn) 26%, transparent);
    border-radius: 14px;
    background: color-mix(in srgb, var(--warn) 7%, transparent);
  }

  .offline-icon {
    display: grid;
    place-items: center;
    width: 30px;
    height: 30px;
    flex: none;
    border-radius: 10px;
    background: color-mix(in srgb, var(--warn) 12%, transparent);
    color: var(--warn);
  }
</style>
