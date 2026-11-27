import type { ControllerBinds, NavigationCommand } from '../types';

export const controllerActions: { key: NavigationCommand; label: string; description: string }[] = [
  { key: 'activate', label: 'Select / Activate', description: 'Open the focused item or press a button.' },
  { key: 'back', label: 'Back', description: 'Close dialogs or return to the previous screen.' },
  { key: 'home', label: 'Home', description: 'Jump back to the Home tab.' },
  { key: 'up', label: 'Move up', description: 'Move focus up (Left stick up also works).' },
  { key: 'down', label: 'Move down', description: 'Move focus down (Left stick down also works).' },
  { key: 'left', label: 'Move left', description: 'Move focus left (Left stick left also works).' },
  { key: 'right', label: 'Move right', description: 'Move focus right (Left stick right also works).' },
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

export function controllerButtonLabel(index: number | null): string {
  if (index === null) return 'Unbound';
  return buttonNames[index] ?? `Button ${index}`;
}

const repeatable = new Set<NavigationCommand>(['up', 'down', 'left', 'right']);
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
  oncommand: (command: NavigationCommand) => void;
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

  function dispatch(command: NavigationCommand) {
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
