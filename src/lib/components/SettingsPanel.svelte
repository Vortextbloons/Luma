<script lang="ts">
  import type { RemoteStatus, Settings } from '../types';
  import { moveApp } from '../state/settings.svelte';
  let { settings, remote, native, close, fullscreen, savePaths, network, detect, message, report }: {
    message: string; detect: () => Promise<void>; settings: Settings; remote: RemoteStatus; native: boolean; close: () => void;
    fullscreen: () => void; savePaths: () => Promise<void>;
    network: (lan: boolean, origin: string) => Promise<void>; report: (message: string) => void;
  } = $props();
  let origin = $state('');
  let busy = $state(false);
  let reveal = $state(false);
  function mountDialog(dialog: HTMLDialogElement) { origin = remote.origin; dialog.showModal(); }
  async function wallpaper(event: Event) {
    const file = (event.currentTarget as HTMLInputElement).files?.[0];
    if (!file) return;
    if (!['image/jpeg', 'image/png', 'image/webp', 'image/gif'].includes(file.type) || file.size > 3_000_000) { report('Choose a PNG, JPEG, WebP, or GIF under 3 MB.'); return; }
    const reader = new FileReader();
    reader.onload = () => settings.wallpaper = String(reader.result);
    reader.onerror = () => report('The background image could not be read.');
    reader.readAsDataURL(file);
  }
  async function configure(lan: boolean) {
    busy = true;
    try { await network(lan, origin); } finally { busy = false; }
  }
</script>

<dialog {@attach mountDialog} class="settings-dialog" oncancel={(event) => { event.preventDefault(); close(); }} aria-labelledby="settings-title">
  <div class="panel-heading"><div><span class="panel-kicker">YOUR SPACE, YOUR WAY</span><h2 id="settings-title">Customize Luma</h2></div><button class="close-btn" onclick={close} aria-label="Close settings">×</button></div>
  <p class="panel-description">Preferences are saved on this PC.</p>{#if message}<p role="status" class="panel-feedback">{message}</p>{/if}
  <section class="settings-group"><h3>Display</h3><button class="panel-secondary" disabled={!native} onclick={fullscreen}>{settings.fullscreen ? 'Switch to windowed mode' : 'Switch to fullscreen'}</button><label class="field">Layout density<select bind:value={settings.density}><option value="comfortable">Comfortable</option><option value="compact">Compact</option></select></label></section>
  <section class="settings-group"><h3>Accent color</h3><div class="swatches">{#each [{ id: 'violet', color: '#a59bff' }, { id: 'blue', color: '#72bfff' }, { id: 'mint', color: '#88e6c5' }, { id: 'peach', color: '#ffae92' }] as accent (accent.id)}<button class={['swatch', { 'is-active': settings.accent === accent.id }]} style:--sw={accent.color} aria-label={accent.id} aria-pressed={settings.accent === accent.id} onclick={() => settings.accent = accent.id as Settings['accent']}></button>{/each}</div><label class="field">Background image<input type="file" accept="image/png,image/jpeg,image/webp,image/gif" onchange={wallpaper} /></label><button class="text-link" onclick={() => settings.wallpaper = ''}>Reset background</button></section>
  <section class="settings-group"><h3>Home screen sections</h3>{#each [{ id: 'continue', label: 'Continue watching' }, { id: 'apps', label: 'Your apps' }, { id: 'featured', label: 'Featured for you' }] as section (section.id)}<label class="setting-toggle"><span>{section.label}</span><input type="checkbox" bind:checked={settings.sections[section.id as keyof Settings['sections']]} /></label>{/each}</section>
  <section class="settings-group"><h3>Application paths and tile order</h3><p class="panel-description">Leave Spotify or Stremio blank to detect a standard installation. Custom apps require a local .exe path. No arguments or shell commands.</p><button class="panel-secondary" disabled={!native || busy} onclick={async () => { busy = true; try { await detect(); } finally { busy = false; } }}>Find installed apps</button>{#each settings.order as id, index (id)}<div class="mb-5"><label class="field">{id}<input bind:value={settings.paths[id]} placeholder="C:\Program Files\App\App.exe" spellcheck="false" /></label><div class="flex gap-3"><button class="text-link" disabled={index === 0} onclick={() => moveApp(settings, id, -1)}>← Earlier</button><button class="text-link" disabled={index === settings.order.length - 1} onclick={() => moveApp(settings, id, 1)}>Later →</button></div></div>{/each}<button class="panel-secondary" disabled={!native || busy} onclick={async () => { busy = true; try { await savePaths(); } finally { busy = false; } }}>Save approved app paths</button></section>
  <section class="settings-group"><h3>Luma Remote</h3><p class="remote-status">{native ? remote.running ? `Listening on ${remote.address}` : 'Server stopped' : 'Available in the desktop application'}</p>{#if remote.error}<p class="error">{remote.error}</p>{/if}<p class="panel-description">{remote.lan ? 'LAN access enabled. Every request requires the session token.' : 'Localhost only. LAN access is disabled.'} Credentials reset when Luma restarts. Six-digit pairing is a later milestone.</p><label class="field">Allowed remote web origin<input bind:value={origin} placeholder="https://frame.example" spellcheck="false" /></label><p class="panel-description">Use the exact origin of your remote client. Native clients must send this Origin header too. LAN traffic uses unencrypted HTTP; use a trusted private network.</p><button class="panel-secondary" disabled={!native || busy} onclick={() => configure(!remote.lan)}>{remote.lan ? 'Disable LAN access' : 'Enable authenticated LAN access'}</button><button class="text-link" disabled={!native || busy} onclick={() => configure(remote.lan)}>Apply origin / restart server</button>{#if remote.running}<button class="text-link" onclick={() => reveal = !reveal}>{reveal ? 'Hide' : 'Show'} session credential</button>{#if reveal}<code class="credential">{remote.token}</code><p class="panel-description">Keep this token private. Use Authorization: Bearer &lt;token&gt; for HTTP. WebSocket clients may use the luma.&lt;token&gt; subprotocol.</p>{/if}{/if}</section>
</dialog>
