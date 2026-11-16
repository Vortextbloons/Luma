import type { SteamGame } from '../types';
import { native, steamOpenGame, steamOpenStore, steamRecentGames, steamStoreRecommendations } from '../services/tauri';

class SteamStore {
  games = $state.raw<SteamGame[]>([]);
  recommendations = $state.raw<SteamGame[]>([]);
  recommendationSource = $state.raw<SteamGame | null>(null);
  private refreshing = false;

  async refresh() {
    if (!native || this.refreshing) return;
    this.refreshing = true;
    this.games = await steamRecentGames().catch(() => this.games);
    const source = this.games[0];
    if (source && (source.appid !== this.recommendationSource?.appid || !this.recommendations.length)) {
      this.recommendationSource = source;
      this.recommendations = await steamStoreRecommendations(source.appid).catch(() => []);
    }
    this.refreshing = false;
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
