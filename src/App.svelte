<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import MediaRow from './lib/MediaRow.svelte';
  let settings = $state({ accent: 'violet', density: 'comfortable', fullscreen: false, sections: true, paths: { Spotify: '', Stremio: '', Custom: '' }, order: ['Spotify', 'Stremio', 'Custom'], wallpaper: '' });
  let open = $state(false);
  let message = $state('');
  let previous: HTMLElement | null = null;
  onMount(() => {
    try { const saved = localStorage.getItem('luma-v1'); if (saved) settings = { ...settings, ...JSON.parse(saved) }; } catch { message = 'Unable to load saved preferences.'; }
    if (settings.fullscreen) getCurrentWindow().setFullscreen(true).catch(e => message = String(e));
  });
  $effect(() => {
    document.documentElement.dataset.accent = settings.accent;
    document.documentElement.dataset.density = settings.density;
    try { localStorage.setItem('luma-v1', JSON.stringify(settings)); } catch { message = 'Unable to save preferences.'; }
  });
  async function launch(name: string) {
    try { await invoke('launch_app', { name, path: settings.paths[name as keyof typeof settings.paths] }); message = `${name} started.`; } catch (e) { message = String(e); }
  }
  async function fullscreen() {
    try { await getCurrentWindow().setFullscreen(!settings.fullscreen); settings.fullscreen = !settings.fullscreen; } catch (e) { message = String(e); }
  }
  function close() { open = false; requestAnimationFrame(() => previous?.focus()); }
  function show() { previous = document.activeElement as HTMLElement; open = true; requestAnimationFrame(() => document.querySelector<HTMLButtonElement>('dialog button')?.focus()); }
  function key(event: KeyboardEvent) {
    if (event.key === 'Escape') { close(); return; }
    if (event.target instanceof HTMLInputElement || event.target instanceof HTMLSelectElement) return;
    if (event.key === 'Backspace') { event.preventDefault(); close(); window.scrollTo(0, 0); return; }
    const elements = [...document.querySelectorAll<HTMLElement>(open ? 'dialog button,dialog input,dialog select' : 'main button')].filter(e => e.getClientRects().length && !e.hasAttribute('disabled'));
    if (event.key === 'Home') { event.preventDefault(); close(); elements[0]?.focus(); return; }
    if (!event.key.startsWith('Arrow')) return;
    event.preventDefault();
    const current = document.activeElement as HTMLElement;
    if (!elements.includes(current)) { elements[0]?.focus(); return; }
    const a = current.getBoundingClientRect();
    const vertical = ['ArrowUp','ArrowDown'].includes(event.key);
    const sign = ['ArrowDown','ArrowRight'].includes(event.key) ? 1 : -1;
    let best: HTMLElement | undefined; let score = Infinity;
    for (const e of elements) {
      if (e === current) continue;
      const b = e.getBoundingClientRect(); const dx = b.x+b.width/2-a.x-a.width/2; const dy = b.y+b.height/2-a.y-a.height/2;
      const primary = vertical ? dy : dx; const secondary = vertical ? dx : dy;
      const distance = Math.abs(primary)+Math.abs(secondary)*3;
      if (primary*sign > 4 && distance < score) { best=e; score=distance; }
    }
    best?.focus({preventScroll:true}); best?.scrollIntoView({block:'nearest',inline:'nearest'});
  }
</script>
<svelte:window onkeydown={key}/>
<main class="app-shell">
  <header class="topbar"><span class="brand">? luma<span class="brand-sub">TV</span></span><nav class="nav-pill"><button class="nav-btn is-active" onclick={() => window.scrollTo(0,0)}>Home</button><button class="nav-btn" onclick={() => document.getElementById('apps')?.scrollIntoView()}>Apps</button><button class="nav-btn" onclick={show}>Settings</button></nav></header>
  <section class="hero" style:background-image={settings.wallpaper ? `url(${settings.wallpaper})` : undefined}>
    <div class="hero-art"></div><div class="hero-shade"></div>
    <div class="relative z-10 flex h-full flex-col justify-end p-[6vw] pb-24 max-w-3xl"><p class="eyebrow">FEATURED TONIGHT</p><h1 class="text-6xl font-semibold tracking-tight my-5">Killers of the<br/>Flower Moon</h1><p class="hero-desc">A story of love, greed, and betrayal in the heart of 1920s Oklahoma.</p><div class="flex gap-4 mt-6"><button class="rounded-xl bg-white text-black px-7 py-3" onclick={() => launch('Stremio')}>Open Stremio</button><button class="rounded-xl bg-white/15 px-7 py-3" onclick={() => message = 'Killers of the Flower Moon · Crime drama · 2023. Demo artwork; playback is available through your installed apps.'}>Details</button></div></div>
  </section>
  <section id="apps" class="px-[4.4vw] py-8"><h2 class="text-2xl mb-5">Your apps</h2><div class="flex gap-5">{#each settings.order as name (name)}<button class="flex-1 rounded-2xl bg-white/10 p-10 text-2xl transition-transform hover:scale-105" onclick={() => launch(name)}>{name}</button>{/each}</div></section>
  {#if settings.sections}<MediaRow onselect={(title) => message = `${title} · Demo content. Open Stremio to browse available titles.`}/>{/if}
  {#if message}<p role="status" class="px-[4.4vw] py-6 text-white/80">{message}</p>{/if}
  {#if open}<dialog open class="fixed inset-0 z-50 m-auto max-h-[90vh] w-[min(650px,95vw)] overflow-auto rounded-3xl border border-white/20 bg-[#14151c] p-8 text-white shadow-2xl"><button class="float-right" onclick={close}>Close</button><h2 class="text-3xl mb-6">Customize Luma</h2><div class="grid gap-5">
    <label>Accent <select bind:value={settings.accent}><option>violet</option><option>blue</option><option>mint</option><option>peach</option></select></label>
    <label>Density <select bind:value={settings.density}><option>comfortable</option><option>compact</option></select></label>
    <button onclick={fullscreen}>{settings.fullscreen ? 'Use windowed mode' : 'Use fullscreen'}</button>
    <label><input type="checkbox" bind:checked={settings.sections}/> Show featured titles</label>
    <label>Wallpaper <input type="file" accept="image/*" onchange={async event => { const file=event.currentTarget.files?.[0]; if (!file) return; if (file.size>3000000) { message='Choose an image under 3 MB.'; return; } const reader=new FileReader(); reader.onload=()=>settings.wallpaper=String(reader.result); reader.readAsDataURL(file); }}/></label><button onclick={() => settings.wallpaper=''}>Reset wallpaper</button>
    {#each settings.order as name, index (name)}<label>{name} executable <input class="w-full" bind:value={settings.paths[name as keyof typeof settings.paths]} placeholder="C:\Program Files\…\app.exe"/></label><button disabled={index===0} onclick={() => { const order=[...settings.order]; [order[index-1],order[index]]=[order[index]!,order[index-1]!]; settings.order=order; }}>Move {name} earlier</button>{/each}
    <p>Remote server and Spotify media sessions are not connected in this build.</p>
  </div></dialog>{/if}
</main>
