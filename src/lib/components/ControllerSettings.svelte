<script lang="ts">
  import { onDestroy } from 'svelte';
  import {
    connectedControllers,
    controllerActions,
    controllerButtonLabel,
    controllerFamilies,
    controllerFamily,
    defaultControllerBinds,
    type ControllerFamily,
  } from '../services/controller';
  import ControllerBrandLogo from './ControllerBrandLogo.svelte';
  import ControllerDiagram from './ControllerDiagram.svelte';
  import type { ControllerAction, Settings } from '../types';

  let { settings = $bindable(), oncapturechange }: {
    settings: Settings;
    oncapturechange?: (handler: ((buttonIndex: number) => boolean) | null) => void;
  } = $props();

  let recording = $state<ControllerAction | null>(null);
  let feedback = $state('');
  let controllers = $state<string[]>([]);
  let family = $state<ControllerFamily>('xbox');
  let manualFamily = $state(false);
  let hoverAction = $state<ControllerAction | null>(null);

  function refreshControllers() {
    controllers = connectedControllers();
    if (controllers.length && !manualFamily) family = controllerFamily(controllers[0]);
  }

  function chooseFamily(next: ControllerFamily) {
    family = next;
    manualFamily = true;
  }

  function stopCapture() {
    recording = null;
    oncapturechange?.(null);
  }

  function startCapture(action: ControllerAction, label: string) {
    if (recording === action) {
      stopCapture();
      feedback = '';
      return;
    }
    recording = action;
    feedback = `Press any button on your controller for ${label} (Escape cancels).`;
    oncapturechange?.(buttonIndex => {
      settings.controllerBinds = { ...settings.controllerBinds, [action]: buttonIndex };
      feedback = `${label} bound to ${controllerButtonLabel(buttonIndex, family)}.`;
      stopCapture();
      return true;
    });
  }

  function clearBind(action: ControllerAction, label: string) {
    if (recording === action) stopCapture();
    settings.controllerBinds = { ...settings.controllerBinds, [action]: null };
    feedback = `${label} cleared.`;
  }

  function resetBinds() {
    stopCapture();
    settings.controllerBinds = defaultControllerBinds();
    feedback = 'Controller binds reset to defaults.';
  }

  function actionLabel(action: ControllerAction) {
    return controllerActions.find(item => item.key === action)?.label ?? action;
  }

  function handleKeydown(event: KeyboardEvent) {
    if (recording && event.key === 'Escape') {
      event.preventDefault();
      stopCapture();
      feedback = 'Rebind cancelled.';
    }
  }

  refreshControllers();

  if (typeof window !== 'undefined') {
    window.addEventListener('gamepadconnected', refreshControllers);
    window.addEventListener('gamepaddisconnected', refreshControllers);
  }

  onDestroy(() => {
    stopCapture();
    if (typeof window !== 'undefined') {
      window.removeEventListener('gamepadconnected', refreshControllers);
      window.removeEventListener('gamepaddisconnected', refreshControllers);
    }
  });
</script>

<svelte:window onkeydown={handleKeydown} />

