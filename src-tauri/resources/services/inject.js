(() => {
  'use strict';

  if (window.top !== window) return;
  if (window.__lumaService) return;
  window.__lumaService = true;

  const STYLE_ID = 'luma-service-focus';
  const OUTLINE = '3px solid #a59bff';
  const CANDIDATES = 'a[href],area[href],button,input:not([type="hidden"]),select,textarea,summary,[role="button"],[role="link"],[role="tab"],[role="option"],[role="menuitem"],[role="checkbox"],[role="switch"],[contenteditable="true"],[tabindex]';
  const INTERACTIVE = /^(A|BUTTON|INPUT|SELECT|TEXTAREA|SUMMARY)$/;
  const savedOutlines = new WeakMap();

  function choose(current, candidates, command) {
    if (!candidates.length) return null;
    if (!current) return candidates[0];
    const vertical = command === 'up' || command === 'down';
    const sign = command === 'down' || command === 'right' ? 1 : -1;
    const origin = current.rect;
    let best = null;
    let score = Infinity;
    for (const candidate of candidates) {
      if (candidate === current) continue;
      const box = candidate.rect;
      const dx = box.left + box.width / 2 - origin.left - origin.width / 2;
      const dy = box.top + box.height / 2 - origin.top - origin.height / 2;
      const primary = vertical ? dy : dx;
      const secondary = vertical ? dx : dy;
      if (primary * sign <= 4) continue;
      const distance = Math.abs(primary) + Math.abs(secondary) * 3;
      if (distance < score) {
        best = candidate;
        score = distance;
      }
    }
    return best;
  }

  function backTarget(index, length) {
    if (typeof index === 'number') return index <= 0 ? 'home' : 'back';
    return length <= 1 ? 'home' : 'back';
  }

  window.__lumaSpatial = { choose, backTarget };

  function installStyle() {
    if (!document.documentElement || document.getElementById(STYLE_ID)) return;
    const style = document.createElement('style');
    style.id = STYLE_ID;
    style.textContent = `:focus{outline:${OUTLINE} !important;outline-offset:3px !important}`;
    (document.head || document.documentElement).appendChild(style);
  }

  function measure(element) {
    if (element.disabled) return null;
    if (element.closest && element.closest('[aria-hidden="true"],[inert],#luma-service-hint')) return null;
    if (element.checkVisibility && !element.checkVisibility({ checkOpacity: true, checkVisibilityCSS: true })) return null;
    const box = element.getBoundingClientRect();
    if (box.width < 8 || box.height < 8) return null;
    if (box.width * box.height > window.innerWidth * window.innerHeight * 0.45) return null;
    return { left: box.left, top: box.top, width: box.width, height: box.height };
  }

  function collect() {
    const items = [];
    for (const element of document.querySelectorAll(CANDIDATES)) {
      if (items.length >= 1500) break;
      const box = measure(element);
      if (box) items.push({ element, rect: box });
    }
    return items;
  }

  function clearOutline(element) {
    const saved = savedOutlines.get(element);
    if (!saved) return;
    element.style.outline = saved[0];
    element.style.outlineOffset = saved[1];
    savedOutlines.delete(element);
  }

  function showFocus(element) {
    if (!element || element.tagName === undefined) return false;
    if (!savedOutlines.has(element)) {
      savedOutlines.set(element, [element.style.outline, element.style.outlineOffset]);
    }
    element.style.setProperty('outline', OUTLINE, 'important');
    element.style.setProperty('outline-offset', '3px', 'important');
    if (!INTERACTIVE.test(element.tagName) && !element.hasAttribute('tabindex')) {
      element.setAttribute('tabindex', '-1');
    }
    element.focus({ preventScroll: true });
    if (document.activeElement !== element) {
      clearOutline(element);
      return false;
    }
    element.scrollIntoView({ block: 'nearest', inline: 'nearest' });
    return true;
  }

  function editable(element) {
    if (!element) return false;
    if (element.isContentEditable) return true;
    if (/^(INPUT|TEXTAREA|SELECT)$/.test(element.tagName)) return true;
    return element.closest?.('[contenteditable="true"]') !== null;
  }

  function playing() {
    if (document.fullscreenElement || document.pictureInPictureElement) return true;
    const active = document.activeElement;
    if (active?.tagName === 'VIDEO') return true;
    return [...document.querySelectorAll('video')].some(video => !video.paused && !video.muted);
  }

  function move(command) {
    if (playing()) return false;
    const items = collect();
    if (!items.length) return false;
    const active = document.activeElement;
    let current = items.find(item => item.element === active) || null;
    if (!current && active && active !== document.body) {
      const box = active.getBoundingClientRect();
      if (box.width > 0 && box.height > 0) {
        current = { element: active, rect: { left: box.left, top: box.top, width: box.width, height: box.height } };
      }
    }
    const target = choose(current, items, command);
    if (!target) return false;
    if (current) clearOutline(current.element);
    return showFocus(target.element);
  }

  function activate() {
    const active = document.activeElement;
    if (!active || active === document.body) return false;
    if (editable(active)) return true;
    if (!measure(active)) return false;
    active.click?.();
    return true;
  }

  function backState() {
    const index = window.navigation?.currentEntry?.index;
    return { index: typeof index === 'number' ? index : null, length: history.length };
  }

  window.__lumaNavigate = command => {
    if (command === 'activate') return activate();
    if (command === 'up' || command === 'down' || command === 'left' || command === 'right') return move(command);
    return false;
  };

  window.__lumaBackState = backState;

  window.__lumaBack = () => {
    const state = backState();
    const target = backTarget(state.index, state.length);
    if (target === 'back') history.back();
    return target;
  };

  window.addEventListener('keydown', event => {
    if (event.defaultPrevented || event.ctrlKey || event.metaKey || event.altKey || editable(event.target)) return;
    const commands = { ArrowUp: 'up', ArrowDown: 'down', ArrowLeft: 'left', ArrowRight: 'right' };
    const command = commands[event.key];
    if (command) {
      if (window.__lumaNavigate?.(command)) {
        event.preventDefault();
        event.stopImmediatePropagation();
      }
      return;
    }
    if (event.key === 'Enter' && activate()) {
      event.preventDefault();
      event.stopImmediatePropagation();
    }
  }, true);

  document.addEventListener('focusout', event => {
    if (event.target) clearOutline(event.target);
  }, true);

  installStyle();
  if (document.readyState === 'loading') document.addEventListener('DOMContentLoaded', installStyle, { once: true });
})();
