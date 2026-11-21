<script lang="ts">
  import { onMount } from "svelte";
  import {
    bridge,
    toBridgeError,
    type Identity,
    type SettingValue,
  } from "@luma/plugin-sdk";

  let identity = $state<Identity | null>(null);
  let greeting = $state("Hello from Luma");
  let busy = $state(false);
  let failed = $state(false);
  let statusText = $state("Starting the package surface.");

  let refreshButton: HTMLButtonElement | null = $state(null);
  let closeButton: HTMLButtonElement | null = $state(null);

  function messageOf(cause: unknown): string {
    return toBridgeError(cause).message;
  }

  function greetingFrom(values: Record<string, SettingValue>): string {
    const value = values.greeting;
    return typeof value === "string" && value.trim() ? value : "Hello from Luma";
  }

  async function refreshGreeting() {
    if (busy) return;
    busy = true;
    try {
      const snapshot = await bridge.readSettings();
      greeting = greetingFrom(snapshot.values);
      failed = false;
      statusText = "Settings refreshed from Luma.";
    } catch (cause) {
      failed = true;
      statusText = `Could not read this package's settings: ${messageOf(cause)}`;
    } finally {
      busy = false;
    }
  }

  function exit() {
    bridge.close().catch(cause => {
      statusText = `Could not close this surface: ${messageOf(cause)}`;
    });
  }

  function focusables(): HTMLButtonElement[] {
    const list: HTMLButtonElement[] = [];
    if (refreshButton) list.push(refreshButton);
    if (closeButton) list.push(closeButton);
    return list;
  }

  function moveFocus(step: number) {
    const items = focusables();
    if (!items.length) return;
    const current = items.indexOf(document.activeElement as HTMLButtonElement);
    const next =
      current === -1
        ? step > 0
          ? 0
          : items.length - 1
        : (current + step + items.length) % items.length;
    items[next].focus();
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === "Escape" || event.key === "BrowserBack") {
      event.preventDefault();
      exit();
      return;
    }
    const step =
      event.key === "ArrowRight" || event.key === "ArrowDown"
        ? 1
        : event.key === "ArrowLeft" || event.key === "ArrowUp"
          ? -1
          : 0;
    if (step === 0) return;
    event.preventDefault();
    moveFocus(step);
  }

  onMount(() => {
    window.addEventListener("keydown", handleKeydown);
    history.pushState({ surface: "starter" }, "");
    window.addEventListener("popstate", exit);

    void initialize();

    return () => {
      window.removeEventListener("keydown", handleKeydown);
      window.removeEventListener("popstate", exit);
    };
  });

  async function initialize() {
    refreshButton?.focus();

    const [identityResult, settingsResult] = await Promise.allSettled([
      bridge.identity(),
      bridge.readSettings(),
    ]);

    if (identityResult.status === "fulfilled") {
      identity = identityResult.value;
    } else {
      failed = true;
      statusText = `Identity unavailable: ${messageOf(identityResult.reason)}`;
    }

    if (settingsResult.status === "fulfilled") {
      greeting = greetingFrom(settingsResult.value.values);
    } else {
      failed = true;
      statusText = `Could not read this package's settings: ${messageOf(settingsResult.reason)}`;
    }

    try {
      await bridge.ready();
      if (!failed) {
        statusText = "Ready. Press Refresh greeting to read settings again.";
      }
    } catch (cause) {
      failed = true;
      statusText = `This surface could not start: ${messageOf(cause)}`;
    }
  }
</script>

<main
  class="flex flex-col gap-7 w-full max-w-[640px] m-auto p-9 rounded-3xl border border-white/12 bg-[rgb(16_19_30/88%)] shadow-[0_30px_80px_rgb(0_0_0/45%)]"
  data-state={failed ? "error" : busy ? "busy" : "ready"}
>
  <header>
    <p class="m-0 mb-2 text-[#a69bff] text-xs font-bold tracking-[0.14em] uppercase">Plugin starter</p>
    <h1 class="m-0 text-4xl sm:text-5xl font-semibold leading-tight text-white">{greeting}</h1>
  </header>

  <p
    class="min-h-12 m-0 text-base leading-relaxed text-[#b9bac7]"
    class:text-[#ff9c8a]={failed}
    role="status"
  >
    {statusText}
  </p>

  <div class="flex flex-wrap gap-3">
    <button
      bind:this={refreshButton}
      type="button"
      class="min-h-12 px-5 rounded-full border border-transparent bg-[#8b7bff] text-[#090b12] font-semibold cursor-pointer focus-visible:outline-3 focus-visible:outline-[#ffd479] focus-visible:outline-offset-3 aria-disabled:opacity-70 aria-disabled:cursor-default"
      aria-busy={busy}
      aria-disabled={busy}
      onclick={refreshGreeting}
    >
      {busy ? "Refreshing…" : "Refresh greeting"}
    </button>
    <button
      bind:this={closeButton}
      type="button"
      class="min-h-12 px-5 rounded-full border border-white/18 bg-white/7 text-inherit font-semibold cursor-pointer hover:bg-white/12 focus-visible:outline-3 focus-visible:outline-[#ffd479] focus-visible:outline-offset-3"
      onclick={exit}
    >
      Close
    </button>
  </div>

  <footer class="border-t border-white/10 pt-4">
    <p class="m-0 text-[#777b8e] text-xs">
      {#if identity}
        {identity.id} {identity.version} · API {identity.apiVersion}
      {/if}
    </p>
  </footer>
</main>
