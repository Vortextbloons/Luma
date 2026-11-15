<script lang="ts">
  import type { IconName } from "../lib/types";

  let { name, className = "size-5" }: { name: IconName; className?: string } = $props();
</script>

<svg class={className} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true" focusable="false">
  {#if name === "back"}
    <path d="M19 12H5" />
    <path d="m11 18-6-6 6-6" />
  {:else if name === "home"}
    <path d="m3 10 9-7 9 7v9a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z" />
    <path d="M9 21v-7h6v7" />
  {:else if name === "more"}
    <circle cx="5" cy="12" r="1.6" fill="currentColor" stroke="none" />
    <circle cx="12" cy="12" r="1.6" fill="currentColor" stroke="none" />
    <circle cx="19" cy="12" r="1.6" fill="currentColor" stroke="none" />
  {:else if name === "apps"}
    <rect x="3.5" y="3.5" width="6.5" height="6.5" rx="1.8" />
    <rect x="14" y="3.5" width="6.5" height="6.5" rx="1.8" />
    <rect x="3.5" y="14" width="6.5" height="6.5" rx="1.8" />
    <rect x="14" y="14" width="6.5" height="6.5" rx="1.8" />
  {:else if name === "keyboard"}
    <rect x="2.5" y="6" width="19" height="12" rx="2.5" />
    <path d="M6 10h.01M9.5 10h.01M13 10h.01M16.5 10h.01M7.5 13.6h9" />
  {:else if name === "display"}
    <rect x="3" y="4" width="18" height="12" rx="2" />
    <path d="M8 20h8M12 16v4" />
  {:else if name === "settings"}
    <path d="M4 7h9M17 7h3M4 17h3M11 17h9" />
    <circle cx="15" cy="7" r="2.2" />
    <circle cx="7" cy="17" r="2.2" />
  {:else if name === "up"}
    <path d="m6 14 6-6 6 6" />
  {:else if name === "down"}
    <path d="m6 10 6 6 6-6" />
  {:else if name === "left"}
    <path d="m14 6-6 6 6 6" />
  {:else if name === "right"}
    <path d="m10 6 6 6-6 6" />
  {:else if name === "previous"}
    <path d="M7 5v14" />
    <path d="M19 5 9 12l10 7z" fill="currentColor" stroke="none" />
  {:else if name === "next"}
    <path d="M17 5v14" />
    <path d="M5 5l10 7L5 19z" fill="currentColor" stroke="none" />
  {:else if name === "play"}
    <path d="M8 5.5v13l10.5-6.5z" fill="currentColor" stroke="none" />
  {:else if name === "pause"}
    <path d="M9 5h2.5v14H9zM13.5 5H16v14h-2.5z" fill="currentColor" stroke="none" />
  {:else if name === "music"}
    <path d="M9 18V6l11-2v12" />
    <circle cx="6" cy="18" r="3" />
    <circle cx="17" cy="16" r="3" />
  {:else if name === "volume-down"}
    <path d="M11 5 6 9H3v6h3l5 4z" fill="currentColor" stroke="none" />
    <path d="M15.5 8.5a5 5 0 0 1 0 7" />
  {:else if name === "volume-up"}
    <path d="M11 5 6 9H3v6h3l5 4z" fill="currentColor" stroke="none" />
    <path d="M15.5 8.5a5 5 0 0 1 0 7" />
    <path d="M18.5 6a9 9 0 0 1 0 12" />
  {:else if name === "mute"}
    <path d="M11 5 6 9H3v6h3l5 4z" fill="currentColor" stroke="none" />
    <path d="m16 9 5 6M21 9l-5 6" />
  {:else if name === "stop"}
    <rect x="7" y="7" width="10" height="10" rx="1.6" fill="currentColor" stroke="none" />
  {:else if name === "close"}
    <path d="m6 6 12 12M18 6 6 18" />
  {:else if name === "expand"}
    <path d="M8 3H5a2 2 0 0 0-2 2v3M16 3h3a2 2 0 0 1 2 2v3M21 16v3a2 2 0 0 1-2 2h-3M3 16v3a2 2 0 0 0 2 2h3" />
  {:else if name === "collapse"}
    <path d="M8 3v3a2 2 0 0 1-2 2H3M16 3v3a2 2 0 0 0 2 2h3M21 16h-3a2 2 0 0 0-2 2v3M3 16h3a2 2 0 0 1 2 2v3" />
  {:else if name === "phone"}
    <rect x="6" y="2.5" width="12" height="19" rx="3" />
    <path d="M10.5 18.5h3" />
  {:else if name === "wifi-off"}
    <path d="M2 8.5a16 16 0 0 1 4.2-2.4M10.7 5.1A16 16 0 0 1 22 8.5" />
    <path d="M5.5 12a11 11 0 0 1 3-1.4M15.9 10.7a11 11 0 0 1 2.6 1.3" />
    <path d="M8.8 15.3a6 6 0 0 1 1.5-.6M14 14.8a6 6 0 0 1 1.2.5" />
    <path d="M12 19h.01M3 3l18 18" />
  {:else if name === "check"}
    <path d="m4.5 12.5 5 5 10-11" />
  {:else if name === "chevron-down"}
    <path d="m6 9 6 6 6-6" />
  {/if}
</svg>
