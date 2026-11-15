(() => {
  'use strict';

  if (window.top !== window) return;
  if (!/(^|\.)stremio\.com$/.test(location.hostname) && !/(^|\.)strem\.io$/.test(location.hostname)) return;

  const session = __LUMA_STREMIO_SESSION__;
  if (location.origin === 'https://web.stremio.com') {
    const marker = 'luma-stremio-account';
    let authenticating = false;
    let timeout;
    function feedback(message) {
      let banner = document.getElementById('luma-account-status');
      if (!banner) {
        banner = document.createElement('div');
        banner.id = 'luma-account-status';
        banner.setAttribute('role', 'status');
        banner.style.cssText = 'position:fixed;top:16px;left:16px;right:16px;z-index:2147483647;padding:16px;background:#181b23;color:white;border:1px solid #a59bff;border-radius:12px;font:16px "Segoe UI",sans-serif';
        document.documentElement.append(banner);
      }
      banner.textContent = message;
    }
    function failed() {
      authenticating = false;
      clearTimeout(timeout);
      feedback('Stremio could not load your account. Return to Luma and reconnect Stremio in Settings, then try again.');
    }
    let coreEvents;
    Object.defineProperty(window, 'onCoreEvent', {
      configurable: true,
      get: () => coreEvents,
      set(listener) {
        coreEvents = event => {
          listener?.(event);
          if (!authenticating || event.name !== 'CoreEvent') return;
          if (event.args.event === 'UserAuthenticated') {
            authenticating = false;
            clearTimeout(timeout);
            localStorage.setItem(marker, session.account.userId);
            document.getElementById('luma-account-status')?.remove();
            setTimeout(() => { location.hash = session.target || '#/'; }, 0);
          } else if (event.args.event === 'Error' && event.args.args.source.event === 'UserAuthenticated') {
            failed();
          }
        };
      },
    });
    window.__lumaSyncAccount = async (target = session.target) => {
      session.target = target;
      if (!window.core || authenticating) return;
      try {
        const { profile } = await window.core.getState('ctx');
        if (!session.account) {
          if (localStorage.getItem(marker)) {
            await window.core.dispatch({ action: 'Ctx', args: { action: 'Logout' } });
            localStorage.removeItem(marker);
            location.hash = '#/intro';
          }
          return;
        }
        if (profile.auth?.user?._id === session.account.userId) {
          localStorage.setItem(marker, session.account.userId);
          return;
        }
        authenticating = true;
        feedback('Loading your Stremio account and add-ons...');
        timeout = setTimeout(failed, 30000);
        await window.core.dispatch({ action: 'Ctx', args: { action: 'Logout' } });
        await window.core.dispatch({ action: 'Ctx', args: { action: 'Authenticate', args: { type: 'LoginWithToken', token: session.account.token } } });
      } catch {
        failed();
      }
    };
    Object.defineProperty(window, 'core', {
      configurable: true,
      set(core) {
        Object.defineProperty(window, 'core', { configurable: true, writable: true, value: core });
        if (core) queueMicrotask(() => window.__lumaSyncAccount());
      },
    });
  }

  function hideWebView() {
    const chrome = window.chrome;
    if (!chrome || !chrome.webview) return;
    try { delete chrome.webview; } catch {}
    if (!chrome.webview) return;
    try { Object.defineProperty(chrome, 'webview', { configurable: true, value: undefined }); } catch {}
    if (!chrome.webview) return;
    const safe = new Proxy(chrome, {
      get: (target, property) => property === 'webview' ? undefined : Reflect.get(target, property),
    });
    try { Object.defineProperty(window, 'chrome', { configurable: true, value: safe }); } catch {}
  }

  hideWebView();
  queueMicrotask(hideWebView);
  document.addEventListener('DOMContentLoaded', hideWebView);
  window.addEventListener('load', hideWebView);

  function close() {
    location.href = 'https://web.stremio.com/__luma_close__';
  }

  function autoHideReturnButton(host, root) {
    let timer;
    function update() {
      root.activeElement?.blur();
      host.style.display = 'none';
      host.inert = true;
    }
    function reveal() {
      clearTimeout(timer);
      host.style.display = '';
      host.inert = false;
      timer = setTimeout(update, 2000);
    }
    window.__lumaRevealReturn = reveal;
    document.addEventListener('pointermove', event => {
      if (event.movementX || event.movementY) reveal();
    }, true);
    document.addEventListener('keydown', reveal, true);
    document.addEventListener('pointerdown', reveal, true);
    document.addEventListener('wheel', reveal, { capture: true, passive: true });
    root.addEventListener('focusin', reveal);
    window.addEventListener('pagehide', () => clearTimeout(timer), { once: true });
    reveal();
  }

  function addReturnButton() {
    if (document.getElementById('luma-return-button')) return;
    const host = document.createElement('div');
    host.id = 'luma-return-button';
    host.style.cssText = 'position:fixed;left:16px;bottom:16px;z-index:2147483647;';
    const root = host.attachShadow({ mode: 'open' });
    const style = document.createElement('style');
    style.textContent = 'button{all:initial;display:inline-flex;align-items:center;gap:7px;padding:11px 16px;border:1px solid rgba(255,255,255,.22);border-radius:999px;background:rgba(12,14,20,.84);color:#fff;font:600 14px/1 "Segoe UI",system-ui,sans-serif;cursor:pointer;box-shadow:0 8px 24px rgba(0,0,0,.35);backdrop-filter:blur(10px)}button:hover{background:rgba(35,39,52,.94)}button:focus-visible{outline:2px solid #a59bff;outline-offset:3px}';
    const button = document.createElement('button');
    button.type = 'button';
    button.textContent = '‹ Back to Luma';
    button.addEventListener('click', close);
    root.append(style, button);
    (document.documentElement ?? document.body).append(host);
    autoHideReturnButton(host, root);
  }

  if (document.readyState === 'loading') document.addEventListener('DOMContentLoaded', addReturnButton, { once: true });
  else addReturnButton();

  function atHome() {
    return !location.hash || location.hash === '#/' || location.hash === '#/intro';
  }

  function back() {
    if (atHome() || history.length <= 1) {
      close();
      return;
    }
    history.back();
  }

  const keys = {
    up: ['ArrowUp', 38],
    down: ['ArrowDown', 40],
    left: ['ArrowLeft', 37],
    right: ['ArrowRight', 39],
  };

  function press(name, code) {
    const event = new Event('keydown', { bubbles: true, cancelable: true });
    Object.defineProperties(event, {
      key: { value: name },
      code: { value: name },
      keyCode: { value: code },
      which: { value: code },
    });
    document.dispatchEvent(event);
  }

  const candidates = 'a[href],button,input:not([type="hidden"]),select,textarea,[tabindex],[role="button"],[role="tab"],[role="option"],[role="menuitem"]';
  const remembered = new Map();
  let selected = null;
  let route = location.hash;

  function editable(element) {
    return element?.isContentEditable || /^(INPUT|TEXTAREA|SELECT)$/.test(element?.tagName);
  }

  function dialog() {
    return [...document.querySelectorAll('[role="dialog"],[aria-modal="true"],[class*="modal-container"]')]
      .reverse().find(element => element.getBoundingClientRect().width && getComputedStyle(element).visibility !== 'hidden');
  }

  function items(scope = document) {
    return [...scope.querySelectorAll(candidates)].filter(element => {
      if (element.disabled || element.getAttribute('aria-disabled') === 'true' || element.closest('[inert],[aria-hidden="true"]')) return false;
      if (element.checkVisibility && !element.checkVisibility({ checkOpacity: true, checkVisibilityCSS: true })) return false;
      const rect = element.getBoundingClientRect();
      return rect.width > 4 && rect.height > 4;
    });
  }

  function focus(element) {
    selected?.removeAttribute('data-luma-selected');
    selected = element;
    element.setAttribute('data-luma-selected', '');
    element.focus({ preventScroll: true });
    element.scrollIntoView({ block: 'nearest', inline: 'nearest', behavior: 'instant' });
    if (!dialog()) {
      remembered.delete(location.hash);
      remembered.set(location.hash, { href: element.getAttribute('href'), index: items().indexOf(element) });
      if (remembered.size > 40) remembered.delete(remembered.keys().next().value);
    }
  }

  function move(command) {
    const scope = dialog() || document;
    const list = items(scope);
    if (!list.length) return;
    const active = list.includes(document.activeElement) ? document.activeElement : null;
    if (!active) {
      const saved = scope === document && remembered.get(location.hash);
      focus((saved?.href && list.find(element => element.getAttribute('href') === saved.href)) || list[saved?.index] || list[0]);
      return;
    }
    const origin = active.getBoundingClientRect();
    const vertical = command === 'up' || command === 'down';
    const sign = command === 'down' || command === 'right' ? 1 : -1;
    let best;
    let score = Infinity;
    for (const element of list) {
      if (element === active) continue;
      const rect = element.getBoundingClientRect();
      const dx = rect.left + rect.width / 2 - origin.left - origin.width / 2;
      const dy = rect.top + rect.height / 2 - origin.top - origin.height / 2;
      const primary = (vertical ? dy : dx) * sign;
      if (primary <= 4) continue;
      const cross = Math.abs(vertical ? dx : dy);
      const overlap = vertical
        ? rect.right > origin.left && rect.left < origin.right
        : rect.bottom > origin.top && rect.top < origin.bottom;
      const distance = primary + cross * 3 + (overlap ? 0 : 10000);
      if (distance < score) { best = element; score = distance; }
    }
    if (best) focus(best);
  }

  function navigate(command) {
    const modal = dialog();
    const video = [...document.querySelectorAll('video')].find(element => element.getBoundingClientRect().width > 0);
    if (command === 'back') {
      if (modal) { press('Escape', 27); return; }
      if (document.fullscreenElement) { document.exitFullscreen().catch(() => {}); return; }
      back();
      return;
    }
    if (video && !modal && !editable(document.activeElement)) {
      if (command === 'left' || command === 'right') {
        if (Number.isFinite(video.duration)) video.currentTime = Math.max(0, Math.min(video.duration, video.currentTime + (command === 'right' ? 10 : -10)));
        return;
      }
      if (command === 'activate' && !items().includes(document.activeElement)) {
        if (video.paused) video.play().catch(() => {});
        else video.pause();
        return;
      }
    }
    if (editable(document.activeElement) && command !== 'activate') {
      if (command === 'left' || command === 'right') { const key = keys[command]; press(key[0], key[1]); return; }
      document.activeElement.blur();
    }
    if (command === 'activate') {
      const list = items(modal || document);
      if (!list.includes(document.activeElement)) { if (list.length) focus(list[0]); }
      else document.activeElement.click();
      return;
    }
    if (keys[command]) move(command);
  }

  window.__lumaNavigate = navigate;

  window.addEventListener('hashchange', () => {
    route = location.hash;
    selected?.removeAttribute('data-luma-selected');
    selected = null;
    const saved = remembered.get(route);
    if (!saved) return;
    requestAnimationFrame(() => {
      if (route !== location.hash || dialog()) return;
      const list = items();
      const target = (saved.href && list.find(element => element.getAttribute('href') === saved.href)) || list[saved.index];
      if (target) focus(target);
    });
  });

  function installFocusStyle() {
    const style = document.createElement('style');
    style.textContent = '[data-luma-selected]{outline:3px solid #a59bff !important;outline-offset:3px !important}';
    document.documentElement.append(style);
  }
  if (document.readyState === 'loading') document.addEventListener('DOMContentLoaded', installFocusStyle, { once: true });
  else installFocusStyle();

  window.addEventListener('keydown', event => {
    if (!event.isTrusted || event.defaultPrevented || event.ctrlKey || event.metaKey || event.altKey || editable(event.target)) return;
    const command = { ArrowUp: 'up', ArrowDown: 'down', ArrowLeft: 'left', ArrowRight: 'right', Enter: 'activate', Escape: 'back', Backspace: 'back' }[event.key];
    if (!command) return;
    event.preventDefault();
    event.stopImmediatePropagation();
    navigate(command);
  }, true);

  window.addEventListener('keydown', event => {
    if (event.ctrlKey && !event.shiftKey && !event.altKey && event.key.toLowerCase() === 'w') {
      event.preventDefault();
      event.stopImmediatePropagation();
      close();
      return;
    }
    if (event.defaultPrevented || event.ctrlKey || event.metaKey || event.altKey) return;
    const target = event.target;
    if (target instanceof HTMLInputElement || target instanceof HTMLTextAreaElement || target instanceof HTMLSelectElement || target?.isContentEditable) return;
    if (document.fullscreenElement) return;
    if (event.key === 'Escape') {
      if (!atHome() || document.querySelector('[class*="modal-container"]')) return;
      event.preventDefault();
      event.stopImmediatePropagation();
      close();
      return;
    }
    if (event.key === 'Backspace') {
      event.preventDefault();
      event.stopImmediatePropagation();
      back();
    }
  }, true);
})();
