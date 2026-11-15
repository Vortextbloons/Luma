(() => {
  if (window.__lumaSportsReady || location.protocol !== 'https:') return;
  window.__lumaSportsReady = true;
  const main = window.top === window;
  if (main && !['isportsurge.ws', 'golygoals.com', 'streamec.st'].includes(location.hostname)) return;
  const direct = main && location.hostname !== 'isportsurge.ws';
  const plans = window.__lumaSportsUrls || [];
  const planIndex = Math.min(Math.max(0, Number((location.hash || '').match(/luma-plan=(\d+)/)?.[1]) || 0), Math.max(0, plans.length - 1));
  const nextPlan = () => {
    if (planIndex + 1 >= plans.length) return false;
    location.href = plans[planIndex + 1] + '#luma-plan=' + (planIndex + 1);
    return true;
  };
  const channel = 'luma-sports-player-v1';
  const id = crypto.randomUUID();
  let video = null;
  let frame = null;
  let player = null;
  let state = null;
  let source = window.__lumaSportsSource || 0;
  const attempted = new Set();
  const retiredPlayers = new Set();
  let sourceStarted = 0;
  let progressPosition = null;
  let started = false;
  let sourceButtons = [];
  const sourceKey = 'luma-sports-source-v1';
  let remembered = {};
  if (main) {
    try { remembered = JSON.parse(localStorage.getItem(sourceKey) || '{}'); } catch {}
    if (!remembered || typeof remembered !== 'object' || Array.isArray(remembered)) remembered = {};
  }
  let overlay;
  let message;
  let playButton;
  let score;
  let hideTimer;
  let waitingTimer;
  let lastReport = 0;
  const frames = () => [...document.querySelectorAll('iframe')];
  const send = (target, payload) => target?.postMessage({ channel, ...payload }, '*');
  const publish = payload => {
    if (main) receive(payload);
    else send(window.parent, payload);
  };
  const range = media => {
    if (media.seekable?.length) return [media.seekable.start(0), media.seekable.end(media.seekable.length - 1)];
    return [0, Number.isFinite(media.duration) ? media.duration : 0];
  };
  const report = () => {
    if (!video || !video.isConnected) return;
    const [start, end] = range(video);
    const bounds = video.getBoundingClientRect();
    publish({ type: 'state', id, paused: video.paused, waiting: video.readyState < 3,
      playbackTime: video.currentTime, position: Math.max(0, video.currentTime - start), duration: Math.max(0, end - start),
      area: bounds.width * bounds.height, error: !!video.error });
  };
  const fill = element => {
    if (!element || element === frame) return;
    frame = element;
    const style = document.createElement('style');
    style.textContent = 'html,body{margin:0!important;width:100%!important;height:100%!important;overflow:hidden!important;background:#000!important}body *{visibility:hidden!important}[data-luma-video],[data-luma-video] *{visibility:visible!important}';
    document.documentElement.append(style);
    element.setAttribute('data-luma-video', '');
    element.style.cssText += ';position:fixed!important;inset:0!important;width:100vw!important;height:100vh!important;max-width:none!important;max-height:none!important;border:0!important;margin:0!important;object-fit:contain!important;background:#000!important;z-index:2147483600!important;transform:none!important';
    let parent = element.parentElement;
    while (parent && parent !== document.body) {
      parent.style.setProperty('transform', 'none', 'important');
      parent.style.setProperty('contain', 'none', 'important');
      parent.style.setProperty('overflow', 'visible', 'important');
      parent = parent.parentElement;
    }
  };
  const providerPlay = () => document.querySelectorAll('button[data-playpause], .play-wrapper[data-poster]')[0];
  const play = async () => {
    const control = !video.currentSrc && providerPlay();
    if (control) control.click();
    else await video.play();
  };
  const command = async payload => {
    if (payload.target === id && video) {
      try {
        const [start, end] = range(video);
        if (payload.action === 'toggle') {
          if (video.paused) await play(); else video.pause();
        } else if (payload.action === 'live') video.currentTime = Math.max(start, end - 1);
        else if (payload.action === 'seek' && Number.isFinite(payload.position)) video.currentTime = Math.max(start, Math.min(end, start + payload.position));
        else if (payload.action === 'previous' || payload.action === 'next') video.currentTime = Math.max(start, Math.min(end, video.currentTime + (payload.action === 'next' ? 10 : -10)));
        report();
      } catch {
        publish({ type: 'play-error', id });
      }
    }
    for (const child of frames()) send(child.contentWindow, payload);
  };
  const act = (action, position) => {
    if (!player) { show('Waiting for the stream. Try another source if it does not start.'); return; }
    void command({ type: 'command', target: player, action, position });
    reveal();
  };
  const show = text => { if (message) { message.textContent = text; message.hidden = !text; } };
  const reveal = () => {
    if (!overlay) return;
    overlay.dataset.visible = 'true';
    clearTimeout(hideTimer);
    if (state && !state.paused && !state.waiting) hideTimer = setTimeout(() => {
      overlay.dataset.visible = 'false';
      hideTimer = null;
      if (overlay.contains(document.activeElement)) document.activeElement.blur();
    }, 4500);
  };
  const receive = payload => {
    if (payload.type === 'return') { location.href = 'https://isportsurge.ws/__luma_close__'; return; }
    if (payload.type === 'key') { window.__lumaSportsNavigate(payload.action); return; }
    if (payload.type === 'play-error' && payload.id === player) { show('Playback could not start. Press Play or try another source.'); reveal(); return; }
    if (retiredPlayers.has(payload.id)) return;
    if (payload.type !== 'state' || typeof payload.id !== 'string' || !Number.isFinite(payload.area) || payload.area <= 0) return;
    if (player && player !== payload.id && state && state.area > payload.area) return;
    const wasPlaying = state && !state.paused && !state.waiting;
    player = payload.id;
    state = payload;
    if (!state.paused && !state.waiting && !state.error && Number.isFinite(state.playbackTime)) {
      if (progressPosition !== null && state.playbackTime > progressPosition + 0.1) {
        clearTimeout(waitingTimer);
        if (!started) {
          started = true;
          remembered[location.pathname] = { id: sourceButtons[source]?.id, at: Date.now() };
          const entries = Object.entries(remembered).sort((a, b) => (b[1]?.at || 0) - (a[1]?.at || 0)).slice(0, 64);
          try { localStorage.setItem(sourceKey, JSON.stringify(Object.fromEntries(entries))); } catch {}
        }
      }
      progressPosition = state.playbackTime;
    }
    if (state.error && !started && Date.now() - sourceStarted > 1000) {
      clearTimeout(waitingTimer);
      waitingTimer = setTimeout(nextSource, 0);
    }
    if (playButton) playButton.textContent = state.paused ? 'Play' : 'Pause';
    show(state.error ? 'This source could not play. Try another source.' : state.waiting ? 'Buffering…' : '');
    if (state.paused || state.waiting || state.error) reveal();
    else if (!wasPlaying) reveal();
  };
  window.addEventListener('message', event => {
    const payload = event.data;
    if (!payload || payload.channel !== channel) return;
    if (!main && event.source === window.parent && payload.type === 'command') { void command(payload); return; }
    const child = frames().find(candidate => candidate.contentWindow === event.source);
    if (!child || (main && child.id !== 'cx-iframe' && (!direct || child !== frame)) || !['state', 'return', 'key', 'play-error'].includes(payload.type)) return;
    if (payload.type === 'state' && payload.area > 0) fill(child);
    publish(payload);
  });
  const nextSource = () => {
    if (started) return;
    const next = sourceButtons.findIndex((_, index) => !attempted.has(index));
    if (next < 0) {
      if (nextPlan()) return;
      show('None of this game’s sources started. The broadcast may be offline or not live yet. Try again or return to Sports.');
      reveal();
      return;
    }
    source = next;
    selectSource();
  };
  const selectSource = () => {
    sourceButtons = [...document.querySelectorAll('[id^="stream-btn-"]')];
    if (direct) sourceButtons = [{ id: location.pathname + location.search, click() {} }];
    if (!sourceButtons.length) { if (nextPlan()) return; show('No sources are available for this event. Return to Sports and refresh.'); return; }
    if (!attempted.size && !source) {
      const saved = remembered[location.pathname];
      const preferred = saved && Date.now() - saved.at < 86400000 ? sourceButtons.findIndex(button => button.id === saved.id) : -1;
      if (preferred >= 0) source = preferred;
    }
    source %= sourceButtons.length;
    attempted.add(source);
    if (player) retiredPlayers.add(player);
    player = null;
    state = null;
    started = false;
    progressPosition = null;
    sourceStarted = Date.now();
    sourceButtons[source].click();
    const stream = document.getElementById('cx-iframe');
    if (stream) fill(stream);
    show(`Trying source ${source + 1} of ${sourceButtons.length}...`);
    reveal();
    clearTimeout(waitingTimer);
    waitingTimer = setTimeout(nextSource, 12000);
  };
  const mount = () => {
    if (main) {
      const style = document.createElement('style');
      style.textContent = `#luma-sports-player,#luma-sports-player *{visibility:visible!important;box-sizing:border-box}#luma-sports-player{position:fixed;inset:0;z-index:2147483647;pointer-events:none;color:#f8f8fa;font-family:system-ui,sans-serif;transition:opacity .18s}#luma-sports-player[data-visible=false]{opacity:0}#luma-sports-player header{position:absolute;top:0;left:0;right:0;padding:4vh 4vw 7vh;background:linear-gradient(#000d,transparent)}#luma-sports-player h1{font-size:clamp(22px,2.4vw,42px);margin:0 0 10px;letter-spacing:-.03em}#luma-sports-player p{font-size:clamp(16px,1.4vw,26px);margin:0;color:#d5d8e2}#luma-sports-player footer{position:absolute;bottom:0;left:0;right:0;padding:8vh 4vw 4vh;background:linear-gradient(transparent,#000e);display:flex;gap:12px;flex-wrap:wrap}#luma-sports-player button{pointer-events:auto;min-height:58px;padding:12px 24px;border:1px solid #ffffff40;border-radius:12px;background:#222630e6;color:#fff;font:600 clamp(16px,1.2vw,23px) system-ui;cursor:pointer}#luma-sports-player button:focus{outline:4px solid #bdb4ff;outline-offset:4px;background:#44405d}#luma-sports-player[data-visible=false] button{pointer-events:none}#luma-sports-message{position:absolute;top:45%;left:15%;right:15%;text-align:center;padding:24px;border-radius:16px;background:#0b0d12e6;font-size:clamp(18px,1.6vw,28px)}@media(prefers-reduced-motion:reduce){#luma-sports-player{transition:none}}`;
      document.documentElement.append(style);
      overlay = document.createElement('section');
      overlay.id = 'luma-sports-player';
      overlay.setAttribute('aria-label', 'Sports player');
      const header = document.createElement('header');
      const title = document.createElement('h1');
      title.textContent = window.__lumaSportsTitle || document.title.replace(/ Live Stream.*$/, '');
      score = document.createElement('p');
      score.textContent = window.__lumaSportsScore || '';
      header.append(title, score);
      message = document.createElement('div');
      message.id = 'luma-sports-message';
      message.setAttribute('role', 'status');
      const footer = document.createElement('footer');
      for (const [label, action] of [['Back to Sports', () => { location.href = 'https://isportsurge.ws/__luma_close__'; }], ['Rewind 10s', () => act('previous')], ['Play', () => act('toggle')], ['Forward 10s', () => act('next')], ['Go live', () => act('live')], ['Try another source', () => { if (direct && nextPlan()) return; attempted.clear(); source++; selectSource(); }]]) {
        const button = document.createElement('button');
        button.textContent = label;
        button.addEventListener('click', action);
        button.addEventListener('focus', reveal);
        if (label === 'Play') playButton = button;
        footer.append(button);
      }
      overlay.append(header, message, footer);
      document.body.append(overlay);
      window.__lumaSportsControl = act;
      window.__lumaSportsScoreUpdate = text => { score.textContent = text; };
      window.__lumaSportsTimeline = position => {
        if (!state || !Number.isFinite(state.duration) || state.duration <= 0) return null;
        if (Number.isFinite(position)) act('seek', position);
        return { position: Number.isFinite(position) ? position : state.position, duration: state.duration };
      };
      window.__lumaSportsNavigate = action => {
        if (action === 'back') {
          if (overlay.dataset.visible === 'true' && state && !state.paused) { overlay.dataset.visible = 'false'; return; }
          location.href = 'https://isportsurge.ws/__luma_close__';
          return;
        }
        const buttons = [...footer.querySelectorAll('button')];
        const wasHidden = overlay.dataset.visible === 'false';
        reveal();
        if (wasHidden) { playButton.focus(); return; }
        if (action === 'activate') { if (buttons.includes(document.activeElement)) document.activeElement.click(); else playButton.focus(); return; }
        const index = buttons.indexOf(document.activeElement);
        buttons[index < 0 ? 2 : (index + (action === 'left' || action === 'up' ? -1 : 1) + buttons.length) % buttons.length]?.focus();
      };
      document.addEventListener('pointermove', reveal, { passive: true });
      selectSource();
    }
    const seen = new WeakSet();
    const initialized = new WeakSet();
    const observe = () => {
      if (direct && !video) { const embeds = frames().filter(child => child.getAttribute('src')); embeds.sort((a, b) => b.clientWidth * b.clientHeight - a.clientWidth * a.clientHeight); if (embeds[0]) fill(embeds[0]); }
      const videos = [...document.querySelectorAll('video')];
      videos.sort((a, b) => b.clientWidth * b.clientHeight - a.clientWidth * a.clientHeight);
      if (videos[0] && videos[0] !== video) {
        video = videos[0];
        video.controls = false;
        fill(video);
      }
      for (const media of videos) {
        if (seen.has(media)) continue;
        seen.add(media);
        for (const event of ['loadedmetadata', 'playing', 'pause', 'waiting', 'error', 'seeked', 'durationchange']) media.addEventListener(event, report);
        media.addEventListener('timeupdate', () => { if (Date.now() - lastReport >= 1000) { lastReport = Date.now(); report(); } });
        if (media.currentSrc) media.play().catch(() => publish({ type: 'play-error', id }));
      }
      if (video && !video.currentSrc && !initialized.has(video) && providerPlay()) {
        initialized.add(video);
        void play().catch(() => publish({ type: 'play-error', id }));
      }
      report();
    };
    observe();
    const observer = new MutationObserver(observe);
    observer.observe(document.body, { childList: true, subtree: true });
    window.addEventListener('pagehide', () => { observer.disconnect(); clearTimeout(hideTimer); clearTimeout(waitingTimer); }, { once: true });
  };
  document.addEventListener('keydown', event => {
    let action = { ArrowLeft: 'left', ArrowRight: 'right', ArrowUp: 'up', ArrowDown: 'down', Enter: 'activate', Escape: 'back', Backspace: 'back' }[event.key];
    if (event.ctrlKey && event.key.toLowerCase() === 'w') action = 'return';
    if (event.code === 'Space' || event.key === 'MediaPlayPause') action = 'toggle';
    if (!action) return;
    event.preventDefault();
    event.stopImmediatePropagation();
    if (main) {
      if (action === 'return') location.href = 'https://isportsurge.ws/__luma_close__';
      else if (action === 'toggle') act('toggle');
      else window.__lumaSportsNavigate?.(action);
    } else if (action === 'toggle' && video) void command({ target: id, action: 'toggle' });
    else publish({ type: action === 'return' ? 'return' : 'key', action });
  }, true);
  if (document.readyState === 'loading') document.addEventListener('DOMContentLoaded', mount, { once: true });
  else mount();
})();
