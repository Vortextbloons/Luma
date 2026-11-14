import type { NavigationCommand } from '../types';

export function createNavigation(back: () => void, home: () => void) {
  const elements = () => [...(document.querySelector('dialog[open]') ?? document.querySelector('main') ?? document).querySelectorAll<HTMLElement>('button:not(:disabled), input:not(:disabled), select:not(:disabled), [tabindex="0"]')].filter(el => el.getClientRects().length > 0);
  function focus(el?: HTMLElement) {
    const target = el?.isConnected && el.getClientRects().length ? el : elements()[0];
    target?.focus({ preventScroll: true });
    target?.scrollIntoView({ block: 'nearest', inline: 'nearest', behavior: 'instant' });
  }
  function dispatch(command: NavigationCommand) {
    if (command === 'back') { back(); return; }
    if (command === 'home') { home(); return; }
    const current = document.activeElement as HTMLElement;
    if ((command === 'left' || command === 'right') && current instanceof HTMLInputElement && current.type === 'range' && !current.disabled) {
      const step = (Number(current.step) || 1) * (Number(current.max) > 10_000 ? 5 : 1);
      const direction = command === 'right' ? 1 : -1;
      const next = Math.min(Number(current.max), Math.max(Number(current.min), Number(current.value) + step * direction));
      current.value = String(next);
      current.dispatchEvent(new Event('input', { bubbles: true }));
      current.dispatchEvent(new Event('change', { bubbles: true }));
      return;
    }
    const candidates = elements();
    if (!candidates.includes(current)) { focus(candidates[0]); return; }
    if (command === 'activate') { current.click(); return; }
    const a = current.getBoundingClientRect();
    const vertical = command === 'up' || command === 'down';
    const sign = command === 'down' || command === 'right' ? 1 : -1;
    let best: HTMLElement | undefined;
    let score = Infinity;
    for (const el of candidates) {
      if (el === current) continue;
      const b = el.getBoundingClientRect();
      const dx = b.x + b.width / 2 - a.x - a.width / 2;
      const dy = b.y + b.height / 2 - a.y - a.height / 2;
      const primary = vertical ? dy : dx;
      const secondary = vertical ? dx : dy;
      const distance = Math.abs(primary) + Math.abs(secondary) * 3;
      if (primary * sign > 4 && distance < score) { best = el; score = distance; }
    }
    if (best) focus(best);
  }
  function keydown(event: KeyboardEvent) {
    if (event.key === 'Tab') {
      const candidates = elements();
      const index = candidates.indexOf(document.activeElement as HTMLElement);
      if (event.shiftKey && index <= 0) { event.preventDefault(); focus(candidates.at(-1)); }
      else if (!event.shiftKey && index === candidates.length - 1) { event.preventDefault(); focus(candidates[0]); }
      return;
    }
    const editing = event.target instanceof HTMLInputElement || event.target instanceof HTMLSelectElement;
    if (editing && event.key !== 'Escape') return;
    const mapping: Record<string, NavigationCommand> = { ArrowUp: 'up', ArrowDown: 'down', ArrowLeft: 'left', ArrowRight: 'right', Enter: 'activate', Escape: 'back', Backspace: 'back', Home: 'home' };
    const command = mapping[event.key];
    if (command) { event.preventDefault(); dispatch(command); }
  }
  return { dispatch, keydown, focus };
}
