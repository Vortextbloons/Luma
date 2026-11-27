<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  let { native, autoConnect = false, pairingOnly = false }: { native: boolean; autoConnect?: boolean; pairingOnly?: boolean } = $props();
  type Status = { enabled: boolean; paired: boolean; addresses: string[]; pcName: string; code: string | null; phonePaired: boolean; phoneUrl: string; phoneCode: string | null };
  type Device = 'phone' | 'frame';
  type RemoteDevice = 'display' | 'phone';
  let status = $state<Status | null>(null);
  let busy = $state(false);
  let error = $state('');
  let note = $state('');
  let confirming = $state<RemoteDevice | 'all' | null>(null);
  let device = $state<Device>('phone');
  let connecting = $state(false);
  let qr = $state('');
  let timer: ReturnType<typeof setTimeout> | undefined;
  let checking = false;
  let disposed = false;
  function delay() { return status?.code || status?.phoneCode ? 3000 : 15000; }
  function schedule() {
    if (!native || disposed) return;
    if (timer) clearTimeout(timer);
    timer = setTimeout(async () => {
      if (!busy && !checking) {
        checking = true;
        try { status = await invoke<Status>('frame_status'); }
        catch (reason) { error = String(reason); }
        finally { checking = false; }
      }
      schedule();
    }, delay());
  }
  async function connect(selected: Device = device) {
    device = selected;
    busy = true;
    error = '';
    note = '';
    confirming = null;
    qr = '';
    try {
      if (!status?.enabled) status = await invoke<Status>('configure_frame', { enabled: true });
      status = await invoke<Status>(selected === 'phone' ? 'phone_pairing' : 'frame_pairing');
      connecting = true;
      if (selected === 'phone' && status.addresses.length) qr = await invoke<string>('phone_remote_qr');
    } catch (reason) { error = String(reason); }
    finally { busy = false; schedule(); }
  }
  const badge = $derived(status?.enabled
    ? status.paired && status.phonePaired
      ? 'Display Box and web remote paired'
      : status.paired
        ? 'Display Box paired'
        : status.phonePaired
          ? 'Web remote paired'
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
  async function forgetDevice(kind: RemoteDevice) {
    if (confirming !== kind) {
      confirming = kind;
      note = `Choose Forget ${kind === 'display' ? 'Display Box' : 'web remote'} again to confirm. The other device stays paired.`;
      return;
    }
    await run('forget_device', { kind });
    if (!error) {
      confirming = null;
      note = kind === 'display'
        ? 'Display Box forgotten. Pair it again with a new code.'
        : 'Web remote forgotten. Every paired browser will need a new code.';
    }
  }
  async function forgetAll() {
    if (confirming !== 'all') {
      confirming = 'all';
      note = 'Choose Forget all remotes again to confirm. Your preferences and accounts are not changed.';
      return;
    }
    await run('forget_remotes');
    if (!error) {
      confirming = null;
      note = 'All paired remotes forgotten. Pair a phone or Display Box again when you need one.';
    }
  }
  async function toggleRemote(event: Event) {
    const input = event.currentTarget as HTMLInputElement;
    note = '';
    confirming = null;
    await run('configure_frame', { enabled: input.checked });
    if (status?.enabled !== input.checked) input.checked = Boolean(status?.enabled);
  }
  async function newPhoneCode() {
    busy = true;
    error = '';
    try {
      status = await invoke<Status>('phone_pairing');
      qr = status.addresses.length ? await invoke<string>('phone_remote_qr') : '';
    } catch (reason) { error = String(reason); }
    finally { busy = false; schedule(); }
  }
  async function openRemote() {
    busy = true;
    error = '';
    try { await invoke('open_phone_remote'); }
    catch (reason) { error = String(reason); }
    finally { busy = false; }
  }
  $effect(() => {
    if (status && !status.phoneCode && qr) qr = '';
  });
  onMount(() => {
    if (!native) return;
    void (autoConnect ? connect() : run('frame_status'));
    schedule();
    return () => { disposed = true; if (timer) clearTimeout(timer); };
  });
</script>

<section class:pairing-only={pairingOnly} class="settings-group">
  {#if !pairingOnly}
  <div class="settings-group-head">
    <h3>Web remote &amp; Display Box</h3>
    {#if status}<span class={`settings-badge ${tone}`}>{badge}</span>{/if}
  </div>
  <p class="panel-description">Pair a phone browser or Display Box to control Luma.</p>
  <p class="panel-description">Using a VPN? Allow local network access in your VPN settings so your web remote and Display Box can reach this PC.</p>
  {/if}
  {#if error}<p role="alert" class="error">{error}</p>{/if}
  {#if status}
    {#if !pairingOnly}
    <button class="panel-secondary is-primary" disabled={busy} onclick={() => connect()}>Connect a remote</button>
    <label class="setting-toggle">
      <span class="setting-toggle-copy"><span>Allow remote control</span><small>Paired devices can control Luma while it is running.</small></span>
      <input type="checkbox" checked={status.enabled} disabled={busy} onchange={toggleRemote} />
    </label>
    {#if status.enabled}
      <div class="settings-device-list" aria-label="Connected devices">
        <div class="settings-device">
          <div class="settings-device-copy"><strong>Display Box</strong><span>{status.paired ? 'Paired frame on this PC.' : 'No Display Box is paired.'}</span></div>
          <span class={status.paired ? 'settings-badge is-good' : 'settings-badge is-muted'}>{status.paired ? 'Paired' : 'Not paired'}</span>
          {#if status.paired}<button class="text-link is-danger" disabled={busy} onclick={() => forgetDevice('display')}>{confirming === 'display' ? 'Confirm forget Display Box' : 'Forget Display Box'}</button>{/if}
        </div>
        <div class="settings-device">
          <div class="settings-device-copy"><strong>Web remote</strong><span>{status.phonePaired ? 'Paired browsers can control Luma.' : 'No web remote is paired.'}</span></div>
          <span class={status.phonePaired ? 'settings-badge is-good' : 'settings-badge is-muted'}>{status.phonePaired ? 'Paired' : 'Not paired'}</span>
          {#if status.phonePaired}<button class="text-link is-danger" disabled={busy} onclick={() => forgetDevice('phone')}>{confirming === 'phone' ? 'Confirm forget web remote' : 'Forget web remote'}</button>{/if}
        </div>
      </div>
      {#if note}<p class="panel-description" role="status">{note}</p>{/if}
    {/if}
    {/if}
    {#if status.enabled && connecting}
      <div class="remote-choice" role="group" aria-label="Choose a device">
        <button disabled={busy} class:selected={device === 'phone'} aria-pressed={device === 'phone'} onclick={() => connect('phone')}>
          <strong>Web remote</strong><span>Use a browser on any phone, tablet, or computer.</span>
        </button>
        <button disabled={busy} class:selected={device === 'frame'} aria-pressed={device === 'frame'} onclick={() => connect('frame')}>
          <strong>Display Box</strong><span>Use a Luma Display Box frame.</span>
        </button>
      </div>
      {#if device === 'phone'}
        <div class="frame-connect">
          {#if status.phoneCode}
            <div class="frame-code">
              <span class="frame-code-label">Web remote pairing code</span>
              <p class="pair-code" aria-label="Web remote pairing code">{status.phoneCode}</p>
              <p class="panel-description">Scan the QR code to fill this in automatically, or type it in the web remote. Valid for five minutes; you can pair multiple browsers.</p>
            </div>
          {/if}
          {#if qr}<img class="remote-qr" src={qr} alt="Scan to open and pair the Luma remote" decoding="async" /><p class="remote-note">Scan with a camera to open and pair the web remote. Keep both devices on the same Wi-Fi.</p>{/if}
          <ol class="remote-steps">
            <li>Scan the QR code, or type this address in a browser on your device.</li>
            <li>The QR code pairs automatically. You can also enter the pairing code shown below.</li>
            <li>After it opens, add Luma to your phone&rsquo;s home screen for one-tap access.</li>
          </ol>
          {#if status.addresses.length}
            <code class="address">{status.phoneUrl}</code>
            {#if status.addresses.length > 1}
              <p class="remote-note">If that address does not open, try the other one for this PC:</p>
              <div class="frame-addresses">{#each status.addresses.slice(1) as address (address)}<code class="address">http://{address}:9877</code>{/each}</div>
            {/if}
          {:else}
            <p class="remote-note">No network address was found. Connect this PC to Wi-Fi and try again.</p>
          {/if}
          <div class="remote-launch"><button class="panel-secondary" disabled={busy} onclick={openRemote}>Open remote in browser</button></div>
          <p class="remote-note">On another device, use the address above, not localhost. Keep Luma running and both devices on the same Wi-Fi.</p>
          <button class="panel-secondary" disabled={busy || !status.addresses.length} onclick={newPhoneCode}>{status.phoneCode ? 'New web remote code' : status.phonePaired ? 'Pair another browser' : 'Pair a web remote'}</button>
          {#if status.phonePaired}<p class="remote-paired">A web remote has been paired. Luma remembers it for next time.</p>{/if}
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
        <p>Make sure the web remote or Display Box is on the same private Wi-Fi network as this PC. On a trusted home Wi-Fi network, set the Windows network profile to Private and allow Luma through Windows Firewall. Guest Wi-Fi may block connections between devices. Remote control uses HTTP on port 9877.</p>
      </details>
    {:else}
      <button class="panel-secondary" disabled={busy} onclick={() => connect()}>Connect a remote</button>
    {/if}
    {#if !pairingOnly && (status.paired || status.phonePaired)}
      <button class="panel-secondary" disabled={busy} onclick={forgetAll}>{confirming === 'all' ? 'Confirm forget all remotes' : 'Forget all remotes'}</button>
      <p class="remote-note">Paired browsers and Display Boxes will need a new code. Your preferences and accounts are not changed.</p>
    {/if}
  {:else if busy}<p class="remote-note" role="status">Preparing your connection?</p>
  {:else if native}<button class="panel-secondary" disabled={busy} onclick={() => connect()}>Try again</button>
  {:else if !native}<p class="panel-description">Available in the desktop application.</p>{/if}
</section>

<style>
  .pairing-only { border-top: 0; padding: 0; }
  .pairing-only .frame-connect { display: flow-root; }
  .pairing-only .remote-qr { float: right; width: min(210px, 36%); margin: 0 0 18px 24px; }
  .pairing-only .frame-code { margin: 0 0 20px; padding: 0; border: 0; }
  .remote-qr { display: block; width: min(240px, 100%); aspect-ratio: 1; margin: 0 0 18px; border: 12px solid white; border-radius: 12px; background: white; }
  .settings-device-list { display: grid; gap: 8px; margin: 16px 0; }
  .settings-device { display: flex; align-items: center; justify-content: space-between; gap: 14px; flex-wrap: wrap; padding: 13px 15px; border: 1px solid var(--line); border-radius: 12px; background: #ffffff05; }
  .settings-device-copy { display: grid; gap: 3px; min-width: 0; }
  .settings-device-copy strong { color: var(--text); font-size: clamp(13px, .9vw, 16px); }
  .settings-device-copy span { color: var(--muted); font-size: clamp(12px, .8vw, 14px); line-height: 1.45; }
  .remote-choice { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 10px; margin: 18px 0; }
  .remote-choice button { display: grid; gap: 5px; padding: 15px 16px; border: 1px solid var(--line); border-radius: 13px; background: #ffffff04; color: var(--text-dim); text-align: left; }
  .remote-choice button:hover { background: #ffffff09; border-color: #ffffff22; }
  .remote-choice button.selected { border-color: var(--accent); background: var(--accent-soft); color: #fff; }
  .remote-choice strong { font-size: clamp(14px, .95vw, 17px); }
  .remote-choice span { color: var(--muted); font-size: clamp(11px, .75vw, 13px); line-height: 1.45; }
  .remote-choice button.selected span { color: var(--text-dim); }
  .frame-connect { padding: clamp(16px, 1.5vw, 24px); border: 1px solid #ffffff10; border-radius: 14px; background: var(--bg); }
  .remote-launch { margin: 18px 0 14px; }
  .remote-steps { display: grid; gap: 7px; margin: 0 0 14px; padding-left: 20px; color: var(--text-dim); font-size: clamp(13px, .88vw, 16px); line-height: 1.55; }
  .frame-addresses { display: grid; gap: 8px; margin: 0 0 16px; }
  .address { display: block; width: fit-content; max-width: 100%; padding: 9px 12px; border: 1px solid var(--line); border-radius: 9px; background: #ffffff06; color: var(--text); font-family: ui-monospace, 'Cascadia Code', Consolas, monospace; font-size: clamp(13px, .95vw, 17px); overflow-wrap: anywhere; }
  .remote-note { margin: 0 0 16px; color: var(--muted); font-size: clamp(13px, .88vw, 16px); line-height: 1.6; }
  .remote-paired { margin: 16px 0 0; color: var(--good); font-size: clamp(12px, .85vw, 15px); line-height: 1.5; }
  .frame-code { margin-top: 18px; padding-top: 18px; border-top: 1px solid #ffffff0d; }
  .frame-code-label { color: var(--muted); font-size: 12px; font-weight: 650; letter-spacing: .04em; }
  .pair-code { margin: 10px 0 12px; color: var(--accent); font-family: ui-monospace, 'Cascadia Code', Consolas, monospace; font-size: clamp(38px, 4.2vw, 76px); font-weight: 700; line-height: 1; letter-spacing: .16em; }
  .frame-code .panel-description { margin: 0; }
  .remote-help { margin-top: 18px; padding-top: 16px; border-top: 1px solid #ffffff0d; color: var(--muted); font-size: clamp(12px, .85vw, 15px); line-height: 1.6; }
  .remote-help summary { width: fit-content; cursor: pointer; color: var(--text-dim); font-weight: 600; }
  .remote-help summary:focus-visible { outline: 3px solid var(--accent); outline-offset: 4px; }
  .remote-help p { margin: 10px 0 0; max-width: 70ch; }
  @media (max-width: 640px) { .remote-choice { grid-template-columns: 1fr; } }
</style>
