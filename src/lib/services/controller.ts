import type { ControllerAction, ControllerBinds, NavigationCommand } from '../types';

export type ControllerFamily = 'xbox' | 'playstation' | 'switch';

export const controllerFamilies: { id: ControllerFamily; label: string }[] = [
  { id: 'xbox', label: 'Xbox' },
  { id: 'playstation', label: 'PlayStation' },
  { id: 'switch', label: 'Switch' },
];

export const controllerActions: { key: ControllerAction; label: string; description: string }[] = [
  { key: 'activate', label: 'Select / Activate', description: 'Open the focused item or press a button.' },
  { key: 'back', label: 'Back', description: 'Close dialogs or return to the previous screen.' },
  { key: 'home', label: 'Home', description: 'Jump back to the Home tab.' },
  { key: 'up', label: 'Move up', description: 'Move focus up (Left stick up also works).' },
  { key: 'down', label: 'Move down', description: 'Move focus down (Left stick down also works).' },
  { key: 'left', label: 'Move left', description: 'Move focus left (Left stick left also works).' },
  { key: 'right', label: 'Move right', description: 'Move focus right (Left stick right also works).' },
  { key: 'tabprev', label: 'Previous tab', description: 'Step back through Home, Sports, Music, Library, and Apps. Cycles settings categories while Settings is open.' },
  { key: 'tabnext', label: 'Next tab', description: 'Step forward through Home, Sports, Music, Library, and Apps. Cycles settings categories while Settings is open.' },
  { key: 'search', label: 'Search', description: 'Focus the search field from any screen.' },
  { key: 'settings', label: 'Settings', description: 'Open Settings, or close it when it is already open.' },
  { key: 'playpause', label: 'Play / pause', description: 'Toggle the current music track without leaving the screen.' },
  { key: 'trackprev', label: 'Previous track', description: 'Skip music back one track.' },
  { key: 'tracknext', label: 'Next track', description: 'Skip music forward one track.' },
  { key: 'scores', label: 'Toggle scores', description: 'Hide or show scores on the Sports tab.' },
];

export function defaultControllerBinds(): ControllerBinds {
  return {
    activate: 0,
    back: 1,
    home: 8,
    up: 12,
    down: 13,
    left: 14,
    right: 15,
    tabprev: 4,
    tabnext: 5,
    search: 3,
    settings: 9,
    playpause: 2,
    trackprev: 6,
    tracknext: 7,
    scores: 11,
  };
}

export function sanitizeControllerBinds(raw: unknown): ControllerBinds {
  const defaults = defaultControllerBinds();
  if (!raw || typeof raw !== 'object') return defaults;
  const input = raw as Record<string, unknown>;
  for (const { key } of controllerActions) {
    const value = input[key];
    if (value === null) defaults[key] = null;
    else if (typeof value === 'number' && Number.isInteger(value) && value >= 0 && value < 32) defaults[key] = value;
  }
  return defaults;
}

const buttonNames: Record<number, string> = {
  0: 'A / Cross',
  1: 'B / Circle',
  2: 'X / Square',
  3: 'Y / Triangle',
  4: 'LB / L1',
  5: 'RB / R1',
  6: 'LT / L2',
  7: 'RT / R2',
  8: 'View / Share',
  9: 'Menu / Options',
  10: 'Left Stick Click',
  11: 'Right Stick Click',
  12: 'D-Pad Up',
  13: 'D-Pad Down',
  14: 'D-Pad Left',
  15: 'D-Pad Right',
  16: 'Guide / Home',
};

const familyFaceLabels: Record<ControllerFamily, [string, string, string, string]> = {
  xbox: ['A', 'B', 'X', 'Y'],
  playstation: ['Cross', 'Circle', 'Square', 'Triangle'],
  switch: ['B', 'A', 'Y', 'X'],
};

