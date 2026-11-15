<script lang="ts">
  import type { Snippet } from "svelte";

  let {
    command,
    onsend,
    immediate = true,
    disabled = false,
    label,
    className = "",
    children,
  }: {
    command: string;
    onsend: (command: string) => Promise<unknown> | unknown;
    immediate?: boolean;
    disabled?: boolean;
    label?: string;
    className?: string;
    children?: Snippet;
  } = $props();

  let busy = $state(false);

  async function fire(): Promise<void> {
    if (busy || disabled) return;
    busy = true;
    try {
      await onsend(command);
    } finally {
      busy = false;
    }
  }

  function pointerdown(event: PointerEvent): void {
    if (!immediate || disabled || !event.isPrimary || event.button !== 0) return;
    event.preventDefault();
    void fire();
  }

  function click(event: MouseEvent): void {
    if (immediate) {
      if (event.detail === 0) void fire();
      return;
    }
    void fire();
  }
</script>

<button
  type="button"
  {disabled}
  aria-busy={busy}
  aria-label={label}
  class={className}
  onpointerdown={pointerdown}
  onclick={click}
>
  {@render children?.()}
</button>
