import type { SoundPreset } from "./types";

const ROLES = ["move", "press", "back", "volume", "open", "toggle"] as const;
type SoundRole = (typeof ROLES)[number];

const GAIN: Record<SoundRole, number> = {
  move: 0.3,
  press: 0.5,
  back: 0.4,
  volume: 0.34,
  open: 0.5,
  toggle: 0.3,
};

const ROLE_FOR_COMMAND: Record<string, SoundRole> = {
  up: "move",
  down: "move",
  left: "move",
  right: "move",
  activate: "press",
  click: "press",
  toggle: "toggle",
  previous: "press",
  next: "press",
  back: "back",
  home: "back",
  volumeup: "volume",
  volumedown: "volume",
  mute: "volume",
  fullscreen: "press",
  stop: "press",
};

export interface SoundFeedback {
  setPreset(preset: SoundPreset): void;
  play(command: string): void;
}

export function createSoundFeedback(): SoundFeedback {
  let preset: SoundPreset = "real";
  let context: AudioContext | null = null;
  let master: GainNode | null = null;
  let lastMove = 0;
  const buffers = new Map<SoundPreset, Partial<Record<SoundRole, AudioBuffer>>>();

  function audioContext(): AudioContext | null {
    if (!context) {
      const AudioCtor = window.AudioContext
        ?? (window as unknown as { webkitAudioContext?: typeof AudioContext }).webkitAudioContext;
      if (!AudioCtor) return null;
      try {
        context = new AudioCtor({ latencyHint: "interactive" });
        master = context.createGain();
        master.gain.value = 0.9;
        master.connect(context.destination);
      } catch (_) {
        context = null;
        master = null;
        return null;
      }
    }
    if (context.state === "suspended") void context.resume().catch(() => {});
    return context;
  }

  function load(target: SoundPreset): void {
    if (target === "off" || buffers.has(target)) return;
    const audio = audioContext();
    if (!audio) return;
    const set: Partial<Record<SoundRole, AudioBuffer>> = {};
    buffers.set(target, set);
    for (const role of ROLES) {
      fetch(`/phone/sound/${target}/${role}.wav`, { credentials: "same-origin" })
        .then(response => (response.ok ? response.arrayBuffer() : null))
        .then(bytes => (bytes ? audio.decodeAudioData(bytes) : null))
        .then(buffer => {
          if (buffer) set[role] = buffer;
        })
        .catch(() => {});
    }
  }

  function play(command: string): void {
    if (preset === "off") return;
    try {
      const audio = audioContext();
      if (!audio || !master) return;
      const role = ROLE_FOR_COMMAND[command] || "open";
      const buffer = buffers.get(preset)?.[role];
      if (!buffer) {
        load(preset);
        return;
      }
      if (role === "move") {
        const now = performance.now();
        if (now - lastMove < 32) return;
        lastMove = now;
      }
      const source = audio.createBufferSource();
      source.buffer = buffer;
      const gain = audio.createGain();
      gain.gain.value = GAIN[role];
      source.connect(gain).connect(master);
      source.start();
    } catch (_) {}
  }

  function setPreset(target: SoundPreset): void {
    preset = target;
    load(target);
  }

  return { setPreset, play };
}
