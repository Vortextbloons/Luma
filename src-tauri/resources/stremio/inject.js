(() => {
  'use strict';

  if (window.top !== window) return;
  if (!/(^|\.)stremio\.com$/.test(location.hostname) && !/(^|\.)strem\.io$/.test(location.hostname)) return;

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
