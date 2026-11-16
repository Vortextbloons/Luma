import type { YouTubeFeedItem } from '../types';
import { native, onYouTubeFeed, openYouTubeVideo, youtubeFeed } from '../services/tauri';

class YouTubeStore {
  feed = $state.raw<YouTubeFeedItem[]>([]);

  async start() {
    if (!native) return () => {};
    const stop = await onYouTubeFeed(items => { this.feed = items; });
    void youtubeFeed().then(items => { this.feed = items; }).catch(() => {});
    return stop;
  }

  openVideo(id: string, fullscreen: boolean) {
    return openYouTubeVideo(id, fullscreen);
  }
}

export function createYouTube() {
  return new YouTubeStore();
}

export type YouTubeState = YouTubeStore;
