<script lang="ts">
  import type { Snippet } from "svelte";

  let {
    open = false,
    id,
    titleId,
    className = "",
    onrequestclose,
    onclosed,
    children,
  }: {
    open?: boolean;
    id?: string;
    titleId?: string;
    className?: string;
    onrequestclose?: () => void;
    onclosed?: () => void;
    children?: Snippet;
  } = $props();

  let dialog = $state<HTMLDialogElement>();

  $effect(() => {
    if (!dialog) return;
    if (open && !dialog.open) dialog.showModal();
    else if (!open && dialog.open) dialog.close();
  });

  function cancel(event: Event): void {
    event.preventDefault();
    onrequestclose?.();
  }
</script>

<dialog
  bind:this={dialog}
  {id}
  data-modal
  class={`remote-sheet ${className}`}
  aria-labelledby={titleId}
  oncancel={cancel}
  onclose={() => onclosed?.()}
>
  {@render children?.()}
</dialog>