<section class="settings-group">
  <h3>Controller navigation</h3>
  <p class="panel-description">Use a game controller when Luma is in focus.</p>
  <label class="setting-toggle">
    <span class="setting-toggle-copy">
      <span>Enable controller input</span>
      <small>Navigate menus, rows, and dialogs with a gamepad.</small>
    </span>
    <input type="checkbox" bind:checked={settings.controllerEnabled} />
  </label>
  <p class="panel-description">
    {#if controllers.length}
      Connected: {controllers.join(', ')}
    {:else}
      No controller detected yet. Press any button on a connected controller to wake it up.
    {/if}
  </p>
</section>

<section class="settings-group">
  <div class="settings-group-head">
    <div class="controller-map-copy">
      <h3>Controller map</h3>
      <p class="panel-description">Select a button to change its action.</p>
    </div>
    <div class="controller-family-picker" role="group" aria-label="Controller layout">
      {#each controllerFamilies as item (item.id)}
        <button class="controller-family-option" aria-pressed={family === item.id} onclick={() => chooseFamily(item.id)}>
          <ControllerBrandLogo family={item.id} label={item.label} />
          <span>{item.label}</span>
        </button>
      {/each}
    </div>
  </div>
  <ControllerDiagram
    {family}
    binds={settings.controllerBinds}
    {recording}
    highlight={hoverAction}
    oncapture={action => startCapture(action, actionLabel(action))}
    onhover={action => hoverAction = action}
  />
  <p class="panel-description controller-map-status" role="status">
    {#if recording}
      Press a button on your controller for {actionLabel(recording)}. Escape cancels.
    {:else if controllers.length}
      Detected: {controllers[0]}. Showing the {controllerFamilies.find(item => item.id === family)?.label ?? family} layout.
    {:else}
      No controller detected yet. Showing the {controllerFamilies.find(item => item.id === family)?.label ?? family} layout.
    {/if}
  </p>
</section>

<section class="settings-group">
  <h3>Button assignments</h3>
  <p class="panel-description">Choose a button for each action.</p>
  <div class="controller-bind-list">
    {#each controllerActions as item (item.key)}
      <div
        class="settings-row controller-bind-row"
        class:is-highlight={hoverAction === item.key}
        role="group"
        aria-label={item.label}
        onpointerenter={() => hoverAction = item.key}
        onpointerleave={() => hoverAction = null}
        onfocusin={() => hoverAction = item.key}
        onfocusout={event => { if (!event.currentTarget.contains(event.relatedTarget as Node)) hoverAction = null; }}
      >
        <div class="settings-row-copy">
          <strong>{item.label}</strong>
          <span>{item.description}</span>
        </div>
        <span class="controller-bind-chip" class:unbound={settings.controllerBinds[item.key] === null}>{controllerButtonLabel(settings.controllerBinds[item.key], family)}</span>
        <div class="controller-bind-actions">
          <button class="panel-secondary" onclick={() => startCapture(item.key, item.label)}>
            {recording === item.key ? 'Press a button…' : 'Rebind'}
          </button>
          <button class="panel-secondary" disabled={settings.controllerBinds[item.key] === null} onclick={() => clearBind(item.key, item.label)}>
            Clear
          </button>
        </div>
      </div>
    {/each}
  </div>
  <div class="settings-actions">
    <button class="panel-secondary" onclick={resetBinds}>Reset binds to defaults</button>
  </div>
  {#if feedback}<p role="status" class="panel-description">{feedback}</p>{/if}
</section>

<style>
  .controller-map-copy { min-width: 0; }
  .controller-family-picker { display: flex; flex-wrap: wrap; gap: 8px; }
  .controller-family-option { display: inline-flex; align-items: center; gap: 9px; min-height: 44px; padding: 0 15px; border: 1px solid #ffffff1c; border-radius: 999px; background: #ffffff08; color: var(--text-dim); font-size: 13px; font-weight: 650; }
  .controller-family-option:hover { background: #ffffff12; color: var(--text); }
  .controller-family-option[aria-pressed='true'] { background: var(--accent-soft); border-color: var(--accent); color: var(--text); }
  .controller-family-option:focus-visible { outline: 3px solid var(--accent); outline-offset: 3px; }
  .controller-map-status { margin: 14px 0 0 !important; font-size: 12px !important; }
  .controller-bind-list { display: grid; gap: 8px; }
  .controller-bind-row { gap: 16px; }
  .controller-bind-row.is-highlight { border-color: color-mix(in srgb, var(--accent) 55%, transparent); background: color-mix(in srgb, var(--accent) 8%, #ffffff04); }
  .controller-bind-chip { flex: none; min-width: 88px; padding: 7px 12px; border: 1px solid #ffffff1c; border-radius: 9px; background: #ffffff0d; color: var(--text); font-size: 12px; font-weight: 650; text-align: center; }
  .controller-bind-chip.unbound { color: var(--muted); }
  .controller-bind-actions { display: flex; flex: none; gap: 8px; }
</style>
