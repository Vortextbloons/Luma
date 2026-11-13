<script lang="ts">
  import { onMount } from 'svelte';
  let { active, navigate, settings }: { active: string; navigate: (tab: string) => void; settings: () => void } = $props();
  let clock = $state('');
  onMount(() => {
    const update = () => clock = new Date().toLocaleTimeString([], { hour: 'numeric', minute: '2-digit' });
    update();
    const timer = setInterval(update, 60000);
    return () => clearInterval(timer);
  });
</script>

<header class="topbar">
  <span class="brand"><span class="brand-mark">L</span> luma<span class="brand-sub">TV</span></span>
  <nav class="nav-pill" aria-label="Main navigation">
    {#each ['Home', 'Watch', 'Music', 'Apps'] as tab (tab)}
      <button id={tab === 'Home' ? 'home-button' : undefined} class={['nav-btn', { 'is-active': active === tab }]} aria-current={active === tab ? 'page' : undefined} onclick={() => navigate(tab)}>{tab}</button>
    {/each}
  </nav>
  <div class="header-right"><time class="clock">{clock}</time><button class="avatar" aria-label="Open settings" onclick={settings}>⚙</button></div>
</header>
