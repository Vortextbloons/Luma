<script lang="ts">
  import { tick } from "svelte";
  import AppOrderDialog from "./AppOrderDialog.svelte";
  import Icon from "./Icon.svelte";
  import Modal from "./Modal.svelte";
  import SettingSwitch from "./SettingSwitch.svelte";
  import { api } from "../lib/api";
  import { themeColors, themeLabel } from "../lib/appearance";
  import { control, setControlMode } from "../lib/control.svelte";
  import { installEnvironment, installSteps, isStandalone } from "../lib/install";
  import { disconnect, refresh, send, session, setSoundLevel, setSoundPreset, setTheme } from "../lib/session.svelte";
  import type { SettingChoice } from "../lib/types";

  let { open = false, onclose }: { open?: boolean; onclose: () => void } = $props();

  type SettingSection = {
    id: string;
    heading: string;
    choices: SettingChoice[];
    action?: "order" | "pc";
  };

  type Category = {
    id: string;
    group: string;
    title: string;
    blurb: string;
    heading: string;
    description: string;
    note?: string;
    sections?: SettingSection[];
  };

  const remoteCategories: Category[] = [
    { id: "controls", group: "This remote", title: "Controls", blurb: "Touchpad or directional pad", heading: "Controls", description: "Choose how this phone's pad drives Luma. Other remotes keep their own setting." },
    { id: "browser", group: "This remote", title: "This browser", blurb: "Theme and fullscreen", heading: "This browser", description: "Appearance and window options saved on this device." },
    { id: "connection", group: "This remote", title: "Connection", blurb: "Pairing and this session", heading: "Connection", description: "The link between this browser and your PC." },
  ];

  const appCategoryMeta: Record<string, { title: string; blurb: string; heading: string; description: string; note: string }> = {
    "appearance-home": {
      title: "Appearance & Home",
      blurb: "Theme, layout, and Home",
      heading: "Appearance & Home",
      description: "The look and layout of Luma on the TV and connected displays.",
      note: "Home rows and widgets are arranged on the PC.",
    },
    "display-sound": {
      title: "Display & Sound",
      blurb: "Fullscreen, sound, and PC window",
      heading: "Display & Sound",
      description: "Fullscreen, interface sounds, and the window Luma is controlling.",
      note: "TV screen and audio output are chosen on the PC.",
    },
    "apps-services": {
      title: "Apps & Services",
      blurb: "Dock order and apps",
      heading: "Apps & Services",
      description: "The shared app dock and the apps available on this remote.",
      note: "App paths and accounts are managed on the PC.",
    },
    system: {
      title: "System",
      blurb: "Startup and memory",
      heading: "System",
      description: "Startup, memory, and background behavior.",
      note: "Startup, updates, and tray behavior are on the PC.",
    },
  };

  const appCategoryOrder = ["appearance-home", "display-sound", "apps-services", "system"];

  const sectionMeta: Record<string, { category: string; heading: string; order: number }> = {
    theme: { category: "appearance-home", heading: "Theme", order: 0 },
    layout: { category: "appearance-home", heading: "Layout", order: 1 },
    "remote-layout": { category: "appearance-home", heading: "Layout", order: 1 },
    display: { category: "display-sound", heading: "Display", order: 0 },
    "remote-display": { category: "display-sound", heading: "Display", order: 0 },
    sound: { category: "display-sound", heading: "Sound", order: 1 },
    startup: { category: "system", heading: "Startup & memory", order: 0 },
    "remote-system": { category: "system", heading: "Startup & memory", order: 0 },
  };

  const cardId = (section: string): string =>
    section === "remote-layout" ? "layout"
      : section === "remote-display" ? "display"
        : section === "remote-system" ? "startup"
          : section;

  let lumaCategories: Category[] = $derived.by(() => {
    const categories = new Map<string, Category>();
    for (const id of appCategoryOrder) {
      const meta = appCategoryMeta[id];
      categories.set(id, {
        id,
        group: "Luma app",
        title: meta.title,
        blurb: meta.blurb,
        heading: meta.heading,
        description: meta.description,
        note: meta.note,
        sections: [],
      });
    }

    const ensure = (id: string, fallbackTitle: string): Category => {
      let category = categories.get(id);
      if (!category) {
        category = {
          id,
          group: "Luma app",
          title: fallbackTitle,
          blurb: "Luma app settings",
          heading: fallbackTitle,
          description: "Settings shared with the TV and connected displays.",
          note: "",
          sections: [],
        };
        categories.set(id, category);
      }
      return category;
    };

    for (const choice of session.state?.settings ?? []) {
      const section = choice.section || choice.group.toLowerCase();
      const meta = sectionMeta[section] ?? {
        category: `app:${choice.group.toLowerCase().replace(/[^a-z0-9]+/g, "-")}`,
        heading: choice.group || "Luma app",
        order: 50,
      };
      const category = ensure(meta.category, choice.group || "Luma app");
      const id = cardId(section);
      let card = category.sections?.find(item => item.id === id);
      if (!card) {
        card = { id, heading: meta.heading, choices: [] };
        category.sections?.push(card);
      }
      card.choices.push(choice);
    }

    ensure("apps-services", "Apps & Services").sections?.push({
      id: "order",
      heading: "Dock",
      choices: [],
      action: "order",
    });
    ensure("display-sound", "Display & Sound").sections?.push({
      id: "pc-window",
      heading: "PC window",
      choices: [],
      action: "pc",
    });

    const sectionRank = (section: SettingSection): number => {
      if (section.action === "order") return 0;
      if (section.action === "pc") return 3;
      return sectionMeta[section.id]?.order ?? 50;
    };
    for (const category of categories.values()) {
      category.sections?.sort((left, right) => sectionRank(left) - sectionRank(right));
    }
    return [...categories.values()].filter(category => (category.sections?.length ?? 0) > 0);
  });

  let categories: Category[] = $derived([...remoteCategories, ...lumaCategories]);

  let groups: { label: string; ids: string[] }[] = $derived([
    { label: "This remote", ids: remoteCategories.map(item => item.id) },
    { label: "Luma app", ids: lumaCategories.map(item => item.id) },
  ]);

  let category = $state("controls");
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
  let browserNote = $state("");
  let pcNote = $state("");
  let wasOpen = false;

  let active = $derived(categories.find(item => item.id === category) ?? categories[0]);
  let appearance = $derived(session.state?.appearance);
  let themeName = $derived(themeLabel(appearance, session.themeChoice));
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

  function categoryClass(id: string): string {
    return `settings-category ${category === id ? "is-current" : ""}`;
  }

  function selectCategory(id: string): void {
    category = id;
    themeOpen = false;
    settingsNote = "";
    settingsTone = "neutral";
    browserNote = "";
    pcNote = "";
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
      if (choice.key === "soundLevel") setSoundLevel(value);
      else if (choice.key === "interfaceSounds") setSoundPreset(value);
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
    browserNote = "";
    try {
      if (document.fullscreenElement) await document.exitFullscreen();
      else await document.documentElement.requestFullscreen();
    } catch (_) {
      browserNote = "Remote fullscreen is unavailable in this browser.";
    }
  }

  async function togglePcFullscreen(): Promise<void> {
    pcNote = "";
    const result = await send("fullscreen");
    if (!result.accepted) {
      pcNote = result.error ?? "Luma did not accept the fullscreen command.";
      return;
    }
    const state = result.state ?? session.state;
    const name = state?.window ?? "The PC window";
    const value = state?.windowFullscreen;
    pcNote = value === true
      ? `${name} is fullscreen.`
      : value === false
        ? `${name} is windowed.`
        : "Fullscreen command sent to the PC app.";
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
    if (!categories.some(item => item.id === category)) category = categories[0]?.id ?? "controls";
  });

  $effect(() => {
    if (open && !wasOpen) {
      wasOpen = true;
      detail = false;
      void refresh();
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
    <button type="button" id="settings-close" class="settings-close" aria-label="Close settings" onclick={onclose}>
      <Icon name="close" className="size-[15px]" />
    </button>
  </header>

  <div class="settings-layout" class:is-detail={detail}>
    <nav class="settings-nav" aria-label="Settings categories">
      {#each groups as group (group.label)}
        <p class="nav-group">{group.label}</p>
        {#each group.ids as id (id)}
          {@const item = categories.find(entry => entry.id === id)}
          {#if item}
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
          {/if}
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
            <SettingSwitch
              checked={control.mode === "touchpad"}
              label="Touchpad mode"
              onchange={value => setControlMode(value ? "touchpad" : "dpad")}
            />
          </div>
          <p class="row-note">This setting is saved on this phone. Touchpad mode is for desktop-style apps; the D-pad is the default for Luma.</p>
        </section>
      {/if}

      {#if category === "browser"}
        <section class="group">
          <h3>Theme</h3>
          <div class="field">
            <span class="row-copy">
              <b>Remote theme</b>
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
          {#if browserNote}<p class="row-note" role="status">{browserNote}</p>{/if}
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

      {#if active.sections}
        {#if settingsNote}
          <p class="settings-note" data-tone={settingsTone} role="status" aria-live="polite">{settingsNote}</p>
        {/if}

        {#each active.sections as section (section.id)}
          <section class="group">
            <h3>{section.heading}</h3>

            {#if section.action === "order"}
              <div class="row">
                <span class="row-copy">
                  <b>App order</b>
                  <small>Move apps into the order you want.</small>
                </span>
                <button type="button" class="pill" data-order-trigger onclick={() => (orderOpen = true)}>Edit</button>
              </div>
            {:else if section.action === "pc"}
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
              {#if pcNote}<p class="row-note" role="status">{pcNote}</p>{/if}
            {:else}
              {#each section.choices as choice (choice.key)}
                {#if choice.kind === "toggle"}
                  <div class="row">
                    <span class="row-copy">
                      <b>{choice.label}</b>
                      <small>{choice.description}</small>
                    </span>
                    <SettingSwitch
                      checked={(drafts[choice.key] ?? choice.value) === "true"}
                      label={choice.label}
                      disabled={settingsSaving}
                      onchange={value => saveSetting(choice, String(value))}
                    />
                  </div>
                {:else}
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
                {/if}
              {/each}
            {/if}
          </section>
        {/each}

        {#if active.note}<p class="row-note">{active.note}</p>{/if}
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
  .field + .field,
  .field + .row,
  .row + .field {
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
    position: sticky;
    top: 0;
    z-index: 2;
    margin: 8px 0 4px;
    padding: 9px 12px;
    border: 1px solid var(--line-strong);
    border-radius: 10px;
    background: color-mix(in srgb, var(--surface-3) 94%, transparent);
    color: var(--text-dim);
    font-size: 11.5px;
    line-height: 1.4;
    backdrop-filter: blur(10px);
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
    min-height: 44px;
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
    width: 100%;
  }

  .theme-select {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    width: 100%;
    min-height: 44px;
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
    min-height: 44px;
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
  }
</style>
