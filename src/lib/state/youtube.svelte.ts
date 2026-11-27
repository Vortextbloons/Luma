import type { YouTubeFeedItem } from '../types';
import { native, onYouTubeFeed, openYouTubeVideo, youtubeFeed, type YouTubeFeed } from '../services/tauri';

class YouTubeStore {
  feed = $state.raw<YouTubeFeedItem[] | null>(null);
  failed = $state(false);

  async start() {
    if (!native) return () => {};
    let received = false;
    const apply = (result: YouTubeFeed) => {
      this.feed = result.items;
      this.failed = result.failed;
    };
    const stop = await onYouTubeFeed(result => { received = true; apply(result); });
    void youtubeFeed().then(result => { if (!received) apply(result); }).catch(() => {
      if (!received) this.failed = true;
    });
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
