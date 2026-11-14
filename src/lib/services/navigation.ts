import type { NavigationCommand } from '../types';

export function homeShortcut(event: KeyboardEvent) {
  if (event.metaKey || event.isComposing || (!event.ctrlKey && !event.altKey) || !/^(?:[a-z0-9]|Home|F(?:[1-9]|1[0-2]))$/i.test(event.key) || event.key === 'F11') return '';
  return `${event.ctrlKey ? 'Ctrl+' : ''}${event.altKey ? 'Alt+' : ''}${event.shiftKey ? 'Shift+' : ''}${event.key.length === 1 ? event.key.toUpperCase() : event.key}`;
}

export function createNavigation(back: () => void, home: () => void, shortcut: () => string = () => 'Ctrl+H') {
  const elements = () => [...(document.querySelector('dialog[open]') ?? document.querySelector('main') ?? document).querySelectorAll<HTMLElement>('button:not(:disabled), input:not(:disabled), select:not(:disabled), summary, [tabindex="0"]')].filter(el => el.getClientRects().length > 0);
  function focus(el?: HTMLElement) {
    const target = el?.isConnected && el.getClientRects().length ? el : elements()[0];
    target?.focus({ preventScroll: true });
    target?.scrollIntoView({ block: 'nearest', inline: 'nearest', behavior: 'instant' });
  }
  function dispatch(command: NavigationCommand) {
    if (command === 'back') { back(); return; }
    if (command === 'home') { home(); return; }
    const current = document.activeElement as HTMLElement;
    if ((command === 'left' || command === 'right') && current instanceof HTMLSelectElement && !current.disabled) {
      const direction = command === 'right' ? 1 : -1;
      for (let index = current.selectedIndex + direction; index >= 0 && index < current.options.length; index += direction) {
        if (current.options[index].disabled) continue;
        current.selectedIndex = index;
        current.dispatchEvent(new Event('input', { bubbles: true }));
        current.dispatchEvent(new Event('change', { bubbles: true }));
        break;
      }
      return;
    }
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
    if (command === 'activate') {
      if (current instanceof HTMLInputElement && current.type === 'search' && current.form) {
        current.form.requestSubmit();
        return;
      }
      if (!(current instanceof HTMLInputElement && current.type === 'range')) current.click();
      return;
    }
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
    if (event.defaultPrevented || event.isComposing) return;
    if (homeShortcut(event) === shortcut()) { event.preventDefault(); if (!event.repeat) dispatch('home'); return; }
    if (event.key === 'Tab') {
      const candidates = elements();
      const index = candidates.indexOf(document.activeElement as HTMLElement);
      if (event.shiftKey && index <= 0) { event.preventDefault(); focus(candidates.at(-1)); }
      else if (!event.shiftKey && index === candidates.length - 1) { event.preventDefault(); focus(candidates[0]); }
      return;
    }
    const editing = event.target instanceof HTMLInputElement || event.target instanceof HTMLSelectElement || (event.target as HTMLElement)?.tagName === 'TEXTAREA' || (event.target as HTMLElement)?.isContentEditable;
    if (editing && event.key !== 'Escape') return;
    if (event.ctrlKey || event.altKey || event.metaKey) return;
    const mapping: Record<string, NavigationCommand> = { ArrowUp: 'up', ArrowDown: 'down', ArrowLeft: 'left', ArrowRight: 'right', Enter: 'activate', Escape: 'back', Backspace: 'back', Home: 'home' };
    const command = mapping[event.key];
    if (command) { event.preventDefault(); dispatch(command); }
  }
  function focusMusic() {
    const candidates = elements();
    for (const selector of ['.music-home .music-now-actions button', '.music-home .music-starting button', '.music-home button', '.music-home input']) {
      const target = candidates.find(el => el.matches(selector));
      if (target) { focus(target); return; }
    }
    focus();
  }
  return { dispatch, keydown, focus, focusMusic };
}
