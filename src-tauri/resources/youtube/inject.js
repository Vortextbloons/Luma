(() => {
  'use strict';

  if (window.top !== window) return;
  if (location.hostname !== 'www.youtube.com' || !/^\/tv\/?$/.test(location.pathname)) return;

  const BRIDGE = window.__LUMA_BRIDGE || 'http://127.0.0.1:9876/integrations';
  const UA = window.__LUMA_UA || navigator.userAgent;
  const categories = ['sponsor', 'selfpromo', 'interaction', 'intro', 'outro', 'preview', 'hook', 'filler'];
  const segmentColors = {
    sponsor: '#00d400',
    selfpromo: '#ffff00',
    interaction: '#cc00ff',
    intro: '#00ffff',
    outro: '#0202ed',
    preview: '#008fd6',
    hook: '#395699',
    filler: '#7300ff',
  };

  try {
    Object.defineProperty(Navigator.prototype, 'userAgent', { configurable: true, get: () => UA });
    Object.defineProperty(Navigator.prototype, 'appVersion', { configurable: true, get: () => UA.replace(/^Mozilla\//, '') });
    const data = {
      brands: [{ brand: 'Luma', version: '27' }],
      mobile: false,
      platform: 'Starboard',
      getHighEntropyValues: async () => ({
        brands: [{ brand: 'Luma', version: '27' }],
        fullVersionList: [{ brand: 'Luma', version: '27.0.0' }],
        mobile: false,
        platform: 'Starboard',
        platformVersion: '',
        architecture: 'x86',
        bitness: '64',
        model: '',
        uaFullVersion: '27.0.0',
        wow64: false,
      }),
      toJSON: () => ({ brands: [{ brand: 'Luma', version: '27' }], mobile: false, platform: 'Starboard' }),
    };
    Object.defineProperty(Navigator.prototype, 'userAgentData', { configurable: true, get: () => data });
  } catch {
    // navigator overrides are best effort
  }

  function stripVideoAds(value, depth = 0) {
    if (!value || typeof value !== 'object' || depth > 4) return;
    if (Array.isArray(value.adPlacements)) value.adPlacements = [];
    if (Array.isArray(value.adSlots)) value.adSlots = [];
    if (Array.isArray(value.entries)) {
      value.entries = value.entries.filter(entry => !entry?.command?.reelWatchEndpoint?.adClientParams?.isAd);
    }
    if (value.playerResponse && typeof value.playerResponse === 'object') stripVideoAds(value.playerResponse, depth + 1);
    if (value.response && typeof value.response === 'object') stripVideoAds(value.response, depth + 1);
  }

  const nativeParse = JSON.parse.bind(JSON);
  JSON.parse = function (text, reviver) {
    const value = nativeParse(text, reviver);
    if (value && typeof value === 'object') {
      try { stripVideoAds(value); } catch { /* never break parsing */ }
    }
    return value;
  };

  const responseModifiers = [];
  const addResponseModifier = modifier => responseModifiers.push(modifier);

  const OriginalXHR = window.XMLHttpRequest;
  const nativeResponseText = Object.getOwnPropertyDescriptor(OriginalXHR.prototype, 'responseText').get;
  const nativeResponse = Object.getOwnPropertyDescriptor(OriginalXHR.prototype, 'response').get;
  const responseEvents = ['readystatechange', 'load', 'loadend'];

  function LumaXHR() {
    const xhr = new OriginalXHR();
    const originalOpen = xhr.open;
    const originalSend = xhr.send;
    const originalAddEventListener = xhr.addEventListener;
    const originalRemoveEventListener = xhr.removeEventListener;

    let url = null;
    let modification = null;
    let modifiedText = null;
    let request = 0;

    function modifyResponse() {
      if (!modification) {
        const current = request;
        modification = (async () => {
          if (xhr.responseType !== '' && xhr.responseType !== 'text') return;
          if (xhr.status === 0) return;
          let text = nativeResponseText.call(xhr);
          for (const modifier of responseModifiers) {
            try {
              const modified = await modifier(url, text);
              if (modified !== undefined) text = modified;
            } catch (error) {
              console.error('[Luma] response modifier failed', error);
            }
          }
          if (current === request) modifiedText = text;
        })();
      }
      return modification;
    }

    function createWrapper(listener) {
      return async function (event) {
        if (xhr.readyState === 4) {
          const current = request;
          await modifyResponse();
          if (current !== request) return;
        }
        if (typeof listener === 'function') listener.call(xhr, event);
        else listener.handleEvent(event);
      };
    }

    const wrappers = Object.fromEntries(responseEvents.map(type => [type, new WeakMap()]));
    function wrap(type, listener) {
      if (!wrappers[type].has(listener)) wrappers[type].set(listener, createWrapper(listener));
      return wrappers[type].get(listener);
    }

    xhr.addEventListener = function (type, listener, options) {
      if (responseEvents.includes(type) && listener) {
        return originalAddEventListener.call(this, type, wrap(type, listener), options);
      }
      return originalAddEventListener.apply(this, arguments);
    };

    xhr.removeEventListener = function (type, listener, options) {
      if (responseEvents.includes(type) && listener) {
        return originalRemoveEventListener.call(this, type, wrappers[type].get(listener) ?? listener, options);
      }
      return originalRemoveEventListener.apply(this, arguments);
    };

    for (const type of responseEvents) {
      let handler = null;
      let wrapper = null;
      Object.defineProperty(xhr, `on${type}`, {
        configurable: true,
        get() { return handler; },
        set(value) {
          if (wrapper) originalRemoveEventListener.call(xhr, type, wrapper);
          handler = typeof value === 'function' ? value : null;
          wrapper = handler ? createWrapper(handler) : null;
          if (wrapper) originalAddEventListener.call(xhr, type, wrapper);
        },
      });
    }

    for (const [property, nativeGetter] of [['responseText', nativeResponseText], ['response', nativeResponse]]) {
      Object.defineProperty(xhr, property, {
        configurable: true,
        get() { return modifiedText ?? nativeGetter.call(xhr); },
      });
    }

    xhr.open = function (method, requestUrl) {
      url = String(requestUrl);
      modification = null;
      modifiedText = null;
      request++;
      return originalOpen.apply(this, arguments);
    };

    xhr.send = function () {
      return originalSend.apply(this, arguments);
    };

    return xhr;
  }

  LumaXHR.prototype = OriginalXHR.prototype;
  for (const key of ['UNSENT', 'OPENED', 'HEADERS_RECEIVED', 'LOADING', 'DONE']) LumaXHR[key] = OriginalXHR[key];
  window.XMLHttpRequest = LumaXHR;

  addResponseModifier((url, text) => {
    if (!url.startsWith('/youtubei/v1/browse') && !url.startsWith('/youtubei/v1/search')) return;
    const json = JSON.parse(text);
    if (url.startsWith('/youtubei/v1/browse')) {
      const feed = json.contents?.tvBrowseRenderer?.content?.tvSurfaceContentRenderer?.content?.sectionListRenderer;
      if (!feed?.contents) return;
      feed.contents = feed.contents.filter(row => !row.adSlotRenderer && !row.promoShelfRenderer && !row.shelfRenderer?.tvhtml5Metadata?.hideLogo);
      for (const row of feed.contents) {
        const items = row?.shelfRenderer?.content?.horizontalListRenderer?.items;
        if (items) row.shelfRenderer.content.horizontalListRenderer.items = items.filter(item => !item.adSlotRenderer);
      }
    } else {
      const feed = json.contents?.sectionListRenderer;
      if (!feed?.contents) return;
      for (const row of feed.contents) {
        const items = row?.shelfRenderer?.content?.horizontalListRenderer?.items;
        if (items) row.shelfRenderer.content.horizontalListRenderer.items = items.filter(item => !item.adSlotRenderer);
      }
    }
    return JSON.stringify(json);
  });

  const brandingCache = new Map();

  async function bridgeJSON(path, params) {
    const query = new URLSearchParams(params);
    const response = await fetch(`${BRIDGE}/${path}?${query}`, { cache: 'no-store' });
    if (!response.ok) return null;
    return response.json();
  }

  async function bridgeImage(videoId) {
    const response = await fetch(`${BRIDGE}/dearrow-thumbnail?videoID=${encodeURIComponent(videoId)}`, { cache: 'force-cache' });
    if (!response.ok) return null;
    const blob = await response.blob();
    if (blob.size === 0) return null;
    return new Promise(resolve => {
      const reader = new FileReader();
      reader.onload = () => resolve(String(reader.result));
      reader.onerror = () => resolve(null);
      reader.readAsDataURL(blob);
    });
  }

  async function branding(videoId) {
    if (brandingCache.has(videoId)) return brandingCache.get(videoId);
    const value = await bridgeJSON('dearrow', { videoID: videoId });
    brandingCache.set(videoId, value);
    return value;
  }

  function collectTiles(url, json) {
    const items = [];
    if (json.continuationContents?.horizontalListContinuation?.items) {
      items.push(...json.continuationContents.horizontalListContinuation.items);
    } else if (json.continuationContents?.gridContinuation?.items) {
      items.push(...json.continuationContents.gridContinuation.items);
    } else {
      let contents = [];
      if (url.startsWith('/youtubei/v1/browse')) {
        if (json.contents?.tvBrowseRenderer?.content?.tvSecondaryNavRenderer?.sections) {
          for (const section of json.contents.tvBrowseRenderer.content.tvSecondaryNavRenderer.sections) {
            const tab = section.tvSecondaryNavSectionRenderer?.tabs?.[0];
            contents = tab?.tabRenderer?.content?.tvSurfaceContentRenderer?.content?.sectionListRenderer?.contents ?? contents;
          }
        } else if (json.contents?.tvBrowseRenderer?.content?.tvSurfaceContentRenderer?.content?.sectionListRenderer?.contents) {
          contents = json.contents.tvBrowseRenderer.content.tvSurfaceContentRenderer.content.sectionListRenderer.contents;
        } else if (json.contents?.tvBrowseRenderer?.content?.tvSurfaceContentRenderer?.content?.gridRenderer) {
          contents = [json.contents.tvBrowseRenderer.content.tvSurfaceContentRenderer.content];
        } else if (json.continuationContents?.tvSurfaceContentContinuation?.content?.sectionListRenderer?.contents) {
          contents = json.continuationContents.tvSurfaceContentContinuation.content.sectionListRenderer.contents;
        }
      } else if (url.startsWith('/youtubei/v1/search')) {
        contents = json.contents?.sectionListRenderer?.contents ?? [];
      } else if (url.startsWith('/youtubei/v1/next')) {
        contents = json.contents?.singleColumnWatchNextResults?.pivot?.sectionListRenderer?.contents ?? [];
      }
      for (const content of contents) {
        if (content.shelfRenderer) items.push(...(content.shelfRenderer.content?.horizontalListRenderer?.items ?? []));
        else if (content.gridRenderer) items.push(...(content.gridRenderer.items ?? []));
      }
    }
    return items;
  }

  addResponseModifier(async (url, text) => {
    if (!url.startsWith('/youtubei/v1/browse') && !url.startsWith('/youtubei/v1/search') && !url.startsWith('/youtubei/v1/next')) return;
    const json = JSON.parse(text);
    const items = collectTiles(url, json).filter(item => item?.tileRenderer?.contentType === 'TILE_CONTENT_TYPE_VIDEO');
    if (items.length === 0) return;
    await Promise.all(items.map(async item => {
      try {
        const id = item.tileRenderer.contentId;
        if (!id || !item.tileRenderer.metadata) return;
        const value = await branding(id);
        if (!value?.videoDuration) return;
        const title = value.titles?.find(entry => entry.locked || entry.votes >= 0);
        if (title?.title) {
          item.tileRenderer.metadata.tileMetadataRenderer.title.simpleText = title.title
            .split(' ')
            .map(word => (word.startsWith('>') ? word.slice(1) : word))
            .join(' ');
        }
        const image = await bridgeImage(id);
        if (image) item.tileRenderer.header.tileHeaderRenderer.thumbnail.thumbnails[0].url = image;
      } catch (error) {
        console.error('[Luma] DeArrow failed', error);
      }
    }));
    return JSON.stringify(json);
  });

  let sheet = null;
  function injectCSS(text) {
    try {
      if (!sheet) {
        sheet = new CSSStyleSheet();
        document.adoptedStyleSheets = [...document.adoptedStyleSheets, sheet];
      }
      sheet.replaceSync(text);
      return;
    } catch {
      let style = document.getElementById('luma-style');
      if (!style) {
        style = document.createElement('style');
        style.id = 'luma-style';
        (document.head ?? document.documentElement).appendChild(style);
      }
      style.textContent = text;
    }
  }

  function layers(segments, from, to) {
    const output = [];
    for (const segment of segments) {
      if (segment.endTime <= from || segment.startTime >= to) continue;
      const color = segmentColors[segment.category] || '#ffffff';
      const start = Math.max(0, ((segment.startTime - from) / (to - from)) * 100);
      const end = Math.min(100, ((segment.endTime - from) / (to - from)) * 100);
      output.push(`linear-gradient(to right, transparent ${start}%, ${color} ${start}%, ${color} ${end}%, transparent ${end}%)`);
    }
    return output.join(', ') || 'none';
  }

  function chapters(duration) {
    const bars = document.querySelectorAll('ytlr-multi-markers-player-bar-renderer [idomkey="progress-bar"] > [idomkey^="chapter-"]');
    if (bars.length === 0 || !(duration > 0)) return null;
    let total = 0;
    for (const bar of bars) total += parseFloat(bar.style.width) || 0;
    if (!(total > 0)) return null;
    const result = [];
    let start = 0;
    for (const bar of bars) {
      const width = parseFloat(bar.style.width) || 0;
      result.push({ key: bar.getAttribute('idomkey'), from: (start / total) * duration, to: ((start + width) / total) * duration });
      start += width;
    }
    return result;
  }

  let segments = [];
  let segmentChapters = null;
  let activeVideoId = null;
  let activeVideo = null;
  let attachTimer = null;

  const duration = () => {
    const player = document.querySelector('.html5-video-player');
    if (typeof player?.getDuration === 'function') return player.getDuration();
    return activeVideo?.duration || 0;
  };

  function drawSegments() {
    const length = duration();
    if (segments.length === 0 || !(length > 0)) {
      injectCSS('');
      return;
    }
    const ordered = [...segments].sort((a, b) => a.endTime - a.startTime - (b.endTime - b.startTime));
    let css = `ytlr-progress-bar [idomkey="slider"]::after { background: ${layers(ordered, 0, length)}; }`;
    if (segmentChapters) {
      for (const chapter of segmentChapters) {
        css += `ytlr-multi-markers-player-bar-renderer [idomkey="${chapter.key}"]::after { background: ${layers(ordered, chapter.from, chapter.to)}; }`;
      }
    }
    injectCSS(css);
  }

  function detachVideo() {
    if (attachTimer) clearTimeout(attachTimer);
    attachTimer = null;
    if (activeVideo) {
      activeVideo.removeEventListener('timeupdate', checkForSkip);
      activeVideo.removeEventListener('durationchange', drawSegments);
    }
    activeVideo = null;
  }

  function attachVideo() {
    detachVideo();
    const video = document.querySelector('video');
    if (!video) {
      attachTimer = setTimeout(attachVideo, 250);
      return;
    }
    activeVideo = video;
    activeVideo.addEventListener('timeupdate', checkForSkip);
    activeVideo.addEventListener('durationchange', drawSegments);
    drawSegments();
  }

  function checkForSkip() {
    if (!activeVideo || activeVideo.paused || segments.length === 0) return;
    const now = activeVideo.currentTime;
    const segment = segments
      .filter(entry => now > entry.startTime && now < entry.startTime + 2 && now < entry.endTime)
      .sort((a, b) => a.startTime - b.startTime)[0];
    if (segment) activeVideo.currentTime = segment.endTime;
  }

  function currentVideoId() {
    const page = new URL(location.hash.substring(1), location.href);
    return page.pathname === '/watch' ? page.searchParams.get('v') : null;
  }

  function onNavigate() {
    const id = currentVideoId();
    if (id === activeVideoId) return;
    activeVideoId = id;
    segments = [];
    segmentChapters = null;
    detachVideo();
    drawSegments();
    if (!id) return;
    bridgeJSON('sponsorblock', { videoID: id })
      .then(value => {
        if (activeVideoId !== id) return;
        segments = Array.isArray(value) ? value.filter(entry => categories.includes(entry.category)) : [];
        attachVideo();
        drawSegments();
      })
      .catch(error => console.error('[Luma] SponsorBlock failed', error));
  }

  setInterval(() => {
    if (!activeVideoId || segments.length === 0) return;
    const found = chapters(duration());
    if (!found || JSON.stringify(found) === JSON.stringify(segmentChapters)) return;
    segmentChapters = found;
    drawSegments();
  }, 1500);

  window.addEventListener('hashchange', onNavigate);
  onNavigate();

  function keyEvent(type, code, key) {
    const event = new Event(type, { bubbles: true, cancelable: true });
    Object.defineProperties(event, {
      keyCode: { value: code },
      which: { value: code },
      key: { value: key },
    });
    document.dispatchEvent(event);
  }

  const keyCodes = { up: [38, 'ArrowUp'], down: [40, 'ArrowDown'], left: [37, 'ArrowLeft'], right: [39, 'ArrowRight'], activate: [13, 'Enter'], back: [27, 'Escape'] };

  window.__lumaNavigate = command => {
    const mapping = keyCodes[command];
    if (!mapping) return;
    keyEvent('keydown', mapping[0], mapping[1]);
    keyEvent('keyup', mapping[0], mapping[1]);
  };

  function exit() {
    fetch(`${BRIDGE}/youtube-exit`, { cache: 'no-store' }).catch(() => {});
  }

  window.addEventListener('keydown', event => {
    if (event.ctrlKey && !event.shiftKey && !event.altKey && event.key.toLowerCase() === 'w') {
      event.preventDefault();
      event.stopImmediatePropagation();
      exit();
    }
  }, true);

  const buttonOffset = 32768;
  const axisThreshold = 0.5;
  const axisButtons = [[19, 20], [17, 18], [23, 24], [21, 22]];
  const pressed = {};
  let focused = document.hasFocus();
  window.addEventListener('focus', () => { focused = true; });
  window.addEventListener('blur', () => { focused = false; });
  const timers = new Map();

  function press(code) {
    if (focused) keyEvent('keydown', code, '');
    clearInterval(timers.get(code));
    const delay = setTimeout(() => {
      timers.set(code, setInterval(() => { if (focused) keyEvent('keydown', code, ''); }, 100));
    }, 500);
    timers.set(`delay:${code}`, delay);
  }

  function release(code) {
    if (focused) keyEvent('keyup', code, '');
    clearTimeout(timers.get(`delay:${code}`));
    clearInterval(timers.get(code));
    timers.delete(`delay:${code}`);
    timers.delete(code);
  }

  function pollGamepads() {
    const gamepads = navigator.getGamepads ? navigator.getGamepads() : [];
    for (const index in pressed) {
      if (!gamepads[index] && pressed[index]) {
        for (const code of pressed[index].buttons) release(code);
        for (const code of Object.values(pressed[index].axes)) if (code) release(code);
        pressed[index] = null;
      }
    }
    for (const gamepad of gamepads) {
      if (!gamepad?.connected) continue;
      const state = pressed[gamepad.index] ?? (pressed[gamepad.index] = { buttons: new Set(), axes: {} });
      gamepad.buttons.forEach((button, index) => {
        const code = buttonOffset + index;
        if (button.pressed && !state.buttons.has(code)) {
          state.buttons.add(code);
          press(code);
        } else if (!button.pressed && state.buttons.has(code)) {
          state.buttons.delete(code);
          release(code);
        }
      });
      axisButtons.forEach(([negative, positive], index) => {
        const value = gamepad.axes[index] ?? 0;
        const code = value < -axisThreshold ? buttonOffset + negative : value > axisThreshold ? buttonOffset + positive : null;
        if (state.axes[index] === code) return;
        if (state.axes[index]) release(state.axes[index]);
        state.axes[index] = code;
        if (code) press(code);
      });
    }
    requestAnimationFrame(pollGamepads);
  }

  requestAnimationFrame(pollGamepads);
})();
