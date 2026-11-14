import type {
  ActivityItem, ActivitySnapshot, OpenOutcome, ProviderConfigInput, ProviderStatus, RefreshResult,
} from '../types';
import {
  clearActivity, disconnectProvider, openActivity, refreshProvider, saveProviderConfig,
  setActivityTracking, setProviderEnabled, setProviderHidden,
} from '../services/activity';
import { activitySnapshot, onActivity } from '../services/tauri';

const empty: ActivitySnapshot = { items: [], providers: [], trackingEnabled: true, generatedAt: '' };

class ActivityStore {
  snapshot = $state.raw<ActivitySnapshot>(empty);
  loading = $state(false);
  error = $state('');
  private started = false;
  private stop: (() => void) | undefined;

  get items(): ActivityItem[] {
    return this.snapshot.items;
  }

  get providers(): ProviderStatus[] {
    return this.snapshot.providers;
  }

  get trackingEnabled(): boolean {
    return this.snapshot.trackingEnabled;
  }

  apply(snapshot: ActivitySnapshot) {
    this.snapshot = snapshot;
  }

  async load() {
    this.loading = true;
    try {
      this.apply(await activitySnapshot());
      this.error = '';
    } catch (cause) {
      this.error = String(cause);
    } finally {
      this.loading = false;
    }
  }

  async start() {
    if (this.started) return () => {};
    this.started = true;
    try {
      this.stop = await onActivity(snapshot => this.apply(snapshot));
    } catch (cause) {
      this.error = String(cause);
      this.started = false;
    }
    await this.load();
    return () => {
      this.stop?.();
      this.stop = undefined;
      this.started = false;
    };
  }

  async open(id: string, fullscreen: boolean): Promise<OpenOutcome> {
    return openActivity(id, fullscreen);
  }

  async remove(itemId: string) {
    this.apply(await clearActivity({ itemId }));
  }

  async clear(provider?: string) {
    this.apply(await clearActivity(provider ? { provider } : {}));
  }

  async setTracking(enabled: boolean) {
    this.apply(await setActivityTracking(enabled));
  }

  async setEnabled(provider: string, enabled: boolean) {
    this.apply(await setProviderEnabled(provider, enabled));
  }

  async setHidden(provider: string, hidden: boolean) {
    this.apply(await setProviderHidden(provider, hidden));
  }

  async saveConfig(provider: string, config: ProviderConfigInput) {
    this.apply(await saveProviderConfig(provider, config));
  }

  async disconnect(provider: string) {
    this.apply(await disconnectProvider(provider));
  }

  async refresh(provider: string): Promise<RefreshResult> {
    const result = await refreshProvider(provider);
    this.apply(result.snapshot);
    if (result.error) this.error = result.error;
    else this.error = '';
    return result;
  }
}

export function createActivity() {
  return new ActivityStore();
}

export type ActivityState = ActivityStore;
