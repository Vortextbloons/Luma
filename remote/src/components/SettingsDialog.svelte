<script lang="ts">
  import { tick } from "svelte";
  import AppOrderDialog from "./AppOrderDialog.svelte";
  import Icon from "./Icon.svelte";
  import Modal from "./Modal.svelte";
  import { api } from "../lib/api";
  import { themeColors, themeLabel } from "../lib/appearance";
  import { control, setControlMode } from "../lib/control.svelte";
  import { installEnvironment, installSteps, isStandalone } from "../lib/install";
  import { disconnect, send, session, setTheme } from "../lib/session.svelte";
  import { say } from "../lib/toast.svelte";
  import type { SettingChoice } from "../lib/types";

  let { open = false, onclose }: { open?: boolean; onclose: () => void } = $props();

  type Category = "controls" | "browser" | "connection" | "display" | "theme" | "sound" | "order" | "pc";

  const categories: {
    id: Category;
    group: string;
    title: string;
    blurb: string;
    heading: string;
    description: string;
  }[] = [
    { id: "controls", group: "This remote", title: "Controls", blurb: "Touchpad or directional pad", heading: "Controls", description: "Choose how this phone's pad drives Luma. Other remotes keep their own setting." },
    { id: "browser", group: "This remote", title: "This browser", blurb: "Theme and fullscreen", heading: "This browser", description: "Appearance and window options saved on this device." },
    { id: "connection", group: "This remote", title: "Connection", blurb: "Pairing and this session", heading: "Connection", description: "The link between this browser and your PC." },
    { id: "display", group: "Luma app", title: "Display", blurb: "Cards and spacing", heading: "Display", description: "Choose how cards and rows fill the TV. Changes apply to every connected display." },
    { id: "theme", group: "Luma app", title: "Theme", blurb: "Background and accent", heading: "Theme", description: "The background, surfaces, and accent used across Luma." },
    { id: "sound", group: "Luma app", title: "Sound", blurb: "Interface sounds and level", heading: "Sound", description: "Short feedback for focus, select, back, and app launch." },
    { id: "order", group: "Luma app", title: "App order", blurb: "Arrange the dock everywhere", heading: "App order", description: "Arrange the app dock on the TV, the Display Box, and this remote." },
    { id: "pc", group: "Luma app", title: "PC actions", blurb: "Fullscreen on the PC", heading: "PC actions", description: "Control the Luma window running on your PC." },
  ];

  const groups: { label: string; ids: Category[] }[] = [
    { label: "This remote", ids: ["controls", "browser", "connection"] },
    { label: "Luma app", ids: ["display", "theme", "sound", "order", "pc"] },
  ];

  let category = $state<Category>("browser");
  let detail = $state(false);
  let narrow = $state(false);
  let themeOpen = $state(false);
  let themePicker = $state<HTMLDivElement>();
  let themeButton = $state<HTMLButtonElement>();
  let orderOpen = $state(false);
  let fullscreenSupported = $state(false);
  let fullscreenActive = $state(false);
  let settingsSaving = $state(false);
  let settingsNote = $state("");
  let settingsTone = $state<"neutral" | "good" | "bad">("neutral");
  let drafts = $state<Record<string, string>>({});
  let disconnectBusy = $state(false);
  let disconnectNotice = $state("");
  let installNote = $state("");
  let wasOpen = false;

  let active = $derived(categories.find(item => item.id === category) ?? categories[0]);
  let appearance = $derived(session.state?.appearance);
  let themeName = $derived(themeLabel(appearance, session.themeChoice));
  let stateSettings = $derived(session.state?.settings ?? []);
  let fullscreenLabel = $derived(!fullscreenSupported ? "Unavailable" : fullscreenActive ? "Exit" : "Enter");
  let fullscreenNote = $derived(
    !fullscreenSupported
      ? "This browser cannot enlarge the remote."
      : fullscreenActive
        ? "This browser is fullscreen."
        : "Fills this browser window. The PC screen is not affected.",
  );
  let pcFullscreen = $derived(session.state?.windowFullscreen);
  let pcLabel = $derived(pcFullscreen === true ? "Exit" : pcFullscreen === false ? "Fullscreen" : "Toggle");

  function settingsFor(group: string): SettingChoice[] {
    return stateSettings.filter(item => String(item.group).toLowerCase() === group);
  }

  function categoryClass(id: Category): string {
    return `settings-category ${category === id ? "is-current" : ""}`;
  }

  function selectCategory(id: Category): void {
    category = id;
    themeOpen = false;
    settingsNote = "";
    settingsTone = "neutral";
    if (narrow) {
      detail = true;
      void tick().then(() => document.querySelector<HTMLElement>(`#settings-dialog h2[data-panel="${id}"]`)?.focus());
    }
  }

  function back(): void {
    detail = false;
    void tick().then(() => document.querySelector<HTMLElement>(`#settings-dialog [data-category="${category}"]`)?.focus());
  }

  function requestClose(): void {
    if (narrow && detail) {
      back();
      return;
    }
    onclose();
  }

  function openTheme(): void {
    if (!themeOpen) themeOpen = true;
  }

  function closeTheme(focus = true): void {
    themeOpen = false;
    if (focus) themeButton?.focus();
  }

  function selectTheme(value: string): void {
    setTheme(value);
    closeTheme(true);
  }

  function themeKeydown(event: KeyboardEvent): void {
    if (event.key === "ArrowDown" || event.key === "ArrowUp") {
      event.preventDefault();
      openTheme();
    } else if (event.key === "Escape") {
      closeTheme(false);
    }
  }

  function themeMenuKeydown(event: KeyboardEvent): void {
    const options = [...(themePicker?.querySelectorAll<HTMLElement>('[role="option"]') ?? [])];
    const index = options.indexOf(document.activeElement as HTMLElement);
    if (event.key === "ArrowDown" || event.key === "ArrowUp") {
      event.preventDefault();
      if (!options.length) return;
      const next = event.key === "ArrowDown"
        ? (index + 1 + options.length) % options.length
        : (index - 1 + options.length) % options.length;
      options[next]?.focus();
    } else if (event.key === "Escape") {
      event.preventDefault();
      closeTheme(true);
    } else if (event.key === "Tab") {
      themeOpen = false;
    }
  }

  async function saveSetting(choice: SettingChoice, value: string): Promise<void> {
    if (settingsSaving || value === choice.value) return;
    settingsSaving = true;
    settingsNote = "Saving…";
    settingsTone = "neutral";
    drafts = { ...drafts, [choice.key]: value };
    try {
      const { response } = await api.settings(choice.key, value);
      if (!response.ok) throw new Error();
      if (session.state) {
        session.state.settings = session.state.settings.map(item =>
          item.key === choice.key ? { ...item, value } : item,
        );
      }
      settingsNote = "Change sent to Luma.";
      settingsTone = "good";
    } catch (_) {
      drafts = { ...drafts, [choice.key]: choice.value };
      settingsNote = "Could not save. Check the connection and try again.";
      settingsTone = "bad";
    } finally {
      settingsSaving = false;
    }
  }

  async function toggleRemoteFullscreen(): Promise<void> {
    if (!fullscreenSupported) return;
    try {
      if (document.fullscreenElement) await document.exitFullscreen();
      else await document.documentElement.requestFullscreen();
    } catch (_) {
      say("Remote fullscreen is unavailable in this browser.", true);
    }
  }

  async function togglePcFullscreen(): Promise<void> {
    const result = await send("fullscreen");
    if (!result.accepted) return;
    const state = result.state ?? session.state;
    const name = state?.window ?? "The PC window";
    const value = state?.windowFullscreen;
    say(value === true
      ? `${name} is fullscreen.`
      : value === false
        ? `${name} is windowed.`
        : "Fullscreen command sent to the PC app.");
  }

  async function disconnectRemote(): Promise<void> {
    if (disconnectBusy) return;
    disconnectBusy = true;
    disconnectNotice = "";
    const error = await disconnect();
    if (error) {
      disconnectNotice = error;
      disconnectBusy = false;
    }
  }

  function showInstallSteps(): void {
    installNote = isStandalone()
      ? "This remote is already running from your home screen."
      : installSteps(installEnvironment());
  }

  function closeOrder(): void {
    orderOpen = false;
    void tick().then(() => document.querySelector<HTMLElement>("[data-order-trigger]")?.focus());
  }

  function dotStyle(colors: { background: string; accent: string }): string {
    return `background:${colors.background};border-color:${colors.accent}`;
  }

  $effect(() => {
    const media = window.matchMedia("(max-width: 719px)");
    const update = () => (narrow = media.matches);
    update();
    media.addEventListener("change", update);
    return () => media.removeEventListener("change", update);
  });

  $effect(() => {
    if (open && !wasOpen) {
      wasOpen = true;
      detail = false;
      void tick().then(() => document.querySelector<HTMLElement>(`#settings-dialog [data-category="${category}"]`)?.focus());
    } else if (!open) {
      wasOpen = false;
      themeOpen = false;
    }
  });

  $effect(() => {
    const choices = session.state?.settings ?? [];
    if (settingsSaving) return;
    const next: Record<string, string> = {};
    for (const choice of choices) next[choice.key] = choice.value;
    drafts = next;
  });

  $effect(() => {
    fullscreenSupported = typeof document.documentElement.requestFullscreen === "function" && document.fullscreenEnabled !== false;
    const sync = () => (fullscreenActive = Boolean(document.fullscreenElement));
    sync();
    document.addEventListener("fullscreenchange", sync);
    return () => document.removeEventListener("fullscreenchange", sync);
  });

  $effect(() => {
    if (!themeOpen) return;
    void tick().then(() => themePicker?.querySelector<HTMLElement>('[role="option"]')?.focus());
    const onPointer = (event: PointerEvent) => {
      if (event.target instanceof Node && themePicker?.contains(event.target)) return;
      themeOpen = false;
    };
    document.addEventListener("pointerdown", onPointer);
    return () => document.removeEventListener("pointerdown", onPointer);
  });
