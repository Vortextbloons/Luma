<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  let { native, autoConnect = false }: { native: boolean; autoConnect?: boolean } = $props();
  type Status = { enabled: boolean; paired: boolean; addresses: string[]; pcName: string; code: string | null; phonePaired: boolean; phoneUrl: string; phoneCode: string | null };
  type Device = 'phone' | 'frame';
  let status = $state<Status | null>(null);
  let busy = $state(false);
  let error = $state('');
  let device = $state<Device>('phone');
  let connecting = $state(false);
  let qr = $state('');
  async function connect(selected: Device = device) {
    device = selected;
    busy = true;
    error = '';
    qr = '';
    try {
      if (!status?.enabled) status = await invoke<Status>('configure_frame', { enabled: true });
      status = await invoke<Status>(selected === 'phone' ? 'phone_pairing' : 'frame_pairing');
      connecting = true;
      if (selected === 'phone' && status.addresses.length) qr = await invoke<string>('phone_remote_qr');
    } catch (reason) { error = String(reason); }
    finally { busy = false; }
  }
  const badge = $derived(status?.enabled
    ? status.paired && status.phonePaired
      ? 'Display Box and phone paired'
      : status.paired
        ? 'Display Box paired'
        : status.phonePaired
          ? 'Phone paired'
          : 'Ready to pair'
    : 'Remote control off');
  const tone = $derived(!status?.enabled ? 'is-muted' : status.paired || status.phonePaired ? 'is-good' : 'is-warn');
  async function run(command: string, args?: Record<string, unknown>) {
    busy = true;
    error = '';
    try { status = await invoke<Status>(command, args); }
    catch (reason) { error = String(reason); }
    finally { busy = false; }
  }
  async function toggleRemote(event: Event) {
    const input = event.currentTarget as HTMLInputElement;
    await run('configure_frame', { enabled: input.checked });
    if (status?.enabled !== input.checked) input.checked = Boolean(status?.enabled);
  }
  async function openRemote() {
    busy = true;
    error = '';
    try { await invoke('open_phone_remote'); }
    catch (reason) { error = String(reason); }
    finally { busy = false; }
  }
  onMount(() => {
    if (native) void (autoConnect ? connect() : run('frame_status'));
    let checking = false;
    const timer = setInterval(async () => {
      if (!native || busy || checking) return;
      checking = true;
      try { status = await invoke<Status>('frame_status'); }
      catch (reason) { error = String(reason); }
      finally { checking = false; }
    }, 3000);
    return () => clearInterval(timer);
  });
</script>

