<script lang="ts">
  import { tick } from "svelte";
  import Modal from "./Modal.svelte";
  import { api } from "../lib/api";
  import { session } from "../lib/session.svelte";
  import { say } from "../lib/toast.svelte";

  let { open = false, onclose }: { open?: boolean; onclose: () => void } = $props();

  let value = $state("");
  let busy = $state(false);
  let feedback = $state("");
  let input = $state<HTMLInputElement>();
  let wasOpen = false;

  let searching = $derived(session.state?.window === "YouTube");
  let title = $derived(searching ? "Search YouTube" : "Type on your PC");
  let hint = $derived(
    searching
      ? "Type what you want to watch and Luma opens the results on your TV."
      : "Select a text field on the TV first, then send the text.",
  );
  let label = $derived(searching ? "Search for" : "Text to send");
  let action = $derived(searching ? "Search" : "Send text");

  $effect(() => {
    if (open && !wasOpen) {
      wasOpen = true;
      value = "";
      feedback = "";
      void tick().then(() => input?.focus());
    } else if (!open) {
      wasOpen = false;
    }
  });

  async function submit(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    const text = value.trim();
    if (busy || !text) return;
    busy = true;
    feedback = "";
    try {
      const { response, data } = await api.text(text);
      if (!response.ok || data?.error) {
        throw new Error(data?.error || "Could not send text. Check your connection and try again.");
      }
      value = "";
      onclose();
      say(searching ? `Searching YouTube for “${text}”.` : "Text sent to your PC.");
    } catch (error) {
      feedback = error instanceof Error ? error.message : "Could not send text. Check the connection and try again.";
    } finally {
      busy = false;
    }
  }
</script>

<Modal
  id="text-dialog"
  titleId="text-title"
  {open}
  className="m-auto max-h-[calc(100dvh-32px)] w-[min(420px,calc(100%-32px))] overflow-y-auto rounded-[22px] border border-line-strong bg-surface-2 p-5 shadow-[0_30px_80px_rgba(0,0,0,.55)] sm:p-6"
  onrequestclose={onclose}
>
  <form onsubmit={submit}>
    <h2 id="text-title" class="text-[21px] font-semibold tracking-[-0.03em] text-ink">{title}</h2>
    <p class="mt-2 text-[12.5px] leading-relaxed text-muted">{hint}</p>

    <label for="remote-text" class="mt-5 block text-[11.5px] font-semibold text-muted">{label}</label>
    <input
      id="remote-text"
      bind:this={input}
      bind:value={value}
      maxlength="200"
      autocomplete="off"
      enterkeyhint={searching ? "search" : "send"}
      spellcheck="false"
      required
      placeholder={searching ? "Try “cooking videos”" : ""}
      class="mt-2 h-13 w-full rounded-xl border border-line-strong bg-canvas px-4 text-[15px] text-ink caret-accent placeholder:text-faint focus-visible:border-transparent"
    />

    <p class="mt-2 min-h-5 text-[12px] leading-relaxed text-danger" role="status">{feedback}</p>

    <div class="mt-4 grid grid-cols-2 gap-2">
      <button
        type="button"
        class="min-h-11 rounded-xl border border-line bg-white/[0.045] text-[13px] font-semibold text-ink-dim transition active:scale-[.98]"
        onclick={onclose}
      >
        Cancel
      </button>
      <button
        type="submit"
        disabled={busy || !value.trim()}
        aria-busy={busy}
        class="min-h-11 rounded-xl bg-accent text-[13px] font-semibold text-accent-ink transition active:scale-[.98] disabled:opacity-50"
      >
        {busy ? "Sending…" : action}
      </button>
    </div>
  </form>
</Modal>
