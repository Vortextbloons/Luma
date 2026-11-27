<script lang="ts">
  import { onMount } from 'svelte';
  import { saverLibrary, type SaverSettings } from '../services/screensaver';
  import type { SpotifyPlaybackState } from '../types/spotify';

  import type { MediaItem } from '../types';

  let { movies, config, playback, reducedMotion }: { movies: MediaItem[]; config: SaverSettings; playback: SpotifyPlaybackState; reducedMotion: boolean } = $props();
  let photos = $state.raw<File[]>([]);
  let video = $state<File>();
  let index = $state(0);
  let now = $state(new Date());
  let source = $state('');
  let failedSource = $state('');
  let playing = $derived(playback.playing && !!playback.title);
  let musicScene = $derived((config.mode === 'music' || config.mode === 'smart') && playing && !(config.mode === 'smart' && config.music && photos.length));
  let photoScene = $derived(config.mode === 'photos' || (config.mode === 'smart' && !musicScene));
  let systemReducedMotion = $state(false);
  let still = $derived(reducedMotion || systemReducedMotion);
  let media = $derived(photoScene ? photos[index] : config.mode === 'video' && !still ? video : undefined);
  let showMusic = $derived(playing && (musicScene || (photoScene && config.music) || config.mode === 'room'));
  let movie = $derived(config.mode === 'movies' ? movies[index % movies.length] : undefined);
  let showClock = $derived(config.mode !== 'black' && (config.clock || config.mode === 'clock' || (!movie && (!media || source === failedSource) && !musicScene && config.mode !== 'room')));

  onMount(() => {
    const motion = window.matchMedia('(prefers-reduced-motion: reduce)');
    const syncMotion = () => { systemReducedMotion = motion.matches; };
    syncMotion();
    motion.addEventListener('change', syncMotion);
    return () => motion.removeEventListener('change', syncMotion);
  });

  onMount(() => {
    if (!['smart', 'photos', 'video'].includes(config.mode)) return;
    let active = true;
    void saverLibrary().then(library => {
      if (!active) return;
      if (config.shuffle) {
        for (let i = library.photos.length - 1; i > 0; i--) {
          const j = Math.floor(Math.random() * (i + 1));
          [library.photos[i], library.photos[j]] = [library.photos[j], library.photos[i]];
        }
      }
      photos = library.photos;
      video = library.video;
    }).catch(() => {});
    return () => { active = false; };
  });

  $effect(() => {
    if (!showClock && config.mode !== 'room') return;
    const timer = setInterval(() => { now = new Date(); }, 15_000);
    return () => clearInterval(timer);
  });

  $effect(() => {
    const length = config.mode === 'movies' ? movies.length : photoScene ? photos.length : 0;
    if (length < 2) return;
    const timer = setInterval(() => { index = (index + 1) % length; }, config.seconds * 1000);
    return () => clearInterval(timer);
  });

  $effect(() => {
    const url = media ? URL.createObjectURL(media) : '';
    source = url;
    return () => { if (url) URL.revokeObjectURL(url); };
  });

</script>

