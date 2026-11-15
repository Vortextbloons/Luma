<script lang="ts">
  import { onMount, tick } from "svelte";
  import BrandMark from "./BrandMark.svelte";
  import { pair, refresh, session } from "../lib/session.svelte";

  let form = $state<HTMLFormElement>();
  let input = $state<HTMLInputElement>();
  let code = $state("");

  onMount(() => {
    input?.focus();
  });

  function onInput(event: Event): void {
    const field = event.currentTarget as HTMLInputElement;
    const next = field.value.replace(/\D/g, "").slice(0, 6);
    field.value = next;
    code = next;
    if (next.length === 6 && !session.pairingBusy) form?.requestSubmit();
  }

  async function submit(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    if (session.pairingBusy) return;
    const accepted = await pair(code);
    if (!accepted) return;
    await refresh();
    await tick();
    const target = document.querySelector<HTMLElement>(".pad-key.pad-ok")
      ?? document.querySelector<HTMLElement>("[data-apps-trigger]");
    target?.focus();
  }
</script>

<main class="screen-enter flex min-h-dvh flex-col px-5 pb-[calc(24px+env(safe-area-inset-bottom))] pt-[calc(20px+env(safe-area-inset-top))]">
  <div class="flex items-center justify-between">
    <BrandMark className="size-7" />
    <span class="rounded-full border border-line bg-white/[0.04] px-3 py-1.5 text-[10.5px] font-semibold text-faint">
      Web remote
    </span>
  </div>

  <div class="mx-auto flex w-full max-w-[400px] flex-1 flex-col justify-center py-10">
    <h1 class="text-balance text-[38px] font-semibold leading-[.98] tracking-[-0.055em] text-ink">
      Pair this remote
    </h1>
    <p class="mt-4 max-w-[36ch] text-[13.5px] leading-relaxed text-muted">
      Open Luma on your PC, choose <strong class="font-semibold text-ink-dim">Settings → Remotes &amp; Devices → Connect a remote</strong>, then scan the QR code or type the six-digit code.
    </p>

    <form bind:this={form} class="mt-8" onsubmit={submit}>
      <label for="pair-code" class="ml-1 text-[11.5px] font-semibold text-muted">Pairing code</label>
      <input
        id="pair-code"
        bind:this={input}
        bind:value={code}
        oninput={onInput}
        inputmode="numeric"
        autocomplete="one-time-code"
        autocapitalize="off"
        spellcheck="false"
        enterkeyhint="done"
        maxlength="6"
        placeholder="000000"
        class="mt-2 h-16 w-full rounded-2xl border border-line-strong bg-surface-2 px-4 text-center font-mono text-[30px] font-semibold tracking-[.3em] text-ink caret-accent placeholder:text-faint/60 focus-visible:border-transparent focus-visible:outline-3 focus-visible:outline-accent"
      />
      <button
        type="submit"
        disabled={session.pairingBusy}
        aria-busy={session.pairingBusy}
        class="mt-3 flex h-13 w-full items-center justify-center rounded-2xl bg-accent text-[14px] font-semibold text-accent-ink transition active:scale-[.985] disabled:opacity-60"
      >
        {session.pairingBusy ? "Pairing…" : "Pair web remote"}
      </button>
    </form>

    <p class="mt-3 min-h-5 text-[12px] leading-relaxed text-danger" role="alert" aria-live="polite">
      {session.pairNotice}
    </p>

    <p class="mt-6 max-w-[38ch] text-[11.5px] leading-relaxed text-faint">
      This pairs only the web remote; a Display Box stays connected. After pairing you can add Luma to your home screen.
    </p>
  </div>
</main>
