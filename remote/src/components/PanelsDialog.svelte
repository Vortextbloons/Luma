<script lang="ts">
  import Icon from "./Icon.svelte";
  import Modal from "./Modal.svelte";
  import { api } from "../lib/api";
  import { panelActionEnabled, panelActionLabel, normalizePanels } from "../lib/panels";
  import { applyMusic, session, showPairing } from "../lib/session.svelte";
  import { say } from "../lib/toast.svelte";
  import type { PanelAction, PanelCommand, RemotePanel } from "../lib/types";

  let { open = false, onclose }: { open?: boolean; onclose: () => void } = $props();

  let panels = $state<RemotePanel[]>([]);
  let loading = $state(false);
  let failed = $state(false);
  let loaded = $state(false);
  let busy = $state("");
  let feedback = $state("");
  let wasOpen = false;
  let request = 0;

  let music = $derived(session.state?.music ?? null);

  async function load(): Promise<void> {
    const current = ++request;
    loading = true;
    failed = false;
    try {
      const { response, data } = await api.panels();
      if (current !== request) return;
      if (response.status === 401) {
        close();
        showPairing("This remote is no longer paired. Choose Connect in Luma for a new code.");
        return;
      }
      if (!response.ok) {
        failed = true;
        return;
      }
      panels = normalizePanels(data?.panels);
      loaded = true;
    } catch (_) {
      if (current === request) failed = true;
    } finally {
      if (current === request) loading = false;
    }
  }

  function close(): void {
    request++;
    onclose();
  }

  async function run(panel: RemotePanel, action: PanelAction): Promise<void> {
    if (busy) return;
    await send(panel, action);
  }

  async function runCommand(panel: RemotePanel, command: PanelCommand): Promise<void> {
    if (busy) return;
    await send(panel, command.id);
  }

  async function send(panel: RemotePanel, action: string): Promise<void> {
    busy = `${panel.id}:${action}`;
    feedback = "";
    try {
      const { response, data } = await api.panelAction(panel.id, action, panel.version);
      if (response.status === 401) {
        close();
        showPairing("This remote is no longer paired. Choose Connect in Luma for a new code.");
        return;
      }
      if (!response.ok) {
        feedback = "That control is unavailable right now. Try again.";
        return;
      }
      if (data?.music !== undefined) applyMusic(data.music ?? null);
      if (data?.accepted) {
        if (panel.version) feedback = "Command sent.";
        return;
      }
      if (data?.status === "pending") {
        say(data.error || "The plugin is starting. Try again in a moment.");
        return;
      }
      feedback = data?.error || "That control is unavailable right now.";
      say(feedback, true);
      if (
        data?.code === "plugin_missing" ||
        data?.code === "plugin_disabled" ||
        data?.code === "plugin_denied" ||
        data?.code === "plugin_command" ||
        data?.code === "denied" ||
        data?.code === "stale"
      ) {
        void load();
      }
    } catch (_) {
      feedback = "Luma is not reachable. Check Wi-Fi and that Luma is open.";
    } finally {
      busy = "";
    }
  }

  $effect(() => {
    if (open && !wasOpen) {
      wasOpen = true;
      feedback = "";
      void load();
    } else if (!open) {
      wasOpen = false;
    }
  });
</script>

