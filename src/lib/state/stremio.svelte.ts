import type { StremioStatus } from '../types';
import { native, stremioOpen, stremioOpenDetail, stremioStatus, stremioSync } from '../services/tauri';
import { listen } from '@tauri-apps/api/event';

class StremioStore {
  status = $state.raw<StremioStatus | null>(null);
  error = $state('');
  loading = $state(false);
  private syncedAt = 0;

  get recent() {
    return this.status?.items.filter(item => (item.lastWatched || item.timeOffset > 0) && item.kind !== 'other') ?? [];
  }

  async start() {
    if (!native) return () => {};
    const stop = await listen('stremio-closed', () => { void this.refresh(true); });
    await this.refresh();
    return stop;
  }

  async refresh(sync = false) {
    if (!native || this.loading) return this.status;
    this.loading = true;
    try {
      this.status = await stremioStatus();
      if (sync && this.status.connected) {
        this.status = await stremioSync();
        this.syncedAt = Date.now();
      }
      this.error = '';
    } catch (error) {
      this.error = String(error);
    } finally {
      this.loading = false;
    }
    return this.status;
  }

  async refreshIfStale() {
    if (this.loading || Date.now() - this.syncedAt < 60_000) return;
    this.syncedAt = Date.now();
    await this.refresh(true);
  }

  async sync() {
    this.status = await stremioSync();
    this.syncedAt = Date.now();
    this.error = '';
    return this.status;
  }

  open(id: string) {
    return stremioOpen(id);
  }

  openDetail(kind: string, id: string) {
    return stremioOpenDetail(kind, id);
  }
}

export function createStremio() {
  return new StremioStore();
}

export type StremioState = StremioStore;
