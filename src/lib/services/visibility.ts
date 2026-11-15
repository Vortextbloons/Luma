let windowVisible = true;
let documentVisible = typeof document === 'undefined' || !document.hidden;
let last = windowVisible && documentVisible;
const listeners = new Set<(visible: boolean) => void>();

function current() {
  return windowVisible && documentVisible;
}

function changed() {
  const visible = current();
  if (visible === last) return;
  last = visible;
  for (const listener of listeners) listener(visible);
}

if (typeof document !== 'undefined') {
  document.addEventListener('visibilitychange', () => {
    documentVisible = !document.hidden;
    changed();
  });
}

export function appVisible() {
  return current();
}

export function setAppVisible(visible: boolean) {
  windowVisible = visible;
  changed();
}

export function onAppVisible(listener: (visible: boolean) => void) {
  listeners.add(listener);
  listener(current());
  return () => { listeners.delete(listener); };
}
