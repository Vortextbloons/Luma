import type { SoundLevel, SoundPreset } from '../types';

export type SoundName = 'move' | 'press' | 'back' | 'open' | 'page' | 'error' | 'toggle' | 'volume' | 'startup' | 'shutdown';
type SoundSet = Exclude<SoundPreset, 'off'>;

const assets = import.meta.glob<string>('../../assets/sounds/*/*.wav', { eager: true, query: '?url', import: 'default' });
const GAIN: Record<SoundName, number> = {
  move: .28, press: .42, back: .36, open: .42, page: .3, error: .38, toggle: .24, volume: .26, startup: .45, shutdown: .4,
};
const LEVEL: Record<SoundLevel, number> = { soft: .5, standard: .8, loud: 1.15 };
const MOVE_GAP = 32;

let preset: SoundPreset = 'real';
let level: SoundLevel = 'standard';
let context: AudioContext | undefined;
let master: GainNode | undefined;
let lastMove = 0;
let lastAction = 0;
const buffers = new Map<SoundSet, Partial<Record<SoundName, AudioBuffer>>>();
const loading = new Map<SoundSet, Promise<void>>();

function urls(set: SoundSet) {
  const found: Partial<Record<SoundName, string>> = {};
  for (const [path, url] of Object.entries(assets)) {
    const match = path.match(new RegExp(`/sounds/${set}/([^/]+)\\.wav$`));
    if (match) found[match[1] as SoundName] = url;
  }
  return found;
}

function ensureContext() {
  if (context) return context;
  if (typeof window === 'undefined' || !window.AudioContext) return undefined;
  try {
    context = new AudioContext({ latencyHint: 'interactive' });
    master = context.createGain();
    master.gain.value = .8;
    master.connect(context.destination);
  } catch {
    context = undefined;
    master = undefined;
  }
  return context;
}

function load(set: SoundSet) {
  const current = loading.get(set);
  if (current) return current;
  const ctx = ensureContext();
  if (!ctx) return Promise.resolve();
  const pending = (async () => {
    const decoded = buffers.get(set) ?? {};
    buffers.set(set, decoded);
    await Promise.all(Object.entries(urls(set)).map(async ([name, url]) => {
      try {
        const response = await fetch(url);
        decoded[name as SoundName] = await ctx.decodeAudioData(await response.arrayBuffer());
      } catch {}
    }));
  })();
  loading.set(set, pending);
  return pending;
}

export function setSoundSettings(next: SoundPreset, nextLevel: SoundLevel) {
  preset = next;
  level = nextLevel;
  if (next === 'off') return;
  ensureContext();
  void load(next);
}

export function unlockSound() {
  const ctx = ensureContext();
  if (ctx?.state === 'suspended') void ctx.resume().catch(() => {});
}

export function warmSound() {
  if (preset === 'off') return;
  ensureContext();
  void load(preset);
}

export function soundPlayedRecently(within = 45) {
  return performance.now() - lastAction < within;
}

export function playSound(name: SoundName): Promise<void> {
  if (preset === 'off' || typeof window === 'undefined' || !window.AudioContext) return Promise.resolve();
  const ctx = ensureContext();
  if (!ctx) return Promise.resolve();
  if (ctx.state === 'suspended') void ctx.resume().catch(() => {});
  const set = preset;
  const buffer = buffers.get(set)?.[name];
  if (!buffer) {
    void load(set).then(() => { if (preset === set && buffers.get(set)?.[name]) void playSound(name); });
    return Promise.resolve();
  }
  if (name === 'move') {
    const now = performance.now();
    if (now - lastMove < MOVE_GAP) return Promise.resolve();
    lastMove = now;
  } else {
    lastAction = performance.now();
  }
  try {
    const source = ctx.createBufferSource();
    source.buffer = buffer;
    const gain = ctx.createGain();
    gain.gain.value = GAIN[name] * LEVEL[level];
    source.connect(gain).connect(master!);
    source.start();
    return new Promise(resolve => {
      const done = () => resolve();
      source.addEventListener('ended', done, { once: true });
      setTimeout(done, buffer.duration * 1000 + 60);
    });
  } catch {
    return Promise.resolve();
  }
}

export function previewSounds() {
  unlockSound();
  const sequence: [SoundName, number][] = [
    ['startup', 0], ['move', 1050], ['move', 1170], ['press', 1310], ['open', 1470], ['back', 1750], ['volume', 1910], ['toggle', 2030], ['shutdown', 2210],
  ];
  for (const [name, delay] of sequence) setTimeout(() => { void playSound(name); }, delay);
}

export async function playStartupSound() {
  if (preset === 'off') return;
  const ctx = ensureContext();
  if (!ctx) return;
  await load(preset);
  if (ctx.state === 'suspended') {
    try { await ctx.resume(); } catch { return; }
  }
  if (ctx.state === 'running') void playSound('startup');
}

export async function playShutdownSound() {
  if (preset === 'off') return;
  await load(preset);
  await playSound('shutdown');
}