const familyButtonLabels: Record<ControllerFamily, Record<number, string>> = {
  xbox: {
    4: 'LB', 5: 'RB', 6: 'LT', 7: 'RT', 8: 'View', 9: 'Menu', 10: 'Left Stick Click', 11: 'Right Stick Click',
    12: 'D-Pad Up', 13: 'D-Pad Down', 14: 'D-Pad Left', 15: 'D-Pad Right', 16: 'Guide',
  },
  playstation: {
    4: 'L1', 5: 'R1', 6: 'L2', 7: 'R2', 8: 'Share', 9: 'Options', 10: 'L3', 11: 'R3',
    12: 'D-Pad Up', 13: 'D-Pad Down', 14: 'D-Pad Left', 15: 'D-Pad Right', 16: 'PS',
  },
  switch: {
    4: 'L', 5: 'R', 6: 'ZL', 7: 'ZR', 8: 'Minus', 9: 'Plus', 10: 'L Stick', 11: 'R Stick',
    12: 'D-Pad Up', 13: 'D-Pad Down', 14: 'D-Pad Left', 15: 'D-Pad Right', 16: 'Home',
  },
};

const familyGlyphs: Record<ControllerFamily, Record<number, string>> = {
  xbox: { 0: 'A', 1: 'B', 2: 'X', 3: 'Y', 4: 'LB', 5: 'RB', 6: 'LT', 7: 'RT', 8: 'V', 9: 'M', 10: 'L3', 11: 'R3', 12: '↑', 13: '↓', 14: '←', 15: '→', 16: 'G' },
  playstation: { 0: '✕', 1: '○', 2: '□', 3: '△', 4: 'L1', 5: 'R1', 6: 'L2', 7: 'R2', 8: 'S', 9: 'O', 10: 'L3', 11: 'R3', 12: '↑', 13: '↓', 14: '←', 15: '→', 16: 'PS' },
  switch: { 0: 'B', 1: 'A', 2: 'Y', 3: 'X', 4: 'L', 5: 'R', 6: 'ZL', 7: 'ZR', 8: '−', 9: '+', 10: 'L3', 11: 'R3', 12: '↑', 13: '↓', 14: '←', 15: '→', 16: 'H' },
};

export function controllerFamily(id: string | null | undefined): ControllerFamily {
  const name = (id ?? '').toLowerCase();
  if (/dualsense|dualshock|playstation|\bps[45]\b|sony/.test(name)) return 'playstation';
  if (/switch|joy-?con|nintendo|pro controller/.test(name)) return 'switch';
  return 'xbox';
}

export function controllerButtonLabel(index: number | null, family?: ControllerFamily): string {
  if (index === null) return 'Unbound';
  if (!family) return buttonNames[index] ?? `Button ${index}`;
  if (index < 4) return familyFaceLabels[family][index];
  return familyButtonLabels[family][index] ?? `Button ${index}`;
}

export function controllerButtonGlyph(index: number | null, family: ControllerFamily): string {
  if (index === null) return '';
  return familyGlyphs[family][index] ?? '?';
}

export function isNavigationCommand(action: ControllerAction): action is NavigationCommand {
  return action === 'up' || action === 'down' || action === 'left' || action === 'right'
    || action === 'activate' || action === 'back' || action === 'home';
}

