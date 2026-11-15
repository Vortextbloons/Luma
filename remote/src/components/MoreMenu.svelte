<script lang="ts">
  import { tick } from "svelte";
  import Icon from "./Icon.svelte";

  let { onsend }: { onsend: (command: string) => Promise<unknown> | unknown } = $props();

  let open = $state(false);
  let trigger = $state<HTMLButtonElement>();
  let menu = $state<HTMLDivElement>();

  function close(restoreFocus = true): void {
    if (!open) return;
    open = false;
    if (restoreFocus) trigger?.focus();
  }

  function toggle(): void {
    if (open) close(true);
    else open = true;
  }

  async function stop(): Promise<void> {
    close(true);
    await onsend("stop");
  }

  $effect(() => {
    if (!open) return;
    void tick().then(() => menu?.querySelector<HTMLElement>('[role="menuitem"]')?.focus());

    const onPointer = (event: PointerEvent) => {
      const target = event.target;
      if (target instanceof Node && (menu?.contains(target) || trigger?.contains(target))) return;
      close(false);
    };
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        event.preventDefault();
        close(true);
        return;
      }
      if (event.key === "Tab") {
        close(false);
        return;
      }
      if (event.key === "ArrowDown" || event.key === "ArrowUp") {
        const items = [...(menu?.querySelectorAll<HTMLElement>('[role="menuitem"]') ?? [])];
        if (!items.length) return;
        event.preventDefault();
        const index = items.indexOf(document.activeElement as HTMLElement);
        const next = event.key === "ArrowDown"
          ? (index + 1 + items.length) % items.length
          : (index - 1 + items.length) % items.length;
        items[next]?.focus();
      }
    };

    document.addEventListener("pointerdown", onPointer);
    document.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("pointerdown", onPointer);
      document.removeEventListener("keydown", onKey);
    };
  });
</script>

<div class="relative">
  <button
    bind:this={trigger}
    type="button"
    class="flex h-12 w-14 items-center justify-center rounded-2xl border border-line bg-white/[0.045] text-muted transition active:scale-[.97] active:bg-white/[.11] active:text-ink"
    aria-label="More actions"
    aria-haspopup="menu"
    aria-expanded={open}
    aria-controls="more-menu"
    onclick={toggle}
  >
    <Icon name="more" className="size-[19px]" />
  </button>

  {#if open}
    <div bind:this={menu} id="more-menu" class="more-menu" role="menu" aria-label="More actions">
      <button type="button" role="menuitem" class="more-item" onclick={stop}>
        <Icon name="stop" className="size-[17px]" />
        <span>Stop Luma</span>
      </button>
    </div>
  {/if}
</div>

<style>
  .more-menu {
    position: absolute;
    top: calc(100% + 8px);
    right: 0;
    z-index: 40;
    min-width: 196px;
    padding: 5px;
    animation: fade-in .12s ease;
    border: 1px solid var(--line-strong);
    border-radius: 14px;
    background: color-mix(in srgb, var(--surface-2) 98%, transparent);
    box-shadow: 0 18px 42px rgba(0, 0, 0, .62);
  }

  .more-item {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    min-height: 46px;
    padding: 0 12px;
    border: 0;
    border-radius: 10px;
    background: transparent;
    color: var(--danger);
    font-size: 13.5px;
    font-weight: 650;
    text-align: left;
  }

  .more-item:hover,
  .more-item:focus-visible {
    outline: none;
    background: color-mix(in srgb, var(--danger) 14%, transparent);
  }
</style>
