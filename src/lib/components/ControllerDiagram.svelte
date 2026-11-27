<script lang="ts">
  import {
    controllerActions,
    controllerButtonGlyph,
    controllerButtonLabel,
    controllerBrandPaths,
    controllerFamilies,
    type ControllerFamily,
  } from '../services/controller';
  import type { ControllerAction, ControllerBinds } from '../types';

  type Point = { x: number; y: number };
  type ControlSpec = {
    index: number;
    circle?: { cx: number; cy: number; r: number };
    rect?: { x: number; y: number; w: number; h: number; rx: number };
    path?: { d: string };
    glyph: Point & { text?: string; tiny?: boolean };
    label?: Point & { anchor: 'start' | 'middle' | 'end' };
    caption?: Point & { text: string; anchor?: 'start' | 'middle' | 'end' };
  };

  let {
    family,
    binds,
    recording = null,
    highlight = null,
    oncapture,
    onhover,
  }: {
    family: ControllerFamily;
    binds: ControllerBinds;
    recording?: ControllerAction | null;
    highlight?: ControllerAction | null;
    oncapture?: (action: ControllerAction) => void;
    onhover?: (action: ControllerAction | null) => void;
  } = $props();

  const familyLabel = $derived(controllerFamilies.find(item => item.id === family)?.label ?? family);

  const shortNames: Record<ControllerAction, string> = {
    activate: 'Select', back: 'Back', home: 'Home', up: 'Move', down: 'Move', left: 'Move', right: 'Move',
    tabprev: 'Prev tab', tabnext: 'Next tab', search: 'Search', settings: 'Settings',
    playpause: 'Play / pause', trackprev: 'Prev track', tracknext: 'Next track', scores: 'Scores',
  };

  const triggers: ControlSpec[] = [
    { index: 6, path: { d: 'M171 84 L177 64 C179 58 184 55 191 55 L209 55 C216 55 221 58 223 64 L229 84 Z' }, glyph: { x: 200, y: 71 }, label: { x: 200, y: 44, anchor: 'middle' } },
    { index: 4, rect: { x: 162, y: 84, w: 76, h: 22, rx: 11 }, glyph: { x: 200, y: 95 }, label: { x: 96, y: 97, anchor: 'end' } },
    { index: 7, path: { d: 'M469 84 L463 64 C461 58 456 55 449 55 L431 55 C424 55 419 58 417 64 L411 84 Z' }, glyph: { x: 440, y: 71 }, label: { x: 440, y: 44, anchor: 'middle' } },
    { index: 5, rect: { x: 402, y: 84, w: 76, h: 22, rx: 11 }, glyph: { x: 440, y: 95 }, label: { x: 544, y: 97, anchor: 'start' } },
  ];

  const face: ControlSpec[] = [
    { index: 3, circle: { cx: 450, cy: 140, r: 17 }, glyph: { x: 450, y: 140 }, label: { x: 450, y: 118, anchor: 'middle' } },
    { index: 2, circle: { cx: 416, cy: 174, r: 17 }, glyph: { x: 416, y: 174 }, label: { x: 392, y: 180, anchor: 'end' } },
    { index: 1, circle: { cx: 484, cy: 174, r: 17 }, glyph: { x: 484, y: 174 }, label: { x: 508, y: 180, anchor: 'start' } },
    { index: 0, circle: { cx: 450, cy: 208, r: 17 }, glyph: { x: 450, y: 208 }, label: { x: 450, y: 234, anchor: 'middle' } },
  ];

  const dpad: ControlSpec[] = [
    { index: 12, rect: { x: 177, y: 164, w: 26, h: 26, rx: 6 }, glyph: { x: 190, y: 177 } },
    { index: 13, rect: { x: 177, y: 216, w: 26, h: 26, rx: 6 }, glyph: { x: 190, y: 229 } },
    { index: 14, rect: { x: 151, y: 190, w: 26, h: 26, rx: 6 }, glyph: { x: 164, y: 203 } },
    { index: 15, rect: { x: 203, y: 190, w: 26, h: 26, rx: 6 }, glyph: { x: 216, y: 203 } },
  ];

  const system: ControlSpec[] = [
    { index: 8, rect: { x: 298, y: 134, w: 26, h: 16, rx: 4 }, glyph: { x: 311, y: 142, tiny: true } },
    { index: 9, rect: { x: 342, y: 134, w: 26, h: 16, rx: 4 }, glyph: { x: 355, y: 142, tiny: true } },
  ];

  const sticks: ControlSpec[] = [
    { index: 10, circle: { cx: 282, cy: 225, r: 24 }, glyph: { x: 282, y: 225 }, caption: { x: 282, y: 282, text: 'Left stick', anchor: 'middle' } },
    { index: 11, circle: { cx: 352, cy: 225, r: 24 }, glyph: { x: 352, y: 225 }, caption: { x: 352, y: 282, text: '', anchor: 'middle' } },
  ];

  const controls = [...triggers, ...face, ...dpad, ...system, ...sticks];

  function actionsFor(index: number) {
    return controllerActions.filter(item => binds[item.key] === index);
  }

  function visibleActions(index: number) {
    return actionsFor(index).filter(item => item.key !== 'up' && item.key !== 'down' && item.key !== 'left' && item.key !== 'right');
  }

  function directionalLabel() {
    const directions: ControllerAction[] = ['up', 'down', 'left', 'right'];
    const bound = directions.some(key => [12, 13, 14, 15].some(index => binds[key] === index));
    return bound ? 'Move' : '';
  }

  function actionLabel(index: number) {
    return visibleActions(index).map(item => shortNames[item.key]).join(' · ');
  }