export const controllerBrandPaths: Record<ControllerFamily, string> = {
  xbox: 'M4.102 21.033C6.211 22.881 8.977 24 12 24c3.026 0 5.789-1.119 7.902-2.967 1.877-1.912-4.316-8.709-7.902-11.417-3.582 2.708-9.779 9.505-7.898 11.417zm11.16-14.406c2.5 2.961 7.484 10.313 6.076 12.912C23.002 17.48 24 14.861 24 12.004c0-3.34-1.365-6.362-3.57-8.536 0 0-.027-.022-.082-.042-.063-.022-.152-.045-.281-.045-.592 0-1.985.434-4.805 3.246zM3.654 3.426c-.057.02-.082.041-.086.042C1.365 5.642 0 8.664 0 12.004c0 2.854.998 5.473 2.661 7.533-1.401-2.605 3.579-9.951 6.08-12.91-2.82-2.813-4.216-3.245-4.806-3.245-.131 0-.223.021-.281.046v-.002zM12 3.551S9.055 1.828 6.755 1.746c-.903-.033-1.454.295-1.521.339C7.379.646 9.659 0 11.984 0H12c2.334 0 4.605.646 6.766 2.085-.068-.046-.615-.372-1.52-.339C14.946 1.828 12 3.545 12 3.545v.006z',
  playstation: 'M8.984 2.596v17.547l3.915 1.261V6.688c0-.69.304-1.151.794-.991.636.18.76.814.76 1.505v5.875c2.441 1.193 4.362-.002 4.362-3.152 0-3.237-1.126-4.675-4.438-5.827-1.307-.448-3.728-1.186-5.39-1.502zm4.656 16.241l6.296-2.275c.715-.258.826-.625.246-.818-.586-.192-1.637-.139-2.357.123l-4.205 1.5V14.98l.24-.085s1.201-.42 2.913-.615c1.696-.18 3.785.03 5.437.661 1.848.601 2.04 1.472 1.576 2.072-.465.6-1.622 1.036-1.622 1.036l-8.544 3.107V18.86zM1.807 18.6c-1.9-.545-2.214-1.668-1.352-2.32.801-.586 2.16-1.052 2.16-1.052l5.615-2.013v2.313L4.205 17c-.705.271-.825.632-.239.826.586.195 1.637.15 2.343-.12L8.247 17v2.074c-.12.03-.256.044-.39.073-1.939.331-3.996.196-6.038-.479z',
  switch: 'M14.176 24h3.674c3.376 0 6.15-2.774 6.15-6.15V6.15C24 2.775 21.226 0 17.85 0H14.1c-.074 0-.15.074-.15.15v23.7c-.001.076.075.15.226.15zm4.574-13.199c1.351 0 2.399 1.125 2.399 2.398 0 1.352-1.125 2.4-2.399 2.4-1.35 0-2.4-1.049-2.4-2.4-.075-1.349 1.05-2.398 2.4-2.398zM11.4 0H6.15C2.775 0 0 2.775 0 6.15v11.7C0 21.226 2.775 24 6.15 24h5.25c.074 0 .15-.074.15-.149V.15c.001-.076-.075-.15-.15-.15zM9.676 22.051H6.15c-2.326 0-4.201-1.875-4.201-4.201V6.15c0-2.326 1.875-4.201 4.201-4.201H9.6l.076 20.102zM3.75 7.199c0 1.275.975 2.25 2.25 2.25s2.25-.975 2.25-2.25c0-1.273-.975-2.25-2.25-2.25s-2.25.977-2.25 2.25z',
};

const repeatable = new Set<ControllerAction>(['up', 'down', 'left', 'right']);
const INITIAL_REPEAT_MS = 360;
const REPEAT_INTERVAL_MS = 110;
const STICK_DEADZONE = 0.55;

export function hideCursor() {
  if (typeof document === 'undefined' || typeof window === 'undefined') return;
  if (typeof document.getElementById !== 'function' || typeof document.createElement !== 'function') return;
  if (document.getElementById('luma-remote-cursor')) return;
  const style = document.createElement('style');
  style.id = 'luma-remote-cursor';
  style.textContent = '*{cursor:none !important}';
  document.documentElement.appendChild(style);
  const show = (event?: PointerEvent) => {
    if (event && event.type === 'pointermove' && !event.movementX && !event.movementY) return;
    style.remove();
    window.removeEventListener('pointermove', show, true);
    const win = window as unknown as { __lumaRemoteCursor?: unknown };
    if (win.__lumaRemoteCursor === show) delete win.__lumaRemoteCursor;
  };
  (window as unknown as { __lumaRemoteCursor?: () => void }).__lumaRemoteCursor = show;
  window.addEventListener('pointermove', show, true);
}

export function connectedControllers(): string[] {
  if (typeof navigator === 'undefined' || typeof navigator.getGamepads !== 'function') return [];
  return [...navigator.getGamepads()].filter((pad): pad is Gamepad => Boolean(pad?.connected)).map(pad => pad.id);
}

