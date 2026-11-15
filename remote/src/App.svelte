<script lang="ts">
  import { onMount } from "svelte";
  import AppsScreen from "./components/AppsScreen.svelte";
  import BootScreen from "./components/BootScreen.svelte";
  import PairingScreen from "./components/PairingScreen.svelte";
  import RemoteScreen from "./components/RemoteScreen.svelte";
  import ToastHost from "./components/ToastHost.svelte";
  import { initScreen, screen } from "./lib/screen.svelte";
  import { boot, destroy, session } from "./lib/session.svelte";

  onMount(() => {
    const cleanupScreen = initScreen();
    boot();
    return () => {
      cleanupScreen();
      destroy();
    };
  });
</script>

{#if !session.booted || (!session.paired && session.bootError)}
  <BootScreen />
{:else if !session.paired}
  <PairingScreen />
{:else if screen.current === "apps"}
  <AppsScreen />
{:else}
  <RemoteScreen />
{/if}

<ToastHost />
