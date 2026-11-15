<script lang="ts">
  import { onMount } from "svelte";
  import AppTile from "./AppTile.svelte";
  import Icon from "./Icon.svelte";
  import { apps, loadApps, recentAppsForSession, rememberApp } from "../lib/apps.svelte";
  import { closeApps } from "../lib/screen.svelte";
  import { send } from "../lib/session.svelte";
  import type { AppEntry } from "../lib/types";

  let backButton = $state<HTMLButtonElement>();
  let launching = $state<string | null>(null);

  let recents = $derived(recentAppsForSession());
  let loading = $derived(apps.loading && apps.list.length === 0);
  let failed = $derived(apps.error && apps.list.length === 0);
  let empty = $derived(!apps.loading && !apps.error && apps.attempted && apps.list.length === 0);

  onMount(() => {
    backButton?.focus({ preventScroll: true });
    void loadApps();
  });

  async function launch(app: AppEntry): Promise<void> {
    if (launching) return;
    launching = app.command;
    const result = await send(app.command);
    if (result.accepted) {
      if (!app.shortcut) rememberApp(app.command);
      launching = null;
      closeApps();
      return;
    }
    launching = null;
  }

  function retry(): void {
    apps.error = false;
    void loadApps(true);
  }

  function onKeydown(event: KeyboardEvent): void {
    if (event.key === "Escape") closeApps();
  }
</script>

<svelte:window onkeydown={onKeydown} />

<main class="screen-enter min-h-dvh">
  <header class="apps-head sticky top-0 z-20 border-b border-line">
    <div class="mx-auto flex w-full max-w-[900px] items-center justify-between gap-3 px-4 pb-3 pt-[calc(14px+env(safe-area-inset-top))]">
      <button
        bind:this={backButton}
        type="button"
        class="flex min-h-10 flex-none items-center gap-1.5 rounded-full border border-line bg-white/[0.04] pl-2.5 pr-3.5 text-[12px] font-semibold text-ink-dim transition active:scale-[.97] active:bg-white/[.09]"
        aria-label="Back to remote"
        onclick={closeApps}
      >
        <Icon name="back" className="size-[15px]" />
        Remote
      </button>

      <span class="rounded-full border border-line bg-white/[0.03] px-3 py-1.5 text-[10.5px] font-semibold text-faint">
        {apps.list.length > 0 ? `${apps.list.length} apps` : "Apps"}
      </span>
    </div>
  </header>

  <div class="mx-auto w-full max-w-[900px] px-4 pb-[calc(30px+env(safe-area-inset-bottom))] pt-5">
    <div class="mb-7">
      <h1 class="text-[34px] font-semibold leading-[.98] tracking-[-0.055em] text-ink">Apps</h1>
      <p class="mt-2 max-w-[34ch] text-[13px] leading-relaxed text-muted">Open something on your TV and keep this phone in hand.</p>
    </div>

    {#if apps.shortcuts.length}
      <section class="mb-7" aria-labelledby="sports-shortcuts-title">
        <h2 id="sports-shortcuts-title" class="mb-2.5 text-[12px] font-semibold text-muted">Sports</h2>
        <div class="grid grid-cols-3 gap-2 sm:grid-cols-5">
          {#each apps.shortcuts as app (app.command)}
            <AppTile {app} {launching} disabled={launching !== null} onlaunch={launch} />
          {/each}
        </div>
      </section>
    {/if}
    {#if loading || (!apps.attempted && !failed)}
      <div class="grid grid-cols-3 gap-2 sm:grid-cols-4 lg:grid-cols-5" aria-label="Loading apps" aria-busy="true">
        {#each Array(8) as _}
          <div class="h-[76px] animate-pulse rounded-[18px] border border-line bg-white/[0.025]"></div>
        {/each}
      </div>
    {:else if failed}
      <div class="rounded-2xl border border-danger/25 bg-danger/[0.06] p-5" role="alert">
        <b class="block text-[14px] font-semibold text-ink">Apps are unavailable right now</b>
        <p class="mt-1.5 max-w-[42ch] text-[12.5px] leading-relaxed text-muted">Check the connection to your PC and try again.</p>
        <button
          type="button"
          class="mt-4 min-h-10 rounded-full border border-line-strong bg-white/[0.06] px-4 text-[12px] font-semibold text-ink transition active:scale-[.97]"
          onclick={retry}
        >
          Try again
        </button>
      </div>
    {:else if empty}
      <div class="rounded-2xl border border-line bg-white/[0.025] p-5">
        <b class="block text-[14px] font-semibold text-ink">No apps are available yet</b>
        <p class="mt-1.5 max-w-[42ch] text-[12.5px] leading-relaxed text-muted">Add or arrange apps in Luma on your TV, then come back here.</p>
      </div>
    {:else}
      {#if recents.length}
        <section class="mb-7" aria-labelledby="recent-title">
          <h2 id="recent-title" class="mb-2.5 text-[12px] font-semibold text-muted">Recently used</h2>
          <div class="grid grid-cols-2 gap-2 sm:grid-cols-4">
            {#each recents as app (app.command)}
              <AppTile {app} {launching} disabled={launching !== null} large onlaunch={launch} />
            {/each}
          </div>
        </section>
      {/if}

      <section aria-labelledby="all-apps-title">
        <h2 id="all-apps-title" class="mb-2.5 text-[12px] font-semibold text-muted">All apps</h2>
        <div class="grid grid-cols-3 gap-2 sm:grid-cols-4 lg:grid-cols-5">
          {#each apps.list as app (app.command)}
            <AppTile {app} {launching} disabled={launching !== null} onlaunch={launch} />
          {/each}
        </div>
      </section>
    {/if}
  </div>
</main>

<style>
  .apps-head {
    background: color-mix(in srgb, var(--bg) 86%, transparent);
    backdrop-filter: blur(16px);
  }
</style>