</script>

{#snippet control(spec: ControlSpec)}
  {@const actions = actionsFor(spec.index)}
  {@const first = actions[0]}
  {@const recordingHere = actions.some(item => item.key === recording)}
  {@const highlightHere = actions.some(item => item.key === highlight)}
  <g
    class="control"
    class:bound={actions.length > 0}
    class:recording={recordingHere}
    class:highlight={highlightHere}
    role="button"
    tabindex={first ? 0 : -1}
    aria-disabled={first ? undefined : true}
    aria-label={first ? `${controllerButtonLabel(spec.index, family)}, ${actions.map(item => item.label).join(' and ')}. Rebind.` : controllerButtonLabel(spec.index, family)}
    onclick={first ? () => oncapture?.(first.key) : undefined}
    onkeydown={first ? (event: KeyboardEvent) => { if (event.key === 'Enter' || event.key === ' ') { event.preventDefault(); event.stopPropagation(); oncapture?.(first.key); } } : undefined}
    onmouseenter={() => onhover?.(first?.key ?? null)}
    onmouseleave={() => onhover?.(null)}
  >
    {#if spec.circle}
      <circle class="button" cx={spec.circle.cx} cy={spec.circle.cy} r={spec.circle.r} />
    {:else if spec.rect}
      <rect class="button" x={spec.rect.x} y={spec.rect.y} width={spec.rect.w} height={spec.rect.h} rx={spec.rect.rx} />
    {:else if spec.path}
      <path class="button" d={spec.path.d} />
    {/if}
    <text class="glyph" class:tiny={spec.glyph.tiny} x={spec.glyph.x} y={spec.glyph.y} text-anchor="middle" dominant-baseline="central">{spec.glyph.text ?? controllerButtonGlyph(spec.index, family)}</text>
  </g>
  {#if spec.label}
    {#if actionLabel(spec.index)}
      <text class="action" x={spec.label.x} y={spec.label.y} text-anchor={spec.label.anchor}>{actionLabel(spec.index)}</text>
    {/if}
  {/if}
  {#if spec.caption && (spec.caption.text || (spec.index === 11 && actionLabel(11)))}
    <text class="caption" x={spec.caption.x} y={spec.caption.y} text-anchor={spec.caption.anchor ?? 'middle'}>{spec.caption.text || actionLabel(11)}</text>
  {/if}
{/snippet}

<svg class="controller-diagram" viewBox="0 0 640 340" role="img" aria-label={`${familyLabel} controller map`}>
  <path class="body" d="M150 120C176 92 226 76 320 76C414 76 464 92 490 120C524 158 550 232 532 286C516 322 476 328 444 288L408 279C398 263 384 255 366 255L274 255C256 255 242 263 232 279L196 288C164 328 124 322 108 286C90 232 116 158 150 120Z" />

  <g class="reserved">
    <rect class="button" x="286" y="44" width="68" height="20" rx="10" />
    <text class="glyph tiny" x="320" y="54" text-anchor="middle" dominant-baseline="central">{controllerButtonLabel(16, family)}</text>
  </g>

  <svg class="brand" x="300" y="82" width="40" height="40" viewBox="0 0 24 24" aria-hidden="true">
    <path d={controllerBrandPaths[family]} fill="currentColor" />
  </svg>

  {#each controls as spec (spec.index)}
    {@render control(spec)}
  {/each}

  {#if directionalLabel()}<text class="action" x="190" y="252" text-anchor="middle">{directionalLabel()}</text>{/if}
  <text class="caption" x="311" y="158" text-anchor="middle">{actionLabel(8) || 'Unbound'}</text>
  <text class="caption" x="355" y="158" text-anchor="middle">{actionLabel(9) || 'Unbound'}</text>
</svg>

<style>
  .controller-diagram { display: block; width: 100%; height: auto; overflow: visible; }
  .body { fill: color-mix(in srgb, var(--surface-3) 55%, var(--surface-2)); stroke: #ffffff26; stroke-width: 1.6; }
  .control .button, .reserved .button { fill: color-mix(in srgb, #ffffff 6%, var(--surface-3)); stroke: #ffffff2b; stroke-width: 1.3; transition: fill .15s, stroke .15s, filter .15s; }
  .control.bound .button { fill: color-mix(in srgb, var(--accent) 12%, var(--surface-3)); stroke: color-mix(in srgb, var(--accent) 42%, #ffffff22); }
  .control.highlight .button { fill: color-mix(in srgb, var(--accent) 22%, var(--surface-3)); stroke: var(--accent); }
  .control.recording .button { fill: color-mix(in srgb, var(--accent) 28%, var(--surface-3)); stroke: var(--accent); stroke-width: 2.2; filter: drop-shadow(0 0 7px var(--accent-soft)); }
  .control[role='button'] { cursor: pointer; }
  .control[role='button']:focus-visible { outline: none; }
  .control[role='button']:focus-visible .button { stroke: var(--accent); stroke-width: 2.4; }
  .glyph { fill: var(--text-dim); font-size: 12px; font-weight: 700; pointer-events: none; }
  .control.bound .glyph { fill: var(--text); }
  .glyph.tiny { font-size: 9px; }
  .action { fill: var(--muted); font-size: 10px; font-weight: 600; pointer-events: none; }
  .control.bound + .action { fill: var(--text-dim); }
  .caption { fill: var(--dim); font-size: 10px; font-weight: 500; pointer-events: none; }
  .brand { color: var(--text-dim); opacity: .55; pointer-events: none; }
  .reserved { opacity: .55; }
</style>
