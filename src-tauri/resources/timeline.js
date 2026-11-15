position => {
  try {
    const videos = [];
    const collect = root => {
      videos.push(...root.querySelectorAll('video'));
      for (const element of root.querySelectorAll('*')) {
        if (element.shadowRoot) collect(element.shadowRoot);
        if (element.tagName === 'IFRAME') {
          try { if (element.contentDocument) collect(element.contentDocument); } catch (_) {}
        }
      }
    };
    collect(document);
    const candidates = videos.filter(video => Number.isFinite(video.duration) && video.duration > 0);
    candidates.sort((a, b) => Number(b.getClientRects().length > 0) - Number(a.getClientRects().length > 0) || Number(a.paused) - Number(b.paused));
    const video = candidates[0];
    if (!video) return null;
    if (position !== null && Number.isFinite(position)) video.currentTime = Math.max(0, Math.min(video.duration, position));
    return { position: video.currentTime, duration: video.duration };
  } catch (_) {
    return { error: 'The player could not change playback position. Try again.' };
  }
}
