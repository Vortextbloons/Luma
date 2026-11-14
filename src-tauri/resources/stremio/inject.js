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

  window.__lumaNavigate = command => {
    if (command === 'back') {
      back();
      return;
    }
    if (command === 'activate') {
      const active = document.activeElement;
      if (active instanceof HTMLElement) active.click();
      return;
    }
    const key = keys[command];
    if (key) press(key[0], key[1]);
  };

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