export function createController(options: {
  enabled: () => boolean;
  binds: () => ControllerBinds;
  oncommand: (command: ControllerAction) => void;
  oncapture?: (buttonIndex: number) => boolean;
  onstatus?: (names: string[]) => void;
}) {
  let frame = 0;
  let running = false;
  let disposed = false;
  const pressedButtons = new Map<number, Set<number>>();
  const buttonRepeatAt = new Map<string, number>();
  let stickDirection: NavigationCommand | null = null;
  let stickRepeatAt = 0;

  function axisDirection(pad: Gamepad): NavigationCommand | null {
    const x = pad.axes[0] ?? 0;
    const y = pad.axes[1] ?? 0;
    if (Math.abs(x) < STICK_DEADZONE && Math.abs(y) < STICK_DEADZONE) return null;
    if (Math.abs(x) >= Math.abs(y)) return x < 0 ? 'left' : 'right';
    return y < 0 ? 'up' : 'down';
  }

  function dispatch(command: ControllerAction) {
    hideCursor();
    options.oncommand(command);
  }

  function poll(now: number) {
    frame = 0;
    if (disposed) return;
    const pads = typeof navigator !== 'undefined' && typeof navigator.getGamepads === 'function'
      ? [...navigator.getGamepads()].filter((pad): pad is Gamepad => Boolean(pad?.connected))
      : [];
    options.onstatus?.(pads.map(pad => pad.id));
    if (!pads.length) {
      running = false;
      pressedButtons.clear();
      buttonRepeatAt.clear();
      stickDirection = null;
      return;
    }

    const focused = typeof document === 'undefined' || typeof document.hasFocus !== 'function' || document.hasFocus();
    if (focused) {
      const binds = options.binds();
      let activeStick: NavigationCommand | null = null;

      for (const pad of pads) {
        let previous = pressedButtons.get(pad.index);
        if (!previous) {
          previous = new Set();
          pressedButtons.set(pad.index, previous);
        }

        for (let index = 0; index < pad.buttons.length; index++) {
          const isDown = Boolean(pad.buttons[index]?.pressed);
          const wasDown = previous.has(index);
          const repeatKey = `${pad.index}:${index}`;

          if (isDown && !wasDown) {
            previous.add(index);
            if (options.oncapture?.(index)) {
              buttonRepeatAt.delete(repeatKey);
              continue;
            }
            if (!options.enabled()) continue;
            for (const { key } of controllerActions) {
              if (binds[key] === index) {
                dispatch(key);
                if (repeatable.has(key)) buttonRepeatAt.set(repeatKey, now + INITIAL_REPEAT_MS);
              }
            }
          } else if (isDown && wasDown) {
            if (!options.enabled()) continue;
            const nextAt = buttonRepeatAt.get(repeatKey);
            if (nextAt !== undefined && now >= nextAt) {
              for (const { key } of controllerActions) {
                if (binds[key] === index && repeatable.has(key)) dispatch(key);
              }
              buttonRepeatAt.set(repeatKey, now + REPEAT_INTERVAL_MS);
            }
          } else if (!isDown && wasDown) {
            previous.delete(index);
            buttonRepeatAt.delete(repeatKey);
          }
        }

        if (options.enabled() && !activeStick) activeStick = axisDirection(pad);
      }

      if (options.enabled()) {
        if (activeStick !== stickDirection) {
          stickDirection = activeStick;
          if (activeStick) {
            dispatch(activeStick);
            stickRepeatAt = now + INITIAL_REPEAT_MS;
          }
        } else if (activeStick && now >= stickRepeatAt) {
          dispatch(activeStick);
          stickRepeatAt = now + REPEAT_INTERVAL_MS;
        }
      } else {
        stickDirection = null;
      }
    }

    frame = requestAnimationFrame(poll);
  }

  function start() {
    if (disposed || running || typeof requestAnimationFrame !== 'function') return;
    running = true;
    frame = requestAnimationFrame(poll);
  }

  function handleConnect() {
    options.onstatus?.(connectedControllers());
    start();
  }

  function handleDisconnect() {
    const names = connectedControllers();
    options.onstatus?.(names);
    if (!names.length) {
      running = false;
      if (frame) {
        cancelAnimationFrame(frame);
        frame = 0;
      }
      pressedButtons.clear();
      buttonRepeatAt.clear();
      stickDirection = null;
    }
  }

  if (typeof window !== 'undefined') {
    window.addEventListener('gamepadconnected', handleConnect);
    window.addEventListener('gamepaddisconnected', handleDisconnect);
    if (connectedControllers().length) start();
  }

  return {
    start,
    destroy() {
      disposed = true;
      running = false;
      if (frame) cancelAnimationFrame(frame);
      if (typeof window !== 'undefined') {
        window.removeEventListener('gamepadconnected', handleConnect);
        window.removeEventListener('gamepaddisconnected', handleDisconnect);
      }
      pressedButtons.clear();
      buttonRepeatAt.clear();
    },
  };
}
