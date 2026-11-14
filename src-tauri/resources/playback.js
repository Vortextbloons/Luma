async action => {
  const video = document.querySelector('video');
  if (!video) return;
  if (action === 'toggle') {
    if (video.paused) await video.play();
    else video.pause();
  } else if (action === 'previous' || action === 'next') {
    const end = Number.isFinite(video.duration) ? video.duration : video.currentTime + 10;
    video.currentTime = Math.max(0, Math.min(end, video.currentTime + (action === 'next' ? 10 : -10)));
  }
}
