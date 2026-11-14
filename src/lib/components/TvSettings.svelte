<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  let { native }: { native: boolean } = $props();
  type Setup = {
    preferences: { display: string; audio: string };
    displays: { id: string; width: number; height: number }[];
    audio: { id: string; name: string }[];
  };
  let setup = $state<Setup>();
  let display = $state('');
  let audio = $state('');
  let busy = $state(false);
  let feedback = $state('');
  let failed = $state(false);
  async function refresh() {
    busy = true;
    feedback = '';
    failed = false;
    try {
      setup = await invoke<Setup>('tv_setup');
      display = setup.preferences.display;
      audio = setup.preferences.audio;
    } catch (error) { feedback = String(error); failed = true; }
    finally { busy = false; }
  }
  async function save() {
    busy = true;
    feedback = '';
    failed = false;
    try {
      await invoke('save_tv_setup', { preferences: { display, audio } });
      feedback = 'TV setup saved. Luma will use these devices when you open it.';
    } catch (error) { feedback = String(error); failed = true; }
    finally { busy = false; }
  }
  async function restore() {
    busy = true;
    failed = false;
    try {
      await invoke('restore_tv_audio');
      feedback = 'PC audio restored. Your TV audio choice is saved for next time.';
    } catch (error) { feedback = String(error); failed = true; }
    finally { busy = false; }
  }
  onMount(() => { if (native) void refresh(); });
</script>

<section class="settings-group" aria-busy={busy}>
  <h3>TV setup</h3>
  <p class="panel-description">Choose the screen and sound output for watching from the sofa. Keep Windows in Extend mode to leave your desktop on its own screen.</p>
  {#if !native}
    <p class="panel-description">Open the desktop app to choose your TV and sound output.</p>
  {:else}
    {#if setup}
      <div class="settings-form-grid">
        <label class="field">TV screen
          <select bind:value={display} disabled={busy}>
            <option value="">Use the current screen</option>
            {#if display && !setup.displays.some(item => item.id === display)}<option value={display}>{display} — disconnected</option>{/if}
            {#each setup.displays as screen (screen.id)}<option value={screen.id}>{screen.id} — {screen.width} × {screen.height}</option>{/each}
          </select>
          <span class="field-hint">Luma and playback windows open here. Use Left and Right on your remote to change a choice, and Up or Down to move on. If the TV is disconnected, Luma opens on an available screen.</span>
        </label>
        <label class="field">TV sound output
          <select bind:value={audio} disabled={busy}>
            <option value="">Use Windows audio output</option>
            {#if audio && !setup.audio.some(item => item.id === audio)}<option value={audio}>Saved output — disconnected</option>{/if}
            {#each setup.audio as output (output.id)}<option value={output.id}>{output.name}</option>{/each}
          </select>
          <span class="field-hint">Choose your TV, HDMI receiver, or speakers. This changes Windows playback audio for the whole PC. Apps with their own output setting may need to be changed separately.</span>
        </label>
      </div>
      <p class="panel-description">PC audio returns when you close Luma to the tray or quit. Remote volume controls adjust Windows volume; use the TV remote for the TV’s own volume.</p>
    {/if}
    <div class="settings-actions">
      <button class="panel-secondary" disabled={busy} onclick={refresh}>{busy ? 'Updating devices…' : 'Refresh devices'}</button>
      <button class="panel-secondary is-primary" disabled={busy || !setup} onclick={save}>Save TV setup</button>
      <button class="panel-secondary" disabled={busy || !setup} onclick={restore}>Restore PC audio</button>
    </div>
    {#if feedback}<p role="status" class={failed ? 'error' : 'panel-feedback'}>{feedback}</p>{/if}
  {/if}
</section>
