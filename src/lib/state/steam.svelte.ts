import type { SteamGame, SteamShortcutInfo } from '../types';
import { native, steamAddShortcut, steamOpenGame, steamOpenStore, steamRecentGames, steamRemoveShortcut, steamShortcuts, steamStoreRecommendations } from '../services/tauri';

class SteamStore {
  games = $state.raw<SteamGame[]>([]);
  shortcuts = $state.raw<SteamShortcutInfo[]>([]);
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
      const source = this.games.find(game => game.appid < 0x80000000);
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

  async loadShortcuts() {
    if (!native) return [];
    this.shortcuts = await steamShortcuts().catch(() => []);
    return this.shortcuts;
  }

  async addShortcut(name: string, exe: string, startDir?: string) {
    const added = await steamAddShortcut(name, exe, startDir);
    await Promise.all([this.loadShortcuts(), this.refresh(true)]);
    return added;
  }

  async removeShortcut(appid: number) {
    await steamRemoveShortcut(appid);
    await Promise.all([this.loadShortcuts(), this.refresh(true)]);
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
