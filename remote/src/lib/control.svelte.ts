export type ControlMode = "dpad" | "touchpad";

const CONTROL_KEY = "luma-joystick";

function initialMode(): ControlMode {
  try {
    return localStorage.getItem(CONTROL_KEY) === "true" ? "touchpad" : "dpad";
  } catch (_) {
    return "dpad";
  }
}

export const control = $state({
  mode: initialMode() as ControlMode,
});

export function setControlMode(mode: ControlMode): void {
  control.mode = mode;
  try {
    localStorage.setItem(CONTROL_KEY, String(mode === "touchpad"));
  } catch (_) {}
}
