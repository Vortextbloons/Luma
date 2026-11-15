<script lang="ts">
  import { onMount, untrack } from "svelte";
  import Icon from "./Icon.svelte";
  import RemoteKey from "./RemoteKey.svelte";
  import { api } from "../lib/api";
  import { control } from "../lib/control.svelte";
  import { isTap, stickVector } from "../lib/gestures";
  import { PointerController } from "../lib/pointer";
  import { showPairing } from "../lib/session.svelte";
  import { say } from "../lib/toast.svelte";

  let { onsend }: { onsend: (command: string) => Promise<unknown> | unknown } = $props();

  const owner = globalThis.crypto?.randomUUID?.() ?? `${Date.now().toString(36)}-${Math.random().toString(36).slice(2)}`;
  const pointer = new PointerController({
    owner,
    post: body => api.pointer(body.action, body.owner, body.x, body.y),
    onUnauthorized: () => showPairing("This remote is no longer paired. Choose Connect in Luma for a new code."),
    onError: message => say(message, true),
    onReleased: () => {
      stopVisual(false);
      say("Mouse control is back on your PC. Lift, then drag again to use the touchpad.");
    },
  });

  let padEl = $state<HTMLDivElement>();
  let active = $state(false);
  let stickX = $state(0);
  let stickY = $state(0);
  let pointerId: number | null = null;
  let originX = 0;
  let originY = 0;
  let travel = 0;
  let startedAt = 0;
  let range = $state(0);
  let heartbeat: ReturnType<typeof setInterval> | null = null;

  let touchpad = $derived(control.mode === "touchpad");

  function clearHeartbeat(): void {
    if (heartbeat) clearInterval(heartbeat);
    heartbeat = null;
  }

  function stopVisual(sendStop: boolean): void {
    const wasActive = active;
    active = false;
    pointerId = null;
    stickX = 0;
    stickY = 0;
    clearHeartbeat();
    if (sendStop && wasActive) pointer.stop();
  }

  function begin(event: PointerEvent): void {
    if (!touchpad || pointerId !== null) return;
    if (event.pointerType === "mouse" && event.button !== 0) return;
    event.preventDefault();
    pointerId = event.pointerId;
    active = true;
    travel = 0;
    startedAt = performance.now();
    originX = event.clientX;
    originY = event.clientY;
    stickX = 0;
    stickY = 0;
    range = Math.max(32, (padEl?.clientWidth ?? 0) * 0.22);
    try {
      padEl?.setPointerCapture(event.pointerId);
    } catch (_) {}
    pointer.start();
    clearHeartbeat();
    heartbeat = setInterval(() => {
      if (active) pointer.move(stickX, stickY);
    }, 300);
  }

  function move(event: PointerEvent): void {
    if (event.pointerId !== pointerId || !active) return;
    event.preventDefault();
    const dx = event.clientX - originX;
    const dy = event.clientY - originY;
    travel = Math.max(travel, Math.hypot(dx, dy));
    const vector = stickVector(dx, dy, padEl?.clientWidth ?? 0);
    if (Math.abs(vector.x - stickX) < 0.02 && Math.abs(vector.y - stickY) < 0.02) return;
    stickX = vector.x;
    stickY = vector.y;
    pointer.move(vector.x, vector.y);
  }

  function end(event: PointerEvent): void {
    if (event.pointerId !== pointerId) return;
    event.preventDefault();
    const tap = event.type === "pointerup" && isTap(travel, performance.now() - startedAt);
    stopVisual(true);
    if (tap) void onsend("click");
  }

  function cancel(event: PointerEvent): void {
    if (event.pointerId !== pointerId) return;
    event.preventDefault();
    stopVisual(true);
  }

  let previousMode = control.mode;
  $effect(() => {
    const mode = control.mode;
    if (previousMode === "touchpad" && mode !== "touchpad") untrack(() => stopVisual(true));
    previousMode = mode;
  });

  onMount(() => {
    const blur = () => stopVisual(true);
    const hide = () => {
      if (document.visibilityState !== "visible") stopVisual(true);
    };
    window.addEventListener("blur", blur);
    window.addEventListener("pagehide", blur);
    document.addEventListener("visibilitychange", hide);
    return () => {
      window.removeEventListener("blur", blur);
      window.removeEventListener("pagehide", blur);
      document.removeEventListener("visibilitychange", hide);
      stopVisual(true);
      clearHeartbeat();
    };
  });
</script>