<section class="settings-group">
  <div class="settings-group-head">
    <h3>Phone &amp; Display Box remote</h3>
    {#if status}<span class={`settings-badge ${tone}`}>{badge}</span>{/if}
  </div>
  <p class="panel-description">Use a phone or Display Box to control Luma from another screen. Luma remembers paired devices after a restart.</p>
  {#if error}<p role="alert" class="error">{error}</p>{/if}
  {#if status}
    <button class="panel-secondary is-primary" disabled={busy} onclick={() => connect()}>Connect a remote</button>
    <label class="setting-toggle">
      <span class="setting-toggle-copy"><span>Allow remote control</span><small>Paired devices can control Luma while it is running.</small></span>
      <input type="checkbox" checked={status.enabled} disabled={busy} onchange={toggleRemote} />
    </label>
    {#if status.enabled && connecting}
      <div class="remote-choice" role="group" aria-label="Choose a device">
        <button disabled={busy} class:selected={device === 'phone'} aria-pressed={device === 'phone'} onclick={() => connect('phone')}>
          <strong>Phone</strong><span>Test on this PC, or use a phone on this Wi-Fi.</span>
        </button>
        <button disabled={busy} class:selected={device === 'frame'} aria-pressed={device === 'frame'} onclick={() => connect('frame')}>
          <strong>Display Box</strong><span>Use a Luma Display Box frame.</span>
        </button>
      </div>
      {#if device === 'phone'}
        <div class="frame-connect">
          {#if qr}<img class="remote-qr" src={qr} alt="Scan to open the Luma remote on your phone" /><p class="remote-note">Scan with your phone camera, then enter the code below. Keep both devices on the same Wi-Fi.</p>{/if}
          <ol class="remote-steps">
            <li>Scan the QR code, or type this address in your phone's browser.</li>
            <li>Enter the pairing code shown below.</li>
          </ol>
          {#if status.addresses.length}
            <code class="address">{status.phoneUrl}</code>
          {:else}
            <p class="remote-note">No network address was found. Connect this PC to Wi-Fi and try again.</p>
          {/if}
          <div class="remote-launch"><button class="panel-secondary" disabled={busy} onclick={openRemote}>Open remote in browser</button></div>
          <p class="remote-note">On your phone, use the address above, not localhost. Keep Luma running and both devices on the same Wi-Fi.</p>
          <button class="panel-secondary" disabled={busy || !status.addresses.length} onclick={() => run('phone_pairing')}>{status.phoneCode ? 'New phone code' : status.phonePaired ? 'Pair another phone' : 'Pair a phone'}</button>
          {#if status.phoneCode}
            <div class="frame-code">
              <span class="frame-code-label">Phone pairing code</span>
              <p class="pair-code" aria-label="Phone pairing code">{status.phoneCode}</p>
              <p class="panel-description">Enter this code on the phone or this PC's browser. It is valid for five minutes and can connect multiple phones.</p>
            </div>
          {/if}
          {#if status.phonePaired}<p class="remote-paired">A phone or browser has been paired. Luma remembers it for next time.</p>{/if}
        </div>
      {:else}
        <div class="frame-connect">
          <ol class="remote-steps">
            <li>On your Display Box, open Settings → Luma mode.</li>
            <li>Enter this PC address on the Display Box.</li>
          </ol>
          {#if status.addresses.length}
            <div class="frame-addresses">{#each status.addresses as address (address)}<code class="address">{address}</code>{/each}</div>
          {:else}
            <p class="remote-note">No network address was found. Connect this PC to Wi-Fi and try again.</p>
          {/if}
          <p class="remote-note">Enter the code below on your Display Box.</p>
          <button class="panel-secondary" disabled={busy || !status.addresses.length} onclick={() => run('frame_pairing')}>{status.code ? 'New Display Box code' : status.paired ? 'Pair another Display Box' : 'Pair Display Box'}</button>
          {#if status.code}
            <div class="frame-code">
              <span class="frame-code-label">Display Box pairing code</span>
              <p class="pair-code" aria-label="Display Box pairing code">{status.code}</p>
              <p class="panel-description">Enter this code on the Display Box. It is valid for five minutes. Pairing again replaces the current Display Box.</p>
            </div>
          {/if}
          {#if status.paired}<p class="remote-paired">A Display Box is paired. It can connect whenever remote control is on and Luma is running.</p>{/if}
        </div>
      {/if}
      <details class="remote-help">
        <summary tabindex="0">Can't connect?</summary>
        <p>Make sure the phone or Display Box is on the same private Wi-Fi network as this PC. On a trusted home Wi-Fi network, set the Windows network profile to Private and allow Luma through Windows Firewall. Guest Wi-Fi may block connections between devices. Remote control uses HTTP on port 9877.</p>
      </details>
    {:else}
      <p class="remote-note">Choose Connect a remote to set up your phone or Display Box.</p>
    {/if}
  {:else if !native}<p class="panel-description">Available in the desktop application.</p>{/if}
</section>

<style>
  .remote-qr { display: block; width: min(240px, 100%); aspect-ratio: 1; margin: 0 0 18px; border: 12px solid white; border-radius: 12px; background: white; }
  .remote-choice { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 10px; margin: 18px 0; }
  .remote-choice button { display: grid; gap: 5px; padding: 15px 16px; border: 1px solid #ffffff12; border-radius: 13px; background: #ffffff04; color: #c7ccd8; text-align: left; }
  .remote-choice button:hover { background: #ffffff09; border-color: #ffffff22; }
  .remote-choice button.selected { border-color: var(--accent); background: var(--accent-soft); color: #fff; }
  .remote-choice strong { font-size: clamp(14px, .95vw, 17px); }
  .remote-choice span { color: #8f97a8; font-size: clamp(11px, .75vw, 13px); line-height: 1.45; }
  .remote-choice button.selected span { color: #c8cbd8; }
  .frame-connect { padding: clamp(16px, 1.5vw, 24px); border: 1px solid #ffffff10; border-radius: 14px; background: #0b0e15; }
  .remote-launch { margin: 18px 0 14px; }
  .remote-steps { display: grid; gap: 7px; margin: 0 0 14px; padding-left: 20px; color: #b6bdca; font-size: clamp(13px, .88vw, 16px); line-height: 1.55; }
  .frame-addresses { display: grid; gap: 8px; margin: 0 0 16px; }
  .address { display: block; width: fit-content; max-width: 100%; padding: 9px 12px; border: 1px solid #ffffff14; border-radius: 9px; background: #ffffff06; color: #dfe3eb; font-family: ui-monospace, 'Cascadia Code', Consolas, monospace; font-size: clamp(13px, .95vw, 17px); overflow-wrap: anywhere; }
  .remote-note { margin: 0 0 16px; color: #a6a7b2; font-size: clamp(13px, .88vw, 16px); line-height: 1.6; }
  .remote-paired { margin: 16px 0 0; color: #9ad9b4; font-size: clamp(12px, .85vw, 15px); line-height: 1.5; }
  .frame-code { margin-top: 18px; padding-top: 18px; border-top: 1px solid #ffffff0d; }
  .frame-code-label { color: #8f97a8; font-size: 12px; font-weight: 650; letter-spacing: .04em; }
  .pair-code { margin: 10px 0 12px; color: var(--accent); font-family: ui-monospace, 'Cascadia Code', Consolas, monospace; font-size: clamp(38px, 4.2vw, 76px); font-weight: 700; line-height: 1; letter-spacing: .16em; }
  .frame-code .panel-description { margin: 0; }
  .remote-help { margin-top: 18px; padding-top: 16px; border-top: 1px solid #ffffff0d; color: #a6a7b2; font-size: clamp(12px, .85vw, 15px); line-height: 1.6; }
  .remote-help summary { width: fit-content; cursor: pointer; color: #c7ccd8; font-weight: 600; }
  .remote-help summary:focus-visible { outline: 3px solid var(--accent); outline-offset: 4px; }
  .remote-help p { margin: 10px 0 0; max-width: 70ch; }
  @media (max-width: 640px) { .remote-choice { grid-template-columns: 1fr; } }
</style>