<Modal id="panels-dialog" titleId="panels-title" {open} className="panels-dialog" onrequestclose={close}>
  <header class="panels-head">
    <button type="button" class="panels-back" aria-label="Back to remote" onclick={close}>
      <Icon name="back" className="size-[15px]" />
      Remote
    </button>
    <div>
      <h2 id="panels-title">Plugin panels</h2>
      <p>Controls shared by plugins enabled in Luma</p>
    </div>
  </header>

  <div class="panels-body">
    {#if loading && !loaded}
      <div class="panel-card is-loading" aria-busy="true" aria-label="Loading plugin panels">
        <span></span><span></span><span></span>
      </div>
    {:else if failed}
      <div class="panels-empty" role="alert">
        <b>Plugin panels are unavailable right now</b>
        <p>Check the connection to your PC, then try again.</p>
        <button type="button" onclick={load}>Try again</button>
      </div>
    {:else if !panels.length}
      <div class="panels-empty">
        <b>No plugin panels are enabled</b>
        <p>Enable a plugin in Luma on your PC. Plugins that offer remote controls appear here.</p>
        <button type="button" onclick={load}>Check again</button>
      </div>
    {:else}
      {#each panels as panel (panel.id)}
        <article class="panel-card" aria-labelledby={`panel-${panel.id}`}>
          <header>
            <div class="panel-copy">
              {#if panel.name.toLowerCase() !== panel.title.toLowerCase()}<small>{panel.name}</small>{/if}
              <h3 id={`panel-${panel.id}`}>{panel.title}</h3>
            </div>
          </header>

          {#if panel.actions.length || music}
            <div class="panel-now">
              <span class="panel-dot" class:is-playing={music?.playing} aria-hidden="true"></span>
              <span class="panel-track">
                <b>{music?.title ?? "Nothing is playing"}</b>
                <small>{music?.artist ?? (music ? "Ready on the PC" : "Start music on the PC or TV")}</small>
              </span>
            </div>
          {/if}

          {#if panel.commands.length}
            <div class="panel-actions" role="group" aria-label={`${panel.title} commands`}>
              {#each panel.commands as command, index (command.id)}
                <button
                  type="button"
                  class={{ 'is-primary': index === 0 }}
                  disabled={busy !== ""}
                  aria-busy={busy === `${panel.id}:${command.id}`}
                  onclick={() => runCommand(panel, command)}
                >
                  <span>{busy === `${panel.id}:${command.id}` ? "Sending…" : command.label}</span>
                </button>
              {/each}
            </div>
          {/if}

          {#if panel.actions.length}
            <div class="panel-actions">
              {#each panel.actions as action (action)}
                <button
                  type="button"
                  class:is-primary={action === "playback.toggle"}
                  disabled={busy !== "" || !panelActionEnabled(action, music)}
                  aria-label={panelActionLabel(action, music)}
                  title={panelActionEnabled(action, music)
                    ? panelActionLabel(action, music)
                    : music
                      ? "This control is unavailable right now"
                      : "Playback is unavailable on the PC"}
                  onclick={() => run(panel, action)}
                >
                  <Icon
                    name={action === "playback.previous"
                      ? "previous"
                      : action === "playback.next"
                        ? "next"
                        : music?.playing
                          ? "pause"
                          : "play"}
                    className="size-[18px]"
                  />
                </button>
              {/each}
            </div>
          {:else if !panel.commands.length}
            <p class="panel-note">This plugin only shows playback on this remote.</p>
          {/if}
        </article>
      {/each}
      <p class="panels-foot">Only controls declared by enabled plugins appear here. Plugin pages stay on the PC.</p>
    {/if}

    {#if feedback}<p class="panels-feedback" role="status">{feedback}</p>{/if}
  </div>
</Modal>

<style>
  :global(dialog.panels-dialog) {
    margin: auto;
    width: min(560px, calc(100% - 24px));
    max-height: min(760px, calc(100dvh - 24px));
    padding: 0;
    overflow: hidden;
    border: 1px solid var(--line-strong);
    border-radius: 22px;
    background: var(--surface-2);
  }

  .panels-head {
    display: flex;
    flex: none;
    flex-direction: column;
    gap: 14px;
    padding: 18px 18px 16px;
    border-bottom: 1px solid var(--line);
  }

  .panels-head h2 {
    margin: 0;
    font-size: 24px;
    font-weight: 700;
    letter-spacing: -.03em;
  }

  .panels-head p {
    margin: 4px 0 0;
    color: var(--muted);
    font-size: 12.5px;
  }

  .panels-back {
    display: flex;
    align-items: center;
    align-self: flex-start;
    gap: 6px;
    min-height: 38px;
    padding: 0 13px 0 10px;
    border: 1px solid var(--line);
    border-radius: 999px;
    background: rgba(255, 255, 255, .04);
    color: var(--text-dim);
    font-size: 12px;
    font-weight: 650;
  }

  .panels-body {
    display: grid;
    gap: 12px;
    min-height: 0;
    padding: 16px 18px calc(18px + env(safe-area-inset-bottom));
    overflow-y: auto;
    overscroll-behavior: contain;
  }

  .panel-card {
    display: grid;
    gap: 12px;
    padding: 14px;
    border: 1px solid var(--line);
    border-radius: 18px;
    background: linear-gradient(180deg, var(--surface), var(--surface-2));
  }

  .panel-card header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 12px;
  }

  .panel-copy {
    display: grid;
    gap: 3px;
    min-width: 0;
  }

  .panel-copy small {
    overflow: hidden;
    color: var(--faint);
    font-size: 10.5px;
    font-weight: 650;
    letter-spacing: .04em;
    text-overflow: ellipsis;
    text-transform: uppercase;
    white-space: nowrap;
  }

  .panel-copy h3 {
    margin: 0;
    overflow: hidden;
    color: var(--text);
    font-size: 17px;
    font-weight: 700;
    line-height: 1.25;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .panel-now {
    display: flex;
    align-items: center;
    gap: 10px;
    min-width: 0;
    padding: 10px 11px;
    border-radius: 13px;
    background: rgba(255, 255, 255, .035);
  }

  .panel-dot {
    flex: none;
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--line-strong);
  }

  .panel-dot.is-playing {
    background: var(--good);
    box-shadow: 0 0 8px color-mix(in srgb, var(--good) 70%, transparent);
  }

  .panel-track {
    display: grid;
    gap: 2px;
    min-width: 0;
  }

  .panel-track b,
  .panel-track small {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .panel-track b {
    color: var(--text-dim);
    font-size: 13px;
    font-weight: 650;
  }

  .panel-track small {
    color: var(--faint);
    font-size: 11.5px;
  }

  .panel-actions {
    display: flex;
    gap: 7px;
  }

  .panel-actions button {
    display: grid;
    flex: 1;
    place-items: center;
    min-height: 42px;
    padding: 0 10px;
    border: 0;
    border-radius: 11px;
    background: rgba(255, 255, 255, .045);
    color: var(--muted);
    font-size: 12.5px;
    font-weight: 650;
    line-height: 1.25;
    text-align: center;
  }

  .panel-actions button span {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .panel-actions button.is-primary {
    background: var(--accent-soft);
    color: var(--accent-strong);
  }

  .panel-actions button:active:not(:disabled) {
    background: rgba(255, 255, 255, .11);
    color: var(--text);
    transform: scale(.96);
  }

  .panel-actions button.is-primary:active:not(:disabled) {
    background: var(--accent);
    color: var(--accent-ink);
  }

  .panel-actions button:disabled {
    opacity: .38;
  }

  .panel-note,
  .panels-foot {
    margin: 0;
    color: var(--faint);
    font-size: 11.5px;
    line-height: 1.45;
  }

  .panels-foot {
    padding: 0 2px;
  }

  .panels-feedback {
    margin: 0;
    padding: 10px 12px;
    border: 1px solid color-mix(in srgb, var(--danger) 30%, transparent);
    border-radius: 12px;
    background: color-mix(in srgb, var(--danger) 8%, transparent);
    color: var(--danger);
    font-size: 12px;
    line-height: 1.45;
  }

  .panels-empty {
    display: grid;
    justify-items: start;
    gap: 8px;
    padding: 18px;
    border: 1px solid var(--line);
    border-radius: 18px;
    background: rgba(255, 255, 255, .025);
  }

  .panels-empty b {
    font-size: 14px;
    font-weight: 650;
  }

  .panels-empty p {
    margin: 0;
    color: var(--muted);
    font-size: 12.5px;
    line-height: 1.55;
  }

  .panels-empty button {
    min-height: 38px;
    margin-top: 4px;
    padding: 0 14px;
    border: 1px solid var(--line-strong);
    border-radius: 999px;
    background: rgba(255, 255, 255, .06);
    color: var(--text);
    font-size: 12px;
    font-weight: 650;
  }

  .panel-card.is-loading span {
    display: block;
    height: 14px;
    border-radius: 99px;
    background: rgba(255, 255, 255, .06);
    animation: pulse-soft 1.1s ease-in-out infinite;
  }

  .panel-card.is-loading span:nth-child(1) { width: 45%; }
  .panel-card.is-loading span:nth-child(2) { width: 100%; height: 40px; }
  .panel-card.is-loading span:nth-child(3) { width: 70%; height: 36px; }

  @media (max-width: 639px) {
    :global(dialog.panels-dialog) {
      width: 100%;
      max-width: 100%;
      max-height: 100dvh;
      margin: 0;
      border: 0;
      border-radius: 0;
    }

    .panels-head {
      padding: max(14px, env(safe-area-inset-top)) max(16px, env(safe-area-inset-right)) 14px max(16px, env(safe-area-inset-left));
    }

    .panels-body {
      padding: 14px max(16px, env(safe-area-inset-right)) calc(18px + env(safe-area-inset-bottom)) max(16px, env(safe-area-inset-left));
    }
  }
</style>
