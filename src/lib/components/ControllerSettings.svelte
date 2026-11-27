<script lang="ts">
  import { onDestroy } from 'svelte';
  import {
    connectedControllers,
    controllerActions,
    controllerButtonLabel,
    defaultControllerBinds,
  } from '../services/controller';
  import type { NavigationCommand, Settings } from '../types';

  let { settings = $bindable(), oncapturechange }: {
    settings: Settings;
    oncapturechange?: (handler: ((buttonIndex: number) => boolean) | null) => void;
  } = $props();

  let recording = $state<NavigationCommand | null>(null);
  let feedback = $state('');
  let controllers = $state<string[]>(connectedControllers());

  function refreshControllers() {
    controllers = connectedControllers();
  }

  function stopCapture() {
    recording = null;
    oncapturechange?.(null);
  }

  function startCapture(action: NavigationCommand, label: string) {
    if (recording === action) {
      stopCapture();
      feedback = '';
      return;
    }
    recording = action;
    feedback = `Press any button on your controller for ${label} (Escape cancels).`;
    oncapturechange?.(buttonIndex => {
      settings.controllerBinds = { ...settings.controllerBinds, [action]: buttonIndex };
      feedback = `${label} bound to ${controllerButtonLabel(buttonIndex)}.`;
      stopCapture();
      return true;
    });
  }

  function clearBind(action: NavigationCommand, label: string) {
    if (recording === action) stopCapture();
    settings.controllerBinds = { ...settings.controllerBinds, [action]: null };
    feedback = `${label} cleared.`;
  }

  function resetBinds() {
    stopCapture();
    settings.controllerBinds = defaultControllerBinds();
    feedback = 'Controller binds reset to defaults.';
  }

  function handleKeydown(event: KeyboardEvent) {
    if (recording && event.key === 'Escape') {
      event.preventDefault();
      stopCapture();
      feedback = 'Rebind cancelled.';
    }
  }

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
  <p class="panel-description">Use a connected Xbox, PlayStation, or generic gamepad to navigate Luma while it is in focus. Left stick navigation is always active when enabled.</p>
  <label class="setting-toggle">
    <span class="setting-toggle-copy">
      <strong>Enable controller input</strong>
      <span>Navigate menus, rows, and dialogs with a gamepad.</span>
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
  <h3>Custom button binds</h3>
  <p class="panel-description">Choose which controller button triggers each action. Changes save automatically.</p>
  {#each controllerActions as item (item.key)}
    <div class="settings-row">
      <div class="settings-row-copy">
        <strong>{item.label}</strong>
        <span>{item.description} Current: {controllerButtonLabel(settings.controllerBinds[item.key])}</span>
      </div>
      <button class="panel-secondary" onclick={() => startCapture(item.key, item.label)}>
        {recording === item.key ? 'Press a button…' : 'Rebind'}
      </button>
      <button class="panel-secondary" disabled={settings.controllerBinds[item.key] === null} onclick={() => clearBind(item.key, item.label)}>
        Clear
      </button>
    </div>
  {/each}
  <div class="settings-actions">
    <button class="panel-secondary" onclick={resetBinds}>Reset binds to defaults</button>
  </div>
  {#if feedback}<p role="status" class="panel-description">{feedback}</p>{/if}
</section>
