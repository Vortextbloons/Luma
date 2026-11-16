<script lang="ts">
  import BrandMark from "./BrandMark.svelte";
  import Icon from "./Icon.svelte";
  import { session } from "../lib/session.svelte";

  let { onsearch, onapps, ontype, onsettings }: {
    onsearch: () => void;
    onapps: () => void;
    ontype: () => void;
    onsettings: () => void;
  } = $props();

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

  const iconButton = "grid size-11 flex-none place-items-center rounded-full border border-line bg-white/[0.04] text-muted transition active:scale-95 active:border-line-strong active:bg-white/[0.09] active:text-ink";
</script>

<header class="flex min-h-11 flex-wrap items-center justify-between gap-2">
  <div class="flex min-w-0 items-center gap-2">
    <span role="status" aria-live="polite" aria-label={statusLabel} title={statusLabel} class="flex size-2 flex-none items-center justify-center">
      <span class={`size-2 rounded-full ${dotClass}`} aria-hidden="true"></span>
    </span>
    <BrandMark className="size-6" />
  </div>

  <div class="ml-auto flex flex-none items-center gap-1.5">
    <button type="button" class={iconButton} data-search-trigger aria-label="Search Luma" title="Search Luma" aria-haspopup="dialog" onclick={onsearch}>
      <Icon name="search" className="size-[18px]" />
    </button>

    <button type="button" class={iconButton} data-apps-trigger aria-label="Apps" title="Apps" onclick={onapps}>
      <Icon name="apps" className="size-[18px]" />
    </button>

    <button
      type="button"
      class={iconButton}
      data-type-trigger
      aria-label={searching ? `Search YouTube on ${session.pcName}` : "Type on your PC"}
      title={searching ? `Search YouTube on ${session.pcName}` : "Type on your PC"}
      onclick={ontype}
    >
      <Icon name="keyboard" className="size-[18px]" />
    </button>

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