<div class="flex flex-col items-center gap-2.5">
  <div
    bind:this={padEl}
    class="pad"
    class:is-touchpad={touchpad}
    class:is-stick={active}
    role="group"
    aria-label={touchpad ? "Touchpad. Drag to move the PC pointer; tap to click." : "Directional pad"}
    onpointerdown={begin}
    onpointermove={move}
    onpointerup={end}
    onpointercancel={cancel}
    onlostpointercapture={cancel}
  >
    <span class="tick" aria-hidden="true"></span>
    <span
      class="stick"
      style={`--stick-x:${stickX * range}px;--stick-y:${stickY * range}px`}
      aria-hidden="true"
    ></span>

    {#if touchpad}
      <button type="button" tabindex="-1" aria-hidden="true" class="pad-key pad-up opacity-25 pointer-events-none">
        <Icon name="up" className="size-[26px]" />
      </button>
      <button type="button" tabindex="-1" aria-hidden="true" class="pad-key pad-left opacity-25 pointer-events-none">
        <Icon name="left" className="size-[26px]" />
      </button>
      <div class="pad-ok pad-ok-touch" aria-hidden="true"><span class="touch-mark"></span></div>
      <button type="button" tabindex="-1" aria-hidden="true" class="pad-key pad-right opacity-25 pointer-events-none">
        <Icon name="right" className="size-[26px]" />
      </button>
      <button type="button" tabindex="-1" aria-hidden="true" class="pad-key pad-down opacity-25 pointer-events-none">
        <Icon name="down" className="size-[26px]" />
      </button>
    {:else}
      <RemoteKey command="up" label="Up" className="pad-key pad-up" onsend={onsend}>
        <Icon name="up" className="size-[26px]" />
      </RemoteKey>
      <RemoteKey command="left" label="Left" className="pad-key pad-left" onsend={onsend}>
        <Icon name="left" className="size-[26px]" />
      </RemoteKey>
      <RemoteKey command="activate" label="OK" className="pad-key pad-ok" onsend={onsend}>OK</RemoteKey>
      <RemoteKey command="right" label="Right" className="pad-key pad-right" onsend={onsend}>
        <Icon name="right" className="size-[26px]" />
      </RemoteKey>
      <RemoteKey command="down" label="Down" className="pad-key pad-down" onsend={onsend}>
        <Icon name="down" className="size-[26px]" />
      </RemoteKey>
    {/if}
  </div>

  {#if touchpad}
    <p class="pad-hint">Touchpad · Drag to move · Tap to click</p>
  {/if}
</div>

<style>
  .pad {
    position: relative;
    width: var(--pad);
    aspect-ratio: 1;
    padding: 9px;
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    grid-template-rows: repeat(3, 1fr);
    border: 1px solid var(--line-strong);
    border-radius: 50%;
    background: radial-gradient(120% 130% at 50% 0%, var(--surface-3) 0%, var(--surface-2) 46%, var(--bg) 100%);
    box-shadow:
      0 24px 60px rgba(0, 0, 0, .34),
      0 0 calc(var(--pad) * .16) color-mix(in srgb, var(--accent) 11%, transparent),
      inset 0 1px rgba(255, 255, 255, .06);
    touch-action: manipulation;
    -webkit-user-select: none;
    user-select: none;
  }

  .pad.is-touchpad {
    touch-action: none;
  }

  .pad::before {
    content: "";
    position: absolute;
    inset: 7%;
    border: 1px solid var(--line);
    border-radius: 50%;
    opacity: .55;
    pointer-events: none;
  }

  .tick {
    position: absolute;
    top: 6px;
    left: 50%;
    width: 24px;
    height: 2px;
    transform: translateX(-50%);
    border-radius: 99px;
    background: linear-gradient(90deg, transparent, color-mix(in srgb, var(--accent) 72%, transparent), transparent);
    opacity: .65;
    pointer-events: none;
  }

  :global(.pad-key) {
    display: grid;
    place-items: center;
    min-width: 44px;
    min-height: 44px;
    margin: 4px;
    border-radius: 50%;
    color: var(--muted);
    transition: background .12s ease, color .12s ease, transform .08s ease;
  }

  :global(.pad-key:active) {
    background: rgba(255, 255, 255, .07);
    color: var(--text);
    transform: scale(.93);
  }

  :global(.pad-up) { grid-area: 1 / 2; }
  :global(.pad-left) { grid-area: 2 / 1; }
  :global(.pad-ok) { grid-area: 2 / 2; }
  :global(.pad-right) { grid-area: 2 / 3; }
  :global(.pad-down) { grid-area: 3 / 2; }

  :global(.pad-ok) {
    margin: 4px;
    border-radius: 50%;
    background: linear-gradient(160deg, var(--accent-strong), var(--accent));
    color: var(--accent-ink);
    font-size: 13px;
    font-weight: 800;
    letter-spacing: .08em;
    box-shadow:
      inset 0 1px 0 rgba(255, 255, 255, .45),
      0 12px 26px color-mix(in srgb, var(--accent) 28%, transparent);
  }

  :global(.pad-ok:active) {
    background: var(--accent);
    color: var(--accent-ink);
    transform: scale(.93);
  }

  .pad-ok-touch {
    display: grid;
    place-items: center;
    border: 1px solid var(--line);
    background: rgba(255, 255, 255, .025);
    box-shadow: none;
  }

  .touch-mark {
    position: relative;
    width: 18px;
    height: 18px;
    border: 1px solid var(--line-strong);
    border-radius: 50%;
  }

  .touch-mark::before,
  .touch-mark::after {
    content: "";
    position: absolute;
    background: var(--faint);
    opacity: .7;
  }

  .touch-mark::before {
    left: 50%;
    top: -4px;
    bottom: -4px;
    width: 1px;
    transform: translateX(-50%);
  }

  .touch-mark::after {
    top: 50%;
    left: -4px;
    right: -4px;
    height: 1px;
    transform: translateY(-50%);
  }

  .stick {
    position: absolute;
    left: 50%;
    top: 50%;
    width: 18px;
    height: 18px;
    margin: -9px;
    border-radius: 50%;
    background: var(--accent);
    opacity: 0;
    transform: translate(var(--stick-x, 0px), var(--stick-y, 0px));
    box-shadow: 0 0 22px color-mix(in srgb, var(--accent) 70%, transparent);
    transition: opacity .15s ease;
    pointer-events: none;
  }

  .pad.is-touchpad .stick { opacity: .35; }
  .pad.is-touchpad.is-stick .stick { opacity: 1; }

  .pad-hint {
    margin: 0;
    color: var(--faint);
    font-size: 11.5px;
    text-align: center;
  }
</style>