</script>

<Modal
  id="settings-dialog"
  titleId="settings-title"
  {open}
  className="settings-dialog"
  onrequestclose={requestClose}
>
  <header class="settings-head">
    <div>
      <h2 id="settings-title">Settings</h2>
      <p>This remote and your Luma app</p>
    </div>
    <button type="button" class="settings-close" aria-label="Close settings" onclick={onclose}>
      <Icon name="close" className="size-[15px]" />
    </button>
  </header>

  <div class="settings-layout" class:is-detail={detail}>
    <nav class="settings-nav" aria-label="Settings categories">
      {#each groups as group (group.label)}
        <p class="nav-group">{group.label}</p>
        {#each group.ids as id (id)}
          {@const item = categories.find(entry => entry.id === id)!}
          <button
            type="button"
            class={categoryClass(id)}
            data-category={id}
            aria-current={category === id ? "page" : undefined}
            onclick={() => selectCategory(id)}
          >
            <span class="category-copy">
              <b>{item.title}</b>
              <small>{item.blurb}</small>
            </span>
            <Icon name="right" className="category-mark size-4" />
          </button>
        {/each}
      {/each}
    </nav>

    <div class="settings-content">
      <div class="panel-head">
        <button type="button" class="settings-back" onclick={back}>
          <Icon name="back" className="size-[14px]" />
          All settings
        </button>
        <span class="panel-tag">{active.group}</span>
        <h2 data-panel={category} tabindex="-1">{active.heading}</h2>
        <p>{active.description}</p>
      </div>

      {#if category === "controls"}
        <section class="group">
          <h3>Pointer</h3>
          <div class="row">
            <span class="row-copy">
              <b>Touchpad mode</b>
              <small>Drag the pad to move the PC pointer and tap to click. Leave this off for directional keys on Luma and TV-style apps.</small>
            </span>
            <button
              type="button"
              role="switch"
              aria-checked={control.mode === "touchpad"}
              aria-label="Touchpad mode"
              class={`relative h-7 w-12 flex-none rounded-full border transition ${control.mode === "touchpad" ? "border-transparent bg-accent" : "border-line-strong bg-white/[0.12]"}`}
              onclick={() => setControlMode(control.mode === "touchpad" ? "dpad" : "touchpad")}
            >
              <span class={`absolute left-0.5 top-0.5 size-6 rounded-full shadow-[0_1px_4px_rgba(0,0,0,.45)] transition-transform ${control.mode === "touchpad" ? "translate-x-5 bg-accent-ink" : "bg-ink"}`}></span>
            </button>
          </div>
          <p class="row-note">This setting is saved on this phone. Touchpad mode is for desktop-style apps; the D-pad is the default for Luma.</p>
        </section>
      {/if}

      {#if category === "browser"}
        <section class="group">
          <h3>Appearance</h3>
          <div class="row">
            <span class="row-copy">
              <b>Theme</b>
              <small>Follow the TV or keep a theme on this phone.</small>
            </span>
            <div class="theme-picker" bind:this={themePicker}>
              <button
                bind:this={themeButton}
                type="button"
                class="theme-select"
                aria-haspopup="listbox"
                aria-expanded={themeOpen}
                onclick={() => (themeOpen ? closeTheme(true) : openTheme())}
                onkeydown={themeKeydown}
              >
                <span class="truncate">{themeName}</span>
                <Icon name="chevron-down" className="size-[14px]" />
              </button>
              {#if themeOpen}
                <div class="theme-menu" role="listbox" aria-label="Remote theme" tabindex="-1" onkeydown={themeMenuKeydown}>
                  <button type="button" role="option" class="theme-option" aria-selected={session.themeChoice === "follow"} onclick={() => selectTheme("follow")}>
                    <span class="theme-dot" style="background:var(--surface);border-color:var(--accent)"></span>
                    <span>Follow TV</span>
                  </button>
                  {#each appearance?.themes ?? [] as theme (theme.id)}
                    <button type="button" role="option" class="theme-option" aria-selected={session.themeChoice === theme.id} onclick={() => selectTheme(theme.id)}>
                      <span class="theme-dot" style={dotStyle(themeColors(theme))}></span>
                      <span class="truncate">{theme.name}</span>
                    </button>
                  {/each}
                </div>
              {/if}
            </div>
          </div>
        </section>

        <section class="group">
          <h3>Window</h3>
          <div class="row">
            <span class="row-copy">
              <b>Fullscreen</b>
              <small>{fullscreenNote}</small>
            </span>
            <button
              type="button"
              class="pill"
              aria-pressed={fullscreenActive}
              disabled={!fullscreenSupported}
              onclick={toggleRemoteFullscreen}
            >
              {fullscreenLabel}
            </button>
          </div>
        </section>
      {/if}

      {#if category === "connection"}
        <section class="group">
          <h3>Current session</h3>
          <div class="row">
            <span class="row-copy">
              <b>Connection</b>
              <small>{session.online ? `Connected to ${session.pcName}` : "Waiting for the PC"}</small>
            </span>
            <span class={`status-dot ${session.online ? "is-online" : ""}`} aria-label={session.online ? "Connected" : "Disconnected"}></span>
          </div>
          <div class="row">
            <span class="row-copy">
              <b>Disconnect this browser</b>
              <small>Pair again with a new code from Settings on the PC.</small>
            </span>
            <button
              type="button"
              class="pill danger"
              disabled={disconnectBusy}
              aria-busy={disconnectBusy}
              onclick={disconnectRemote}
            >
              {disconnectBusy ? "Disconnecting…" : "Disconnect"}
            </button>
          </div>
          {#if disconnectNotice}<p class="row-notice" role="alert">{disconnectNotice}</p>{/if}
        </section>

        <section class="group">
          <h3>Home screen</h3>
          <div class="row">
            <span class="row-copy">
              <b>Add to Home Screen</b>
              <small>Open Luma without scanning the QR code again.</small>
            </span>
            <button type="button" class="pill" onclick={showInstallSteps}>Show steps</button>
          </div>
          {#if installNote}<p class="row-note" role="status">{installNote}</p>{/if}
        </section>
      {/if}

      {#if category === "display" || category === "theme" || category === "sound"}
        <section class="group">
          <h3>{category === "display" ? "Cards and layout" : category === "theme" ? "App theme" : "Interface sounds"}</h3>
          {#if settingsNote}
            <p class="settings-note" data-tone={settingsTone} role="status" aria-live="polite">{settingsNote}</p>
          {/if}
          {#if settingsFor(category).length}
            {#each settingsFor(category) as choice (choice.key)}
              <label class="field" for={`setting-${choice.key}`}>
                <span class="row-copy">
                  <b>{choice.label}</b>
                  <small>{choice.description}</small>
                </span>
                <select
                  id={`setting-${choice.key}`}
                  value={drafts[choice.key] ?? choice.value}
                  disabled={settingsSaving}
                  onchange={event => saveSetting(choice, event.currentTarget.value)}
                >
                  {#each choice.options as option (option.value)}
                    <option value={option.value}>{option.label}</option>
                  {/each}
                </select>
              </label>
            {/each}
          {:else}
            <p class="row-note">{session.state ? "No settings are available for this section." : "Connecting to Luma…"}</p>
          {/if}
        </section>
      {/if}

      {#if category === "order"}
        <section class="group">
          <h3>Dock</h3>
          <div class="row">
            <span class="row-copy">
              <b>App order</b>
              <small>Move apps and choose which ones appear.</small>
            </span>
            <button type="button" class="pill" data-order-trigger onclick={() => (orderOpen = true)}>Edit</button>
          </div>
        </section>
      {/if}

      {#if category === "pc"}
        <section class="group">
          <h3>Window</h3>
          <div class="row">
            <span class="row-copy">
              <b>Fullscreen app</b>
              <small>Toggles full screen on the app Luma is controlling.</small>
            </span>
            <button
              type="button"
              class="pill"
              aria-pressed={pcFullscreen === true}
              aria-label="Toggle fullscreen for the PC window"
              onclick={togglePcFullscreen}
            >
              {pcLabel}
            </button>
          </div>
        </section>
      {/if}
    </div>
  </div>

  <AppOrderDialog open={orderOpen} onclose={closeOrder} />
</Modal>

<style>
  :global(.settings-dialog) {
    width: min(940px, calc(100% - 32px));
    height: min(88dvh, 780px);
    max-height: min(88dvh, 780px);
    margin: auto;
    padding: 0;
    overflow: hidden;
    border: 1px solid var(--line-strong);
    border-radius: 24px;
    background: linear-gradient(180deg, var(--surface), var(--surface-2));
    color: var(--text);
    box-shadow: 0 30px 80px rgba(0, 0, 0, .55);
  }

  :global(.settings-dialog[open]) {
    display: flex;
    flex-direction: column;
  }

  .settings-head {
    display: flex;
    flex: none;
    align-items: center;
    justify-content: space-between;
    gap: 14px;
    padding: 20px 22px 14px;
    border-bottom: 1px solid var(--line);
  }

  .settings-head h2 {
    margin: 0;
    font-size: 20px;
    font-weight: 700;
    letter-spacing: -.02em;
  }

  .settings-head p {
    margin: 4px 0 0;
    color: var(--faint);
    font-size: 12px;
  }

  .settings-close {
    display: grid;
    place-items: center;
    width: 40px;
    height: 40px;
    flex: none;
    border: 1px solid var(--line);
    border-radius: 50%;
    background: rgba(255, 255, 255, .05);
    color: var(--muted);
    transition: background .12s ease, color .12s ease;
  }

  .settings-close:active {
    background: rgba(255, 255, 255, .12);
    color: var(--text);
  }

  .settings-layout {
    display: grid;
    flex: 1;
    min-height: 0;
    grid-template-columns: 238px minmax(0, 1fr);
  }

  .settings-nav {
    display: flex;
    flex-direction: column;
    gap: 3px;
    overflow-y: auto;
    overscroll-behavior: contain;
    padding: 14px 12px 18px;
    border-right: 1px solid var(--line);
    background: color-mix(in srgb, var(--surface-2) 55%, transparent);
  }

  .nav-group {
    margin: 14px 10px 3px;
    color: var(--faint);
    font-size: 10.5px;
    font-weight: 700;
  }

  .nav-group:first-child {
    margin-top: 2px;
  }

  .settings-category {
    position: relative;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    width: 100%;
    min-height: 54px;
    padding: 10px 12px;
    border: 1px solid transparent;
    border-radius: 12px;
    background: transparent;
    color: var(--muted);
    text-align: left;
    transition: background .16s ease, border-color .16s ease, color .16s ease;
  }

  .settings-category::before {
    content: "";
    position: absolute;
    top: 20%;
    bottom: 20%;
    left: 0;
    width: 3px;
    border-radius: 99px;
    background: transparent;
    transition: background .16s ease, box-shadow .16s ease;
  }

  .category-copy {
    display: grid;
    gap: 2px;
    min-width: 0;
  }

  .category-copy b {
    color: var(--text-dim);
    font-size: 13.5px;
    font-weight: 650;
    letter-spacing: -.01em;
  }

  .category-copy small {
    color: var(--muted);
    font-size: 11px;
    line-height: 1.4;
  }

  :global(.category-mark) {
    color: var(--faint);
    opacity: 0;
    transform: translateX(-4px);
    transition: opacity .16s ease, transform .16s ease;
  }

  .settings-category:hover {
    border-color: var(--line);
    background: rgba(255, 255, 255, .05);
  }

  .settings-category:hover :global(.category-mark) {
    opacity: .7;
    transform: none;
  }

  .settings-category.is-current {
    border-color: var(--line-strong);
    background: linear-gradient(90deg, var(--accent-soft), rgba(255, 255, 255, .04) 62%, transparent);
    color: var(--text);
    box-shadow: inset 0 1px rgba(255, 255, 255, .08);
  }

  .settings-category.is-current::before {
    background: var(--accent);
    box-shadow: 0 0 12px var(--accent);
  }

  .settings-category.is-current .category-copy b { color: var(--text); }
  .settings-category.is-current .category-copy small { color: var(--text-dim); }
  .settings-category.is-current :global(.category-mark) { opacity: 1; transform: none; color: var(--accent); }

  .settings-content {
    min-width: 0;
    min-height: 0;
    overflow-y: auto;
    overscroll-behavior: contain;
    padding: 18px 26px 26px;
    scroll-padding-top: 16px;
  }

  .panel-head {
    display: grid;
    gap: 6px;
    padding-bottom: 14px;
    border-bottom: 1px solid var(--line);
  }

  .panel-tag {
    width: max-content;
    padding: 4px 9px;
    border: 1px solid var(--line-strong);
    border-radius: 99px;
    background: rgba(255, 255, 255, .05);
    color: var(--faint);
    font-size: 10px;
    font-weight: 700;
  }

  .panel-head h2 {
    margin: 0;
    font-size: clamp(24px, 3.4vw, 30px);
    font-weight: 650;
    letter-spacing: -.035em;
    line-height: 1.05;
  }

  .panel-head h2:focus { outline: none; }

  .panel-head p {
    max-width: 62ch;
    margin: 0;
    color: var(--muted);
    font-size: 12.5px;
    line-height: 1.55;
  }

  .settings-back {
    display: none;
    align-items: center;
    gap: 6px;
    width: max-content;
    min-height: 36px;
    margin-bottom: 4px;
    padding: 0 11px 0 8px;
    border: 1px solid var(--line);
    border-radius: 99px;
    background: rgba(255, 255, 255, .05);
    color: var(--text-dim);
    font-size: 12px;
    font-weight: 650;
  }

  .group {
    margin-top: 14px;
    padding: 16px 16px 6px;
    border: 1px solid var(--line);
    border-radius: 16px;
    background: linear-gradient(180deg, rgba(255, 255, 255, .035), rgba(255, 255, 255, .012));
    box-shadow: inset 0 1px rgba(255, 255, 255, .04);
  }

  .group h3 {
    margin: 0 0 2px;
    padding: 0 2px;
    color: var(--faint);
    font-size: 10.5px;
    font-weight: 700;
  }

  .row {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    align-items: center;
    gap: 16px;
    padding: 12px 2px;
  }

  .row + .row,
  .field + .field {
    border-top: 1px solid var(--line);
  }

  .row-copy {
    display: grid;
    gap: 3px;
    min-width: 0;
  }

  .row-copy b {
    color: var(--text);
    font-size: 14px;
    font-weight: 650;
    letter-spacing: -.01em;
  }

  .row-copy small {
    color: var(--muted);
    font-size: 12px;
    line-height: 1.45;
  }

  .row-notice {
    margin: 0 2px 8px;
    color: var(--danger);
    font-size: 11.5px;
    line-height: 1.4;
  }

  .row-note {
    margin: 0 2px 10px;
    color: var(--muted);
    font-size: 11.5px;
    line-height: 1.45;
  }

  .settings-note {
    margin: 8px 0 4px;
    padding: 9px 12px;
    border: 1px solid var(--line-strong);
    border-radius: 10px;
    background: color-mix(in srgb, var(--surface-3) 90%, transparent);
    color: var(--text-dim);
    font-size: 11.5px;
    line-height: 1.4;
  }

  .settings-note[data-tone="good"] {
    border-color: color-mix(in srgb, var(--good) 30%, transparent);
    color: var(--good);
  }

  .settings-note[data-tone="bad"] {
    border-color: color-mix(in srgb, var(--danger) 30%, transparent);
    color: var(--danger);
  }

  .field {
    display: grid;
    gap: 7px;
    padding: 12px 2px;
  }

  .field select {
    width: 100%;
    min-height: 44px;
    padding: 10px 12px;
    border: 1px solid var(--line-strong);
    border-radius: 10px;
    background: var(--surface-2);
    color: var(--text);
    font-size: 13px;
    font-weight: 600;
  }

  .pill {
    min-height: 38px;
    padding: 0 16px;
    border: 1px solid var(--line-strong);
    border-radius: 99px;
    background: rgba(255, 255, 255, .07);
    color: var(--text);
    font-size: 12.5px;
    font-weight: 650;
    white-space: nowrap;
    transition: background .14s ease, transform .08s ease;
  }

  .pill:active {
    background: rgba(255, 255, 255, .13);
    transform: scale(.96);
  }

  .pill:disabled {
    opacity: .5;
  }

  .pill.danger {
    border-color: color-mix(in srgb, var(--danger) 26%, transparent);
    background: color-mix(in srgb, var(--danger) 8%, transparent);
    color: var(--danger);
  }

  .status-dot {
    width: 10px;
    height: 10px;
    border-radius: 50%;
    background: var(--faint);
  }

  .status-dot.is-online {
    background: var(--good);
    box-shadow: 0 0 10px color-mix(in srgb, var(--good) 45%, transparent);
  }

  .theme-picker {
    position: relative;
    flex: none;
  }

  .theme-select {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    min-width: 150px;
    max-width: 190px;
    min-height: 40px;
    padding: 0 12px;
    border: 1px solid var(--line-strong);
    border-radius: 11px;
    background: color-mix(in srgb, var(--surface-2) 88%, transparent);
    color: var(--text);
    font-size: 12.5px;
    font-weight: 650;
    text-align: left;
  }

  .theme-select[aria-expanded="true"] {
    border-color: var(--accent);
    background: var(--accent-soft);
  }

  .theme-menu {
    position: absolute;
    top: calc(100% + 6px);
    right: 0;
    z-index: 6;
    width: 214px;
    max-height: 248px;
    overflow-y: auto;
    padding: 5px;
    border: 1px solid var(--line-strong);
    border-radius: 14px;
    background: color-mix(in srgb, var(--surface-2) 98%, transparent);
    box-shadow: 0 18px 42px rgba(0, 0, 0, .62);
  }

  .theme-option {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    min-height: 38px;
    padding: 0 10px;
    border-radius: 9px;
    background: transparent;
    color: var(--text-dim);
    font-size: 12.5px;
    font-weight: 600;
    text-align: left;
  }

  .theme-option:hover,
  .theme-option:focus-visible {
    outline: none;
    background: var(--accent-soft);
    color: var(--text);
  }

  .theme-option[aria-selected="true"] {
    background: color-mix(in srgb, var(--accent) 16%, transparent);
    color: var(--text);
  }

  .theme-dot {
    width: 16px;
    height: 16px;
    flex: none;
    border: 2px solid var(--accent);
    border-radius: 50%;
    background: var(--surface);
  }

  @media (max-width: 719px) {
    :global(.settings-dialog) {
      width: 100%;
      max-width: none;
      height: 100dvh;
      max-height: 100dvh;
      margin: 0;
      padding-top: env(safe-area-inset-top);
      padding-bottom: env(safe-area-inset-bottom);
      border: 0;
      border-radius: 0;
    }

    .settings-head {
      padding: 16px 18px 14px;
    }

    .settings-layout {
      display: flex;
      flex-direction: column;
    }

    .settings-nav {
      flex: 1;
      padding: 8px 14px 24px;
      border-right: 0;
    }

    .settings-content {
      display: none;
      flex: 1;
      padding: 16px 18px 28px;
    }

    .settings-layout.is-detail .settings-nav { display: none; }
    .settings-layout.is-detail .settings-content { display: block; }

    .settings-back { display: inline-flex; }

    .category-copy small,
    :global(.category-mark) {
      display: none;
    }

    .nav-group {
      margin-top: 12px;
    }

    .settings-category {
      min-height: 48px;
    }

    .group {
      padding: 14px 14px 4px;
    }

    .row {
      gap: 12px;
    }

    .theme-select {
      max-width: 150px;
    }
  }
</style>
