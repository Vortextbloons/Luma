import type { NavigationCommand } from '../types';

const focusable = 'button:not(:disabled), input:not(:disabled):not([type="hidden"]), select:not(:disabled), textarea:not(:disabled), a[href], summary, [tabindex="0"]';

export function preserveHomeFocus(root: HTMLElement) {
  let last: { element: HTMLElement; instance: string; key: string | undefined; controlIndex: number; rowIndex: number } | undefined;
  const rows = () => [...root.querySelectorAll<HTMLElement>('[data-home-instance]')];
  const controls = (row: HTMLElement) => [...row.querySelectorAll<HTMLElement>(focusable)].filter(element => element.getClientRects().length > 0 && !element.closest('[hidden], [inert]'));
  const remember = (event: FocusEvent) => {
    const element = event.target as HTMLElement;
    const row = element.closest?.<HTMLElement>('[data-home-instance]');
    if (!row || !root.contains(row)) { last = undefined; return; }
    last = { element, instance: row.dataset.homeInstance!, key: element.dataset.focusKey, controlIndex: controls(row).indexOf(element), rowIndex: rows().indexOf(row) };
  };
  const observer = new MutationObserver(() => {
    if (!last || !root.isConnected || document.querySelector('dialog[open]')) return;
    const retained = last.element.isConnected && last.element.getClientRects().length > 0 && !last.element.matches(':disabled') && !last.element.closest('[hidden], [inert]');
    if (retained && document.activeElement === last.element) return;
    if (document.activeElement !== document.body && document.activeElement !== last.element) return;
    const currentRows = rows();
    const sameRow = currentRows.find(row => row.dataset.homeInstance === last!.instance);
    const candidates = sameRow ? controls(sameRow) : [];
    let target: HTMLElement | undefined = (retained ? last.element : undefined) ?? candidates.find(element => last!.key !== undefined && element.dataset.focusKey === last!.key) ?? candidates[Math.min(Math.max(0, last.controlIndex), candidates.length - 1)];
    if (!target) {
      const nearest = currentRows.map((row, index) => ({ row, index })).sort((a, b) => Math.abs(a.index - last!.rowIndex) - Math.abs(b.index - last!.rowIndex));
      for (const { row } of nearest) {
        target = controls(row)[0];
        if (target) break;
      }
    }
    target ??= document.querySelector<HTMLElement>('#home-button') ?? undefined;
    target?.focus({ preventScroll: true });
    target?.scrollIntoView({ block: 'nearest', inline: 'nearest', behavior: 'instant' });
  });
  document.addEventListener('focusin', remember);
  observer.observe(root, { childList: true, subtree: true, attributes: true, attributeFilter: ['hidden', 'inert', 'disabled', 'style', 'class'] });
  return () => { observer.disconnect(); document.removeEventListener('focusin', remember); last = undefined; };
}

export function homeShortcut(event: KeyboardEvent) {
  if (event.metaKey || event.isComposing || (!event.ctrlKey && !event.altKey) || !/^(?:[a-z0-9]|Home|F(?:[1-9]|1[0-2]))$/i.test(event.key) || event.key === 'F11') return '';
  return `${event.ctrlKey ? 'Ctrl+' : ''}${event.altKey ? 'Alt+' : ''}${event.shiftKey ? 'Shift+' : ''}${event.key.length === 1 ? event.key.toUpperCase() : event.key}`;
}

