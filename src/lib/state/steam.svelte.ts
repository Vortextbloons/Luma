import type { SteamGame } from '../types';
import { native, steamOpenGame, steamOpenStore, steamRecentGames, steamStoreRecommendations } from '../services/tauri';

class SteamStore {
  games = $state.raw<SteamGame[]>([]);
  recommendations = $state.raw<SteamGame[]>([]);
  recommendationSource = $state.raw<SteamGame | null>(null);
  private refreshing = false;
  private refreshedAt = 0;

  async refresh(force = false) {
    if (!native || this.refreshing || (!force && this.refreshedAt && Date.now() - this.refreshedAt < 60_000)) return;
    this.refreshing = true;
    try {
      this.games = await steamRecentGames();
      this.refreshedAt = Date.now();
      const source = this.games[0];
      if (!source) {
        this.recommendationSource = null;
        this.recommendations = [];
      } else if (source.appid !== this.recommendationSource?.appid || !this.recommendations.length) {
        this.recommendationSource = source;
        this.recommendations = await steamStoreRecommendations(source.appid).catch(() => []);
      }
    } catch {
      return;
    } finally {
      this.refreshing = false;
    }
  }

  openGame(appid: number) {
    return steamOpenGame(appid);
  }

  openStore(appid: number) {
    return steamOpenStore(appid);
  }
}

export function createSteam() {
  return new SteamStore();
}

export type SteamState = SteamStore;
