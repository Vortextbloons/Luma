import { invoke } from '@tauri-apps/api/core';
import type { ActivitySnapshot, ProviderConfigInput, RefreshResult } from '../types';
import { activitySnapshot, onActivity, openActivity } from './tauri';

export { activitySnapshot, onActivity, openActivity };
export * from './activitySelectors';

export const refreshProvider = (provider: string) => invoke<RefreshResult>('refresh_provider', { provider });
export const clearActivity = (args: { itemId?: string; provider?: string } = {}) => invoke<ActivitySnapshot>('clear_activity', { itemId: args.itemId ?? null, provider: args.provider ?? null });
export const setActivityTracking = (enabled: boolean) => invoke<ActivitySnapshot>('set_activity_tracking', { enabled });
export const setProviderEnabled = (provider: string, enabled: boolean) => invoke<ActivitySnapshot>('set_provider_enabled', { provider, enabled });
export const setProviderHidden = (provider: string, hidden: boolean) => invoke<ActivitySnapshot>('set_provider_hidden', { provider, hidden });
export const saveProviderConfig = (provider: string, config: ProviderConfigInput) => invoke<ActivitySnapshot>('save_provider_config', { provider, config });
export const disconnectProvider = (provider: string) => invoke<ActivitySnapshot>('disconnect_provider', { provider });
