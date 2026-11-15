<script lang="ts">
  import BrandMark from "./BrandMark.svelte";
  import Icon from "./Icon.svelte";
  import { session } from "../lib/session.svelte";

  let { onapps, ontype, onsettings }: {
    onapps: () => void;
    ontype: () => void;
    onsettings: () => void;
  } = $props();

  let view = $derived(session.state?.view ?? "Home");
  let status = $derived(!session.paired ? "offline" : session.connecting ? "connecting" : session.online ? "online" : "offline");
  let statusLabel = $derived(
    status === "online"
      ? `Connected to ${session.pcName}`
      : status === "connecting"
        ? "Connecting to Luma"
        : "Luma PC disconnected",
  );
  let dotClass = $derived(
    status === "online"
      ? "bg-good shadow-[0_0_8px_color-mix(in_srgb,var(--good)_70%,transparent)]"
      : status === "connecting"
        ? "bg-warn animate-pulse"
        : "bg-faint",
  );
  let searching = $derived(session.state?.window === "YouTube");

  const iconButton = "grid size-10 flex-none place-items-center rounded-full border border-line bg-white/[0.04] text-muted transition active:scale-95 active:border-line-strong active:bg-white/[0.09] active:text-ink";
</script>

<header class="flex min-h-11 items-center justify-between gap-2">
  <BrandMark className="size-6" />

  <div class="flex min-w-0 items-center gap-1.5">
    {#if view && view !== "Home"}
      <span
        class="hidden max-w-[72px] truncate rounded-full border border-line-strong bg-accent-soft px-2.5 py-1 text-[11px] font-semibold text-ink min-[360px]:inline-flex"
        title={view}
      >
        {view}
      </span>
    {/if}

    <button type="button" class={iconButton} data-apps-trigger aria-label="Apps" title="Apps" onclick={onapps}>
      <Icon name="apps" className="size-[18px]" />
    </button>

    <button
      type="button"
      class={iconButton}
      data-type-trigger
      aria-label={searching ? `Search YouTube on ${session.pcName}` : "Type or search on your PC"}
      title={searching ? `Search YouTube on ${session.pcName}` : "Type or search on your PC"}
      onclick={ontype}
    >
      <Icon name="keyboard" className="size-[18px]" />
    </button>

    <span
      class="relative grid size-10 flex-none place-items-center rounded-full border border-line bg-white/[0.035] text-muted"
      role="status"
      aria-live="polite"
      aria-label={statusLabel}
      title={statusLabel}
    >
      <Icon name="display" className="size-[17px]" />
      <span class={`absolute bottom-2 right-2 size-2 rounded-full ring-2 ring-canvas ${dotClass}`} aria-hidden="true"></span>
    </span>

    <button
      type="button"
      class={iconButton}
      data-settings-trigger
      aria-label="Remote settings"
      title="Remote settings"
      aria-haspopup="dialog"
      onclick={onsettings}
    >
      <Icon name="settings" className="size-[18px]" />
    </button>
  </div>
</header>