<div class="saver" class:still class:right={config.placement === 'right'} aria-label="Screensaver">
  <div class="scene" style:opacity={config.brightness / 100}>
    {#if movie}
      {#key movie.id}<img class="movie reveal" src={movie.backdrop || movie.poster} alt="" />{/key}
    {:else if musicScene && playback.artworkUrl}
      <div class="album-scene">{#key playback.artworkUrl}<img class="album reveal" src={playback.artworkUrl} alt="" />{/key}</div>
    {:else if photoScene && source && source !== failedSource}
      {#key source}<img class="photo reveal" src={source} alt="" style:object-fit={config.fit} onerror={event => { failedSource = (event.currentTarget as HTMLImageElement).src; }} />{/key}
    {:else if config.mode === 'video' && source && source !== failedSource}
      <video class="reveal" src={source} autoplay muted loop playsinline aria-label="Silent ambient video" onerror={() => { failedSource = source; }}></video>
    {:else if config.mode === 'room'}
      <div class="room" class:night={now.getHours() < 7 || now.getHours() >= 19}>
        <div class="window"><div class="skyline"></div></div>
        <div class="lamp"></div>
        <div class="cabinet"><div class="record" class:spinning={playing && !still}></div></div>
      </div>
    {:else if showClock && !showMusic}
      <div class="clock-glow"></div>
    {/if}
  </div>
  {#if movie || showMusic || showClock}
    <div class="overlay" class:cinema={!!movie} class:has-art={!!media || !!movie || config.mode === 'room' || musicScene} style:opacity={Math.sqrt(config.brightness / 100)}>
      {#if showClock}
        {@const centered = !movie && !showMusic && (!media || source === failedSource) && config.mode !== 'room'}
        <time class:centered>
          {now.toLocaleTimeString([], { hour: 'numeric', minute: '2-digit' })}
          {#if centered}<span>{now.toLocaleDateString([], { weekday: 'long', month: 'long', day: 'numeric' })}</span>{/if}
        </time>
      {/if}
      {#if movie}
        {#key movie.id}
          <div class="caption movie-caption">
            {#if movie.subtitle}<span>{movie.subtitle}</span>{/if}
            <strong>{movie.title}</strong>
          </div>
        {/key}
      {:else if showMusic}
        {#key playback.trackId ?? playback.title}
          <div class="caption track-caption"><strong>{playback.title}</strong>{#if playback.artist}<span>{playback.artist}</span>{/if}</div>
        {/key}
      {/if}
    </div>
  {/if}
</div>

<style>
  .saver { position: fixed; inset: 0; z-index: 2147483647; background: #000; color: #fff; cursor: none; overflow: hidden; }
  .scene, .photo, video, .room, .movie { width: 100%; height: 100%; }
  video { object-fit: contain; }
  .movie { object-fit: cover; }
  .reveal { animation: reveal 780ms ease-out both; }
  @keyframes reveal { from { opacity: 0; } }
  .album-scene { height: 100%; display: grid; place-items: center; background: radial-gradient(ellipse at 50% 34%, #242234, #08090d 72%); }
  .album { width: min(46vh, 72vw); aspect-ratio: 1; object-fit: contain; border-radius: 4px; box-shadow: 0 18px 54px #0009; transform: translateY(-7vh); }
  .overlay { position: absolute; inset: 0; }
  .overlay.has-art { background: linear-gradient(180deg, #0008 0%, transparent 24%, transparent 51%, #000a 80%, #000e 100%); }
  .overlay.cinema { background: linear-gradient(0deg, #050608 0%, #050608dc 15%, #05060856 43%, transparent 68%); }
  .caption, time { position: absolute; inset-inline-start: 6vw; text-shadow: 0 2px 14px #000d; }
  .right :is(.caption, time:not(.centered)) { inset-inline-start: auto; inset-inline-end: 6vw; text-align: end; }
  time { top: 7vh; font-size: clamp(1.5rem, 4vw, 5rem); font-weight: 500; font-variant-numeric: tabular-nums; }
  time.centered { inset: 50% auto auto 50%; transform: translate(-50%, -50%); white-space: nowrap; font-size: clamp(3rem, 11vw, 12rem); letter-spacing: -.055em; }
  time.centered span { display: block; margin-top: .25em; text-align: center; font-size: .22em; letter-spacing: .025em; color: #cfd4ef; }
  .clock-glow { height: 100%; background: radial-gradient(ellipse at center, #222542 0%, #101422 45%, #030406 78%); animation: room-light 22s ease-in-out infinite alternate; }
  .caption { bottom: 8vh; display: grid; gap: .3em; max-width: 86vw; overflow-wrap: anywhere; font-size: clamp(1.35rem, 3.3vw, 4.5rem); line-height: 1.08; }
  .caption strong { font-weight: 750; letter-spacing: -.035em; }
  .caption span { font-size: .58em; color: #f0eef6; font-weight: 500; }
  .movie-caption { max-width: min(86vw, 20ch); font-size: clamp(2rem, 6.5vw, 8.5rem); animation: caption-in 330ms ease-out; }
  .movie-caption span { font-size: clamp(.95rem, 1.55vw, 2.2rem); letter-spacing: .025em; }
  .track-caption { animation: caption-in 330ms ease-out; }
  .still :is(.reveal, .movie-caption, .track-caption, .clock-glow, .lamp, .spinning) { animation: none; }
  @keyframes caption-in { from { transform: translateY(8px); } }
  @media (prefers-reduced-motion: reduce) { .reveal, .movie-caption, .track-caption, .clock-glow, .lamp, .spinning { animation: none; } }
  @keyframes room-light { from { opacity: .55; } to { opacity: 1; } }
  .room { position: relative; background: linear-gradient(to bottom, #3d375b 75%, #252033 75%); }
  .room.night { background: linear-gradient(to bottom, #211d3c 75%, #17152b 75%); }
  .window { position: absolute; width: 32%; height: 40%; top: 14%; left: 18%; border: 1vw solid #79688d; background: linear-gradient(#d77589, #ffd197); box-shadow: inset 0 -.8vw #ba788e; }
  .night .window { background: radial-gradient(circle at 72% 25%, #fff1c8 0 6%, transparent 6.5%), linear-gradient(#202351, #51547f); }
  .skyline { position: absolute; inset: auto 0 0; height: 28%; background: #654964; clip-path: polygon(0 50%, 14% 50%, 14% 8%, 31% 8%, 31% 70%, 45% 70%, 45% 36%, 62% 36%, 62% 57%, 78% 57%, 78% 19%, 91% 19%, 91% 70%, 100% 70%, 100% 100%, 0 100%); }
  .night .skyline { background: #262641; }
  .lamp { position: absolute; inset: auto 5% 42% auto; width: 38%; height: 42%; opacity: .46; background: radial-gradient(ellipse, #f2a57855, transparent 65%); }
  .night .lamp { animation: room-light 14s ease-in-out infinite alternate; }
  .cabinet { position: absolute; bottom: 23%; right: 17%; width: 23%; height: 18%; background: #8b566c; border: 1vw solid #442d48; box-shadow: 0 1vw #171222; display: grid; place-items: center; }
  .record { width: 34%; aspect-ratio: 1; border-radius: 50%; background: repeating-radial-gradient(circle, #211b2e 0 3px, #302941 3px 5px); position: relative; }
  .record::after { content: ''; position: absolute; width: 8%; aspect-ratio: 1; background: #f9c182; top: 14%; left: 45%; }
  .spinning { animation: spin 5s linear infinite; }
  @keyframes spin { to { transform: rotate(1turn); } }
</style>
