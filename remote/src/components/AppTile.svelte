<script lang="ts">
  import type { AppEntry } from "../lib/types";

  let {
    app,
    launching = null,
    disabled = false,
    large = false,
    onlaunch,
  }: {
    app: AppEntry;
    launching?: string | null;
    disabled?: boolean;
    large?: boolean;
    onlaunch: (app: AppEntry) => void;
  } = $props();

  let busy = $derived(launching === app.command);

  const base = "group relative flex flex-col items-center justify-center gap-2 rounded-[18px] border border-line bg-white/[0.035] px-2 text-center transition active:scale-[.96] active:border-line-strong";
  let className = $derived(`${base} ${large ? "min-h-[88px] py-3" : "min-h-[76px] py-2.5"} ${disabled && !busy ? "opacity-45" : ""}`);

  function click(): void {
    if (!disabled) onlaunch(app);
  }
</script>

<button
  type="button"
  class={className}
  aria-label={`Open ${app.name}`}
  aria-busy={busy}
  {disabled}
  onclick={click}
>
  <img
    src={app.icon}
    alt=""
    width={large ? 40 : 32}
    height={large ? 40 : 32}
    loading="lazy"
    decoding="async"
    class={large ? "size-10 object-contain" : "size-8 object-contain"}
  />
  <span class={`max-w-full truncate font-semibold text-ink-dim ${large ? "text-[12px]" : "text-[11px]"}`}>
    {app.name}
  </span>
  {#if busy}
    <span class="absolute right-2 top-2 size-3 animate-spin rounded-full border border-accent/30 border-t-accent" aria-hidden="true"></span>
  {/if}
</button>
