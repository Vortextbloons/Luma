<script lang="ts">
  import { getVersion, getName } from "@tauri-apps/api/app";

  let name = $state("");
  let greeting = $state("");
  let pending = $state(false);
  let error = $state("");
  let appName = $state("Luma");
  let version = $state("");

  $effect(() => {
    getName()
      .then((value) => (appName = value))
      .catch(() => {
        /* Not running inside Tauri (e.g. plain `npm run dev` in a browser). */
      });
    getVersion()
      .then((value) => (version = value))
      .catch(() => {});
  });

  async function greet() {
    const trimmed = name.trim();
    if (!trimmed || pending) return;

    pending = true;
    error = "";
    try {
      greeting = await invokeGreet(trimmed);
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause);
    } finally {
      pending = false;
    }
  }

  // Imported lazily so the browser preview doesn't fail on the Tauri bridge.
  async function invokeGreet(value: string): Promise<string> {
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<string>("greet", { name: value });
  }
</script>

<main class="relative flex h-screen flex-col overflow-hidden bg-neutral-950">
  <!-- Ambient gradient wash -->
  <div
    class="pointer-events-none absolute inset-x-0 -top-40 h-96 bg-[radial-gradient(ellipse_at_center,rgba(129,140,248,0.18),transparent_70%)]"
    aria-hidden="true"
  ></div>

  <header class="relative z-10 flex items-center gap-3 px-6 py-5">
    <div
      class="flex size-9 items-center justify-center rounded-xl bg-gradient-to-br from-indigo-400 to-fuchsia-500 text-sm font-semibold text-white shadow-lg shadow-indigo-500/20"
    >
      L
    </div>
    <div class="flex flex-col leading-tight">
      <span class="text-sm font-semibold tracking-tight">{appName}</span>
      <span class="text-xs text-neutral-500">
        {version ? `v${version}` : "Tauri 2 · Svelte 5 · Tailwind 4"}
      </span>
    </div>
  </header>

  <section class="relative z-10 flex flex-1 items-center justify-center px-6 pb-16">
    <div
      class="w-full max-w-md rounded-2xl border border-white/10 bg-white/[0.03] p-6 shadow-2xl shadow-black/40 backdrop-blur"
    >
      <h1 class="text-lg font-semibold tracking-tight">Rust, wired up.</h1>
      <p class="mt-1.5 text-sm leading-relaxed text-neutral-400">
        This card calls the <code class="font-mono text-xs text-indigo-300">greet</code>
        command in <code class="font-mono text-xs text-indigo-300">src-tauri/src/lib.rs</code>.
      </p>

      <form class="mt-5 flex gap-2" onsubmit={(event) => (event.preventDefault(), greet())}>
        <input
          bind:value={name}
          placeholder="Your name"
          autocomplete="off"
          spellcheck="false"
          class="min-w-0 flex-1 rounded-lg border border-white/10 bg-neutral-900/80 px-3 py-2 text-sm text-neutral-100 placeholder:text-neutral-600 outline-none transition focus:border-indigo-400/60 focus:ring-2 focus:ring-indigo-400/20"
        />
        <button
          type="submit"
          disabled={pending || !name.trim()}
          class="rounded-lg bg-indigo-500 px-4 py-2 text-sm font-medium text-white transition hover:bg-indigo-400 disabled:cursor-not-allowed disabled:opacity-40"
        >
          {pending ? "…" : "Greet"}
        </button>
      </form>

      {#if greeting}
        <p
          class="mt-4 rounded-lg border border-emerald-400/20 bg-emerald-400/10 px-3 py-2 text-sm text-emerald-200"
        >
          {greeting}
        </p>
      {/if}

      {#if error}
        <p
          class="mt-4 rounded-lg border border-red-400/20 bg-red-400/10 px-3 py-2 text-sm text-red-200"
        >
          {error}
        </p>
      {/if}
    </div>
  </section>
</main>
