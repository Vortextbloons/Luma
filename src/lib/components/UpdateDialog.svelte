<script lang="ts">
  import { version as buildVersion } from '../../../package.json';
  import { updates, downloadUpdate, installUpdate } from '../state/updates.svelte';
  import ReleaseNotes from './ReleaseNotes.svelte';

  let { kind, onclose }: { kind: 'installed' | 'update'; onclose: () => void } = $props();
  const available = $derived(kind === 'update');
  const version = $derived(available ? updates.nextVersion : updates.version || buildVersion);
  const busy = $derived(updates.phase === 'downloading' || updates.phase === 'installing');
  const progress = $derived(updates.total ? Math.min(100, Math.round(updates.downloaded / updates.total * 100)) : undefined);

  function show(dialog: HTMLDialogElement) {
    const reader = dialog.querySelector<HTMLElement>('[data-release-notes-scroll]');
    reader?.setAttribute('tabindex', '0');
    dialog.showModal();
    reader?.focus({ preventScroll: true });
  }

  function update() {
    if (updates.phase === 'ready') void installUpdate();
    else if (updates.phase === 'available') void downloadUpdate();
  }
</script>

<dialog class="update-dialog" aria-labelledby="update-title" {@attach show} oncancel={event => { event.preventDefault(); onclose(); }}>
  <div class="update-layout">
    <header class="update-header">
      <div>
        <p class="release-version">Luma {version} · {available ? 'Update available' : 'Installed version'}</p>
        <h1 id="update-title">What’s new</h1>
      </div>
      <button class="btn btn-glass" onclick={onclose}>Close</button>
    </header>
    {#if available}
      <div class="update-controls">
        <div class="update-status" role="status">
          {#if updates.phase === 'downloading'}
            <p>{progress === undefined ? 'Downloading update…' : `Downloading update · ${progress}%`}</p>
            <progress aria-label="Update download" value={progress} max="100"></progress>
          {:else if updates.phase === 'ready'}
            <p>Ready to install. Restart Luma to finish.</p>
          {:else if updates.phase === 'installing'}
            <p>Restarting to install the update…</p>
          {:else}
            <p>Download the update and restart when you’re ready.</p>
          {/if}
          {#if updates.error}<p class="error">{updates.error}</p>{/if}
        </div>
        <button class="btn btn-primary" aria-disabled={busy} onclick={update}>
          {updates.phase === 'ready' ? 'Restart to update' : updates.phase === 'installing' ? 'Restarting…' : updates.phase === 'downloading' ? 'Downloading…' : 'Update now'}
        </button>
      </div>
    {/if}
    <div class="update-scroll" data-release-notes-scroll role="region" aria-label="Changelog">
      <div class="update-reading">
        <ReleaseNotes {version} notes={available ? updates.notes : undefined} />
      </div>
    </div>
  </div>
</dialog>

<style>
  .update-dialog[open] { position: fixed; inset: 0; display: block; width: min(1120px, calc(100vw - 32px)); height: min(86dvh, 1000px); max-width: none; max-height: none; margin: auto; padding: 0; overflow: hidden; border: 1px solid var(--line-strong); border-radius: 22px; background: var(--bg); color: var(--text); box-shadow: 0 24px 80px #0009; }
  .update-dialog::backdrop { background: #04050ac2; }
  .update-layout { height: 100%; display: flex; flex-direction: column; min-height: 0; }
  .update-header { display: grid; grid-template-columns: 1fr auto 1fr; align-items: center; gap: 12px; padding: clamp(16px, 2vw, 34px) clamp(16px, 2.4vw, 38px); border-bottom: 1px solid var(--line-strong); background: var(--surface-2); text-align: center; }
  .update-header > div { grid-column: 2; }
  .update-header h1 { margin: .12em 0 0; font-size: clamp(30px, 3vw, 56px); font-weight: 700; line-height: 1.04; letter-spacing: -.045em; }
  .release-version { margin: 0; color: var(--accent-strong); font-size: clamp(14px, 1.1vw, 19px); font-weight: 650; }
  .update-header .btn { justify-self: end; min-width: 0; min-height: 44px; padding: 0 16px; }
  .update-controls { display: flex; flex-wrap: wrap; justify-content: center; align-items: center; text-align: center; gap: 12px 22px; padding: clamp(10px, 1.3vw, 18px) 18px; border-bottom: 1px solid var(--line); background: var(--surface); }
  .update-status { color: var(--text-dim); font-size: clamp(14px, .95vw, 18px); }
  .update-status p { margin: 0; }
  .update-status .error { margin-top: 5px; }
  .update-controls progress { width: min(320px, 54vw); height: 8px; accent-color: var(--accent); vertical-align: middle; }
  .update-controls button[aria-disabled='true'] { opacity: .65; cursor: wait; }
  .update-scroll { flex: 1; min-height: 0; overflow-y: auto; overscroll-behavior: contain; scroll-padding: 16px; }
  .update-scroll:focus-visible { outline: 2px solid var(--accent); outline-offset: -2px; }
  .update-reading { max-width: 84ch; margin-inline: auto; padding: clamp(18px, 2.4vw, 36px) clamp(18px, 2.5vw, 40px) 40px; }
  @media (max-width: 520px) {
    .update-header { grid-template-columns: minmax(0, 1fr); }
    .update-header > div { grid-column: 1; }
    .update-header .btn { justify-self: center; }
  }
</style>
