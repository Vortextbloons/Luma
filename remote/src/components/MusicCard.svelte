<script lang="ts">
  import Icon from "./Icon.svelte";
  import { music as prefs, setMusicCollapsed } from "../lib/music.svelte";
  import { session } from "../lib/session.svelte";

  let { onsend }: { onsend: (command: string) => Promise<unknown> | unknown } = $props();

  let music = $derived(session.state?.music ?? null);
  let visible = $derived(prefs.enabled && Boolean(music && (music.title || music.playing)));
  let artworkFailed = $state(false);
  let busy = $state(false);

  let artKey = $derived(`${music?.title ?? ""}|${music?.artist ?? ""}`);
  let artSrc = $derived(artKey ? `/phone/music/art?k=${encodeURIComponent(artKey)}` : "/phone/music/art");

  let duration = $derived(music?.durationMs ?? 0);
  let elapsed = $derived.by(() => {
    if (!music) return 0;
    const position = music.positionMs ?? 0;
    const drift = music.playing ? Math.max(0, Date.now() - (music.updatedAt || Date.now())) : 0;
    const total = position + drift;
    return duration > 0 ? Math.min(total, duration) : total;
  });
  let progress = $derived(duration > 0 ? Math.min(1, elapsed / duration) : 0);

  $effect(() => {
    artKey;
    artworkFailed = false;
  });

  function clock(ms: number): string {
    const total = Math.max(0, Math.floor(ms / 1000));
    return `${Math.floor(total / 60)}:${String(total % 60).padStart(2, "0")}`;
  }

  async function fire(command: string): Promise<void> {
    if (busy) return;
    busy = true;
    try {
      await onsend(command);
    } finally {
      busy = false;
    }
  }
</script>

