position => {
  const video = [...document.querySelectorAll('video')].find(video => video.duration > 0 && video.getClientRects().length);
  if (!video || !Number.isFinite(video.duration) || !video.seekable.length) return null;
  const start = video.seekable.start(0);
  const end = video.seekable.end(video.seekable.length - 1);
  if (position !== null && Number.isFinite(position)) video.currentTime = Math.max(start, Math.min(end, position));
  return { position: video.currentTime, duration: video.duration };
}
