<script lang="ts">
  import { onMount, tick } from "svelte";
  import Deck from "./Deck.svelte";
  import DPad from "./DPad.svelte";
  import Icon from "./Icon.svelte";
  import InstallHint from "./InstallHint.svelte";
  import MoreMenu from "./MoreMenu.svelte";
  import MusicCard from "./MusicCard.svelte";
  import OfflineBanner from "./OfflineBanner.svelte";
  import PanelsDialog from "./PanelsDialog.svelte";
  import RemoteKey from "./RemoteKey.svelte";
  import SettingsDialog from "./SettingsDialog.svelte";
  import SearchDialog from "./SearchDialog.svelte";
  import TextDialog from "./TextDialog.svelte";
  import TopBar from "./TopBar.svelte";
  import { openApps, openSearch, closeSearch, screen } from "../lib/screen.svelte";
  import { send, session } from "../lib/session.svelte";
  import { say } from "../lib/toast.svelte";

  let settingsOpen = $state(false);
  let panelsOpen = $state(false);
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

  function closePanels(): void {
    panelsOpen = false;
    void tick().then(() => document.querySelector<HTMLElement>("#more-toggle")?.focus());
  }

  $effect(() => {
    if (screen.current === "remote" && screen.restoreSearchFocus) {
      screen.restoreSearchFocus = false;
      void tick().then(() => document.querySelector<HTMLElement>("[data-search-trigger]")?.focus());
    }
  });

  onMount(() => {
    if (screen.restoreAppsFocus) {
      screen.restoreAppsFocus = false;
      void tick().then(() => document.querySelector<HTMLElement>("[data-apps-trigger]")?.focus());
    }
  });
</script>

<div class="remote-shell screen-enter">
  <TopBar
    onsearch={openSearch}
    onapps={openApps}
    ontype={() => (textOpen = true)}
    onsettings={() => (settingsOpen = true)}
  />

  {#if !session.online}
    <OfflineBanner />
  {/if}

  <InstallHint />

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
        <MoreMenu onsend={command} onpanels={() => (panelsOpen = true)} />
      </div>

      <div class="pad-group">
        <DPad onsend={command} />
      </div>
    </div>

    <Deck onsend={command} />
  </main>

  <MusicCard onsend={command} />

  <SettingsDialog open={settingsOpen} onclose={closeSettings} />
  <PanelsDialog open={panelsOpen} onclose={closePanels} />
  <SearchDialog open={screen.current === "search"} onclose={closeSearch} />
  <TextDialog open={textOpen} onclose={closeText} />
</div>

<style>
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