{#if visible && music}
  <section class="music-card" class:is-collapsed={prefs.collapsed} aria-label="Music playing on the PC">
    <div class="music-head">
      <span class="music-copy">
        <b class:is-playing={music.playing}>{music.title ?? "Spotify"}</b>
        <small>{music.artist ?? (music.playing ? "Playing on the PC" : "Paused")}</small>
      </span>
      {#if prefs.collapsed}
        <button
          type="button"
          class="music-inline-key"
          aria-label={music.playing ? "Pause music" : "Play music"}
          aria-pressed={music.playing}
          disabled={busy || !music.canToggle}
          onclick={() => fire("musictoggle")}
        >
          <Icon name={music.playing ? "pause" : "play"} className="size-[16px]" />
        </button>
      {:else}
        <span class="music-state" class:is-playing={music.playing} aria-hidden="true"></span>
      {/if}
      <button
        type="button"
        class="music-toggle"
        aria-expanded={!prefs.collapsed}
        aria-label={prefs.collapsed ? "Show music controls" : "Hide music controls"}
        onclick={() => setMusicCollapsed(!prefs.collapsed)}
      >
        <Icon name="chevron-down" className="size-[15px]" />
      </button>
      <span class="music-art" aria-hidden="true">
        {#if music.hasArtwork && !artworkFailed}
          <img src={artSrc} alt="" decoding="async" onerror={() => (artworkFailed = true)} />
        {:else}
          <Icon name="music" className="size-[18px]" />
        {/if}
      </span>
    </div>

    {#if !prefs.collapsed}
      <div class="music-controls">
        <button
          type="button"
          class="music-key"
          aria-label="Previous track"
          disabled={busy || !music.canSkipPrevious}
          onclick={() => fire("musicprevious")}
        >
          <Icon name="previous" className="size-[17px]" />
        </button>
        <button
          type="button"
          class="music-key is-primary"
          aria-label={music.playing ? "Pause music" : "Play music"}
          aria-pressed={music.playing}
          disabled={busy || !music.canToggle}
          onclick={() => fire("musictoggle")}
        >
          <Icon name={music.playing ? "pause" : "play"} className="size-[19px]" />
        </button>
        <button
          type="button"
          class="music-key"
          aria-label="Next track"
          disabled={busy || !music.canSkipNext}
          onclick={() => fire("musicnext")}
        >
          <Icon name="next" className="size-[17px]" />
        </button>
      </div>

      {#if duration > 0}
        <div class="music-timeline" aria-hidden="true">
          <span class="music-progress"><span style:transform={`scaleX(${progress})`}></span></span>
          <span class="music-time">{clock(elapsed)} / {clock(duration)}</span>
        </div>
      {/if}
    {/if}
  </section>
{/if}

<style>
  .music-card {
    display: grid;
    gap: 10px;
    padding: 11px 12px;
    border: 1px solid var(--line);
    border-radius: 18px;
    background: linear-gradient(180deg, var(--surface), var(--surface-2));
  }

  .music-head {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto auto auto;
    align-items: center;
    gap: 8px;
  }

  .music-card.is-collapsed {
    gap: 0;
    padding: 9px 10px;
  }

  .music-art {
    display: grid;
    place-items: center;
    width: 38px;
    height: 38px;
    overflow: hidden;
    border-radius: 10px;
    border: 1px solid var(--line);
    background: rgba(255, 255, 255, .05);
    color: var(--muted);
  }

  .music-art img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .music-copy {
    display: grid;
    gap: 2px;
    min-width: 0;
  }

  .music-copy b {
    overflow: hidden;
    color: var(--text-dim);
    font-size: 13px;
    font-weight: 650;
    line-height: 1.3;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .music-copy b.is-playing {
    color: var(--text);
  }

  .music-copy small {
    overflow: hidden;
    color: var(--faint);
    font-size: 11.5px;
    line-height: 1.3;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .music-state {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--line-strong);
  }

  .music-state.is-playing {
    background: var(--good);
    box-shadow: 0 0 8px color-mix(in srgb, var(--good) 70%, transparent);
  }

  .music-inline-key {
    display: grid;
    place-items: center;
    width: 32px;
    height: 32px;
    border: 0;
    border-radius: 9px;
    background: var(--accent-soft);
    color: var(--accent-strong);
    transition: background .12s ease, color .12s ease, transform .08s ease;
  }

  .music-inline-key:active:not(:disabled) {
    background: var(--accent);
    color: var(--accent-ink);
    transform: scale(.94);
  }

  .music-inline-key:disabled {
    opacity: .38;
  }

  .music-toggle {
    display: grid;
    place-items: center;
    width: 32px;
    height: 32px;
    border: 0;
    border-radius: 9px;
    background: transparent;
    color: var(--faint);
    transition: background .12s ease, color .12s ease, transform .16s var(--ease-remote);
  }

  .music-toggle:active {
    background: rgba(255, 255, 255, .08);
    color: var(--text-dim);
  }

  .music-toggle[aria-expanded="true"] :global(svg) {
    transform: rotate(180deg);
  }

  .music-controls {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 7px;
  }

  .music-key {
    display: grid;
    place-items: center;
    min-height: 40px;
    border: 0;
    border-radius: 11px;
    background: rgba(255, 255, 255, .045);
    color: var(--muted);
    transition: background .12s ease, color .12s ease, transform .08s ease;
  }

  .music-key:active:not(:disabled) {
    background: rgba(255, 255, 255, .11);
    color: var(--text);
    transform: scale(.96);
  }

  .music-key.is-primary {
    background: var(--accent-soft);
    color: var(--accent-strong);
  }

  .music-key.is-primary:active:not(:disabled) {
    background: var(--accent);
    color: var(--accent-ink);
  }

  .music-key:disabled {
    opacity: .38;
  }

  .music-timeline {
    display: grid;
    gap: 4px;
  }

  .music-progress {
    display: block;
    height: 3px;
    overflow: hidden;
    border-radius: 99px;
    background: var(--line);
  }

  .music-progress span {
    display: block;
    height: 100%;
    border-radius: 99px;
    background: var(--accent);
    transform-origin: left center;
  }

  .music-time {
    color: var(--faint);
    font-family: var(--font-mono);
    font-size: 10px;
    font-variant-numeric: tabular-nums;
  }
</style>
