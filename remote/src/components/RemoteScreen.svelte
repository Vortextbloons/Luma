<script lang="ts">
  import { onMount, tick } from "svelte";
  import Deck from "./Deck.svelte";
  import DPad from "./DPad.svelte";
  import Icon from "./Icon.svelte";
  import InstallHint from "./InstallHint.svelte";
  import MoreMenu from "./MoreMenu.svelte";
  import MusicCard from "./MusicCard.svelte";
  import OfflineBanner from "./OfflineBanner.svelte";
  import RemoteKey from "./RemoteKey.svelte";
  import SettingsDialog from "./SettingsDialog.svelte";
  import TextDialog from "./TextDialog.svelte";
  import TopBar from "./TopBar.svelte";
  import { openApps, screen } from "../lib/screen.svelte";
  import { send, session } from "../lib/session.svelte";
  import { say } from "../lib/toast.svelte";

  let settingsOpen = $state(false);
  let textOpen = $state(false);

  const navKey = "flex min-h-12 items-center justify-center gap-2 rounded-2xl border border-line bg-white/[0.045] text-[13px] font-semibold text-ink-dim transition active:scale-[.97] active:bg-white/[.11]";

  async function command(value: string) {
    const result = await send(value);
    if (!result.accepted) return result;
    if (value === "stop") {
      say("Luma is in the tray. Press Home to return.");
    } else if (value === "fullscreen") {
      const state = result.state ?? session.state;
      const name = state?.window ?? "The PC window";
      const fullscreen = state?.windowFullscreen;
      say(fullscreen === true
        ? `${name} is fullscreen.`
        : fullscreen === false
          ? `${name} is windowed.`
          : "Fullscreen command sent to the PC app.");
    }
    return result;
  }

  function closeSettings(): void {
    settingsOpen = false;
    void tick().then(() => document.querySelector<HTMLElement>("[data-settings-trigger]")?.focus());
  }

  function closeText(): void {
    textOpen = false;
    void tick().then(() => document.querySelector<HTMLElement>("[data-type-trigger]")?.focus());
  }

  onMount(() => {
    if (screen.restoreAppsFocus) {
      screen.restoreAppsFocus = false;
      void tick().then(() => document.querySelector<HTMLElement>("[data-apps-trigger]")?.focus());
    }
  });
</script>

<div class="remote-shell screen-enter">
  <TopBar
    onapps={openApps}
    ontype={() => (textOpen = true)}
    onsettings={() => (settingsOpen = true)}
  />

  {#if !session.online}
    <OfflineBanner />
  {/if}

  <InstallHint />

  <nav class="sports-shortcuts" aria-label="Sports shortcuts">
    {#each [{ command: 'sports', name: 'Sports' }, { command: 'sportsnfl', name: 'NFL' }, { command: 'sportsnba', name: 'NBA' }, { command: 'sportsmlb', name: 'MLB' }, { command: 'sportsnhl', name: 'NHL' }] as shortcut (shortcut.command)}
      <RemoteKey command={shortcut.command} label={`Open ${shortcut.name} on the TV`} className="sports-shortcut" onsend={command}>
        <img src={`/apps/${shortcut.command}/icon`} alt="" width="22" height="22" />
        <span>{shortcut.name}</span>
      </RemoteKey>
    {/each}
  </nav>

  <main class="stage">
    <div class="nav-stack">
      <div class="nav-row" role="group" aria-label="Navigation actions">
        <RemoteKey command="back" className={navKey} onsend={command}>
          <Icon name="back" className="size-[18px]" />
          <span>Back</span>
        </RemoteKey>
        <RemoteKey command="home" className={navKey} onsend={command}>
          <Icon name="home" className="size-[18px]" />
          <span>Home</span>
        </RemoteKey>
        <MoreMenu onsend={command} />
      </div>

      <div class="pad-group">
        <DPad onsend={command} />
      </div>
    </div>

    <Deck onsend={command} />
  </main>

  <MusicCard onsend={command} />

  <SettingsDialog open={settingsOpen} onclose={closeSettings} />
  <TextDialog open={textOpen} onclose={closeText} />
</div>

<style>
  .sports-shortcuts { display: flex; gap: 8px; overflow-x: auto; padding: 3px; scrollbar-width: none; }
  .sports-shortcuts :global(.sports-shortcut) { display: inline-flex; flex: 1; align-items: center; justify-content: center; gap: 6px; min-height: 44px; padding: 8px 12px; border: 1px solid var(--line); border-radius: 12px; background: var(--surface-2); color: var(--text-dim); font-size: 12px; font-weight: 650; white-space: nowrap; }
  .sports-shortcuts img { flex: none; object-fit: contain; }

  .remote-shell {
    --pad: clamp(190px, min(88vw, 46dvh), 380px);
    display: flex;
    flex-direction: column;
    width: min(100%, 460px);
    min-height: 100dvh;
    margin: 0 auto;
    padding:
      calc(14px + env(safe-area-inset-top))
      max(16px, env(safe-area-inset-right))
      calc(14px + env(safe-area-inset-bottom))
      max(16px, env(safe-area-inset-left));
    gap: 12px;
  }

  .stage {
    display: flex;
    flex: 1;
    min-height: 0;
    flex-direction: column;
    gap: 12px;
  }

  .nav-stack {
    display: flex;
    flex: 1;
    min-height: 0;
    flex-direction: column;
    gap: 12px;
  }

  .nav-row {
    display: grid;
    grid-template-columns: 1fr 1fr auto;
    gap: 8px;
    width: min(100%, var(--pad));
    margin-inline: auto;
  }

  .pad-group {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 10px;
    flex: 1;
    justify-content: center;
  }

  @media (min-width: 760px) and (min-height: 520px), (min-width: 560px) and (orientation: landscape) {
    .remote-shell {
      --pad: min(510px, 62dvh, 34vw);
      width: min(100%, 1240px);
      max-width: 1240px;
      padding:
        calc(18px + env(safe-area-inset-top))
        max(32px, env(safe-area-inset-right))
        calc(18px + env(safe-area-inset-bottom))
        max(32px, env(safe-area-inset-left));
      gap: 18px;
    }

    .stage {
      display: grid;
      grid-template-columns: minmax(0, 1fr) minmax(0, .94fr);
      align-items: center;
      gap: clamp(28px, 4vw, 64px);
    }

    .nav-stack {
      align-self: center;
    }

    .nav-row {
      width: 100%;
    }

    .pad-group {
      flex: initial;
    }
  }
</style>