export function createNavigation(back: () => void, home: () => void, shortcut: () => string = () => 'Ctrl+W', sound: (name: 'move' | 'press' | 'volume') => void = () => {}) {
  let cached: HTMLElement[] | undefined;
  let cachedRoot: Element | Document | undefined;
  let observer: MutationObserver | undefined;
  const invalidate = () => { cached = undefined; cachedRoot = undefined; };
  const elements = () => {
    const root = document.querySelector('dialog[open]') ?? document.querySelector('main') ?? document;
    if (cached && cachedRoot === root && cached[0]?.isConnected) return cached;
    if (!observer && typeof MutationObserver !== 'undefined') {
      observer = new MutationObserver(invalidate);
      observer.observe(document.body, { childList: true, subtree: true, attributes: true, attributeFilter: ['class', 'disabled', 'hidden', 'open', 'inert', 'style', 'tabindex'] });
      window.addEventListener('resize', invalidate, { passive: true });
    }
    cachedRoot = root;
    cached = [...root.querySelectorAll<HTMLElement>(focusable)].filter(el => el.getClientRects().length > 0 && !el.closest?.('[inert], [hidden]'));
    return cached;
  };
  function focus(el?: HTMLElement, audible = false) {
    const target = el?.isConnected && el.getClientRects().length ? el : elements()[0];
    const changed = !!target && document.activeElement !== target;
    target?.focus({ preventScroll: true });
    target?.scrollIntoView({ block: 'nearest', inline: 'nearest', behavior: 'instant' });
    if (audible && changed) sound('move');
  }
  function dispatch(command: NavigationCommand) {
    if (command === 'back') { back(); return; }
    if (command === 'home') { sound('press'); home(); return; }
    const current = document.activeElement as HTMLElement;
    if ((command === 'left' || command === 'right') && current instanceof HTMLSelectElement && !current.disabled) {
      const direction = command === 'right' ? 1 : -1;
      for (let index = current.selectedIndex + direction; index >= 0 && index < current.options.length; index += direction) {
        if (current.options[index].disabled) continue;
        current.selectedIndex = index;
        current.dispatchEvent(new Event('input', { bubbles: true }));
        current.dispatchEvent(new Event('change', { bubbles: true }));
        sound('move');
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
      sound('volume');
      return;
    }
    const candidates = elements();
    if (!candidates.includes(current)) { focus(candidates[0], true); return; }
    if (command === 'activate') {
      if (current instanceof HTMLInputElement && current.type === 'range') { sound('volume'); return; }
      if (!(current instanceof HTMLInputElement && (current.type === 'checkbox' || current.type === 'radio'))) sound('press');
      if (current instanceof HTMLInputElement && current.type === 'search' && current.form) {
        current.form.requestSubmit();
        return;
      }
      current.click();
      return;
    }
    const rail = current.closest?.('.sports-grid, .app-track');
    if (rail && (command === 'left' || command === 'right')) {
      const row = candidates.filter(element => rail.contains(element));
      const next = row[row.indexOf(current) + (command === 'right' ? 1 : -1)];
      if (next) focus(next, true);
      return;
    }
    const categories = current.closest?.('.settings-categories');
    const content = current.closest?.('.settings-content');
    const page = current.closest?.('.settings-page');
    if (categories && command === 'right') {
      const target = candidates.find(el => el.closest?.('.settings-content'));
      if (target) focus(target, true);
      return;
    }
    const sections = current.closest?.('.settings-sections');
    if (sections && command === 'down') {
      const currentRect = current.getBoundingClientRect();
      const bottom = currentRect.bottom ?? (currentRect.top ?? currentRect.y) + currentRect.height;
      const target = candidates.find(el => {
        if (!el.closest?.('.settings-content') || el.closest('.settings-sections')) return false;
        const rect = el.getBoundingClientRect();
        return (rect.top ?? rect.y) >= bottom;
      });
      if (target) focus(target, true);
      return;
    }
    if ((categories || content) && (command === 'up' || command === 'down')) {
      const group = candidates.filter(el => (categories ?? content)!.contains(el));
      const index = group.indexOf(current) + (command === 'down' ? 1 : -1);
      if (group[index]) focus(group[index], true);
      else if (command === 'up') focus(page?.querySelector<HTMLElement>(content ? '.settings-category.selected' : '.settings-page-header button') ?? current, true);
      return;
    }
    const a = current.getBoundingClientRect();
    const vertical = command === 'up' || command === 'down';
    const sign = command === 'down' || command === 'right' ? 1 : -1;
    let best: HTMLElement | undefined;
    let score = Infinity;
    for (const el of candidates) {
      if (el === current) continue;
      if (content && !content.contains(el)) continue;
      const b = el.getBoundingClientRect();
      if (content && !vertical && (b.y + b.height <= a.y || b.y >= a.y + a.height)) continue;
      const dx = b.x + b.width / 2 - a.x - a.width / 2;
      const dy = b.y + b.height / 2 - a.y - a.height / 2;
      const primary = vertical ? dy : dx;
      const secondary = vertical ? dx : dy;
      const distance = Math.abs(primary) + Math.abs(secondary) * 3;
      if (primary * sign > 4 && distance < score) { best = el; score = distance; }
    }
    if (best) focus(best, true);
    else if (content && command === 'left') focus(page?.querySelector<HTMLElement>('.settings-category.selected') ?? current, true);
  }
  function keydown(event: KeyboardEvent) {
    if (event.defaultPrevented || event.isComposing) return;
    if (homeShortcut(event) === shortcut()) { event.preventDefault(); if (!event.repeat) dispatch('home'); return; }
    if (event.key === 'Tab') {
      const candidates = elements();
      const index = candidates.indexOf(document.activeElement as HTMLElement);
      if (event.shiftKey && index <= 0) { event.preventDefault(); focus(candidates.at(-1), true); }
      else if (!event.shiftKey && index === candidates.length - 1) { event.preventDefault(); focus(candidates[0], true); }
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
