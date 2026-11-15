<script lang="ts">
  import { onMount } from "svelte";
  import Icon from "./Icon.svelte";
  import RemoteKey from "./RemoteKey.svelte";
  import { api } from "../lib/api";
  import { session } from "../lib/session.svelte";
  import { formatClock, TimelineModel, type TimelineRequest } from "../lib/timeline";
  import { say } from "../lib/toast.svelte";

  let { onsend }: { onsend: (command: string) => Promise<unknown> | unknown } = $props();

  let position = $state(0);
  let duration = $state(0);
  let progress = $derived(duration > 0 ? Math.min(100, Math.max(0, (position / duration) * 100)) : 0);
  let timecode = $derived(duration > 0 ? `${formatClock(position)} / ${formatClock(duration)}` : "");

  const request: TimelineRequest = async seek => {
    const result = await api.playback(seek);
    return { response: result.response, data: result.data };
  };

  const timeline = new TimelineModel(
    request,
    () => session.paired && document.visibilityState === "visible",
    () => {
      position = timeline.position;
      duration = timeline.duration;
    },
    message => say(message, true),
  );

  function input(event: Event): void {
    timeline.preview(Number((event.currentTarget as HTMLInputElement).value));
  }

  function change(event: Event): void {
    timeline.endDrag(Number((event.currentTarget as HTMLInputElement).value));
  }

  function begin(): void {
    timeline.beginDrag();
  }

  function cancel(): void {
    timeline.cancelDrag();
  }

  onMount(() => {
    void timeline.refresh();
    const timer = setInterval(() => {
      if (!timeline.dragging) void timeline.refresh();
    }, 1000);
    return () => clearInterval(timer);
  });
</script>

<section class="deck" aria-label="Playback controls">
  <div class="transport">
    <div class="flex items-baseline justify-between gap-3 px-1">
      <span class="label">Playback</span>
      {#if timecode}<span class="timecode">{timecode}</span>{/if}
    </div>

    <div class="keys">
      <RemoteKey
        command="previous"
        label="Previous track or rewind video 10 seconds"
        className="deck-key"
        onsend={onsend}
      >
        <Icon name="previous" className="size-[21px]" />
      </RemoteKey>
      <RemoteKey command="toggle" label="Play or pause" className="deck-key deck-play" onsend={onsend}>
        <Icon name="play" className="size-[22px]" />
      </RemoteKey>
      <RemoteKey
        command="next"
        label="Next track or advance video 10 seconds"
        className="deck-key"
        onsend={onsend}
      >
        <Icon name="next" className="size-[21px]" />
      </RemoteKey>
    </div>

    <div class="timeline">
      <input
        type="range"
        min="0"
        max={Math.max(duration, 1)}
        step="1"
        value={position}
        disabled={duration <= 0}
        style={`--progress:${progress}%`}
        aria-label="Playback position"
        aria-valuetext={duration > 0 ? `${formatClock(position)} of ${formatClock(duration)}` : "Playback position is unavailable"}
        onpointerdown={begin}
        oninput={input}
        onchange={change}
        onpointercancel={cancel}
      />
      {#if duration <= 0}
        <p class="hint">Start a movie in a supported player to seek.</p>
      {/if}
    </div>
  </div>

  <div class="divider"></div>

  <div class="volume">
    <span class="label">PC volume</span>
    <div class="volume-keys">
      <RemoteKey command="volumedown" label="Volume down" className="deck-volume-key" onsend={onsend}>
        <Icon name="volume-down" className="size-[20px]" />
      </RemoteKey>
      <RemoteKey command="volumeup" label="Volume up" className="deck-volume-key" onsend={onsend}>
        <Icon name="volume-up" className="size-[20px]" />
      </RemoteKey>
      <RemoteKey command="mute" label="Mute" className="deck-volume-key" onsend={onsend}>
        <Icon name="mute" className="size-[20px]" />
      </RemoteKey>
    </div>
  </div>
</section>

<style>
  .deck {
    overflow: hidden;
    border: 1px solid var(--line);
    border-radius: 22px;
    background: linear-gradient(180deg, var(--surface), var(--surface-2));
    box-shadow: 0 14px 34px rgba(0, 0, 0, .22);
  }

  .transport {
    display: grid;
    gap: 8px;
    padding: 11px 11px 10px;
  }

  .label {
    color: var(--faint);
    font-size: 11px;
    font-weight: 600;
    line-height: 1.3;
  }

  .timecode {
    color: var(--muted);
    font-family: var(--font-mono);
    font-size: 10.5px;
    font-variant-numeric: tabular-nums;
    line-height: 1.3;
  }

  .keys {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 8px;
  }

  :global(.deck-key) {
    display: grid;
    place-items: center;
    min-height: 50px;
    border-radius: 14px;
    background: rgba(255, 255, 255, .045);
    color: var(--muted);
    transition: background .12s ease, color .12s ease, transform .08s ease;
  }

  :global(.deck-key:active) {
    background: rgba(255, 255, 255, .11);
    color: var(--text);
    transform: scale(.96);
  }

  :global(.deck-key.deck-play) {
    background: var(--accent);
    color: var(--accent-ink);
    box-shadow: 0 8px 20px color-mix(in srgb, var(--accent) 24%, transparent);
  }

  :global(.deck-key.deck-play:active) {
    background: var(--accent-strong);
    color: var(--accent-ink);
  }

  .timeline {
    min-width: 0;
    padding: 0 2px;
  }

  .hint {
    margin: 0 2px 2px;
    color: var(--faint);
    font-size: 11px;
    line-height: 1.45;
  }

  input[type="range"] {
    display: block;
    width: 100%;
    height: 34px;
    margin: 0;
    appearance: none;
    -webkit-appearance: none;
    background: transparent;
    accent-color: var(--accent);
  }

  input[type="range"]::-webkit-slider-runnable-track {
    height: 4px;
    border-radius: 99px;
    background: linear-gradient(
      90deg,
      var(--accent) 0 var(--progress, 0%),
      var(--line-strong) var(--progress, 0%) 100%
    );
  }

  input[type="range"]::-webkit-slider-thumb {
    width: 18px;
    height: 18px;
    margin-top: -7px;
    appearance: none;
    -webkit-appearance: none;
    border: 2px solid var(--surface-2);
    border-radius: 50%;
    background: var(--accent);
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--accent) 14%, transparent);
  }

  input[type="range"]::-moz-range-track {
    height: 4px;
    border-radius: 99px;
    background: var(--line-strong);
  }

  input[type="range"]::-moz-range-progress {
    height: 4px;
    border-radius: 99px;
    background: var(--accent);
  }

  input[type="range"]::-moz-range-thumb {
    width: 14px;
    height: 14px;
    border: 2px solid var(--surface-2);
    border-radius: 50%;
    background: var(--accent);
  }

  input[type="range"]:disabled {
    opacity: .45;
  }

  .divider {
    height: 1px;
    background: var(--line);
  }

  .volume {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr);
    align-items: center;
    gap: 10px;
    padding: 9px 11px;
  }

  .volume-keys {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 6px;
  }

  :global(.deck-volume-key) {
    display: grid;
    place-items: center;
    min-height: 42px;
    border-radius: 12px;
    background: rgba(255, 255, 255, .03);
    color: var(--muted);
    transition: background .12s ease, color .12s ease, transform .08s ease;
  }

  :global(.deck-volume-key:active) {
    background: rgba(255, 255, 255, .11);
    color: var(--text);
    transform: scale(.96);
  }
</style>
