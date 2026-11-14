import { invoke } from '@tauri-apps/api/core';
import { getVersion } from '@tauri-apps/api/app';
import { check, type Update } from '@tauri-apps/plugin-updater';
import { native } from '../services/tauri';

type Phase = 'idle' | 'checking' | 'current' | 'available' | 'downloading' | 'ready' | 'installing' | 'unconfigured';
let update: Update | null = null;
let initialized = false;
export const updates = $state({
  phase: 'idle' as Phase, version: '', nextVersion: '', notes: '', error: '', downloaded: 0, total: 0, dismissedVersion: '',
});

export async function checkUpdates() {
  if (!native || ['checking', 'downloading', 'ready', 'installing', 'unconfigured'].includes(updates.phase)) return;
  updates.phase = 'checking';
  updates.error = '';
  try {
    if (update) { await update.close(); update = null; }
    update = await check({ timeout: 15000 });
    updates.nextVersion = update?.version ?? '';
    updates.notes = update?.body ?? '';
    updates.phase = update ? 'available' : 'current';
  } catch {
    updates.phase = 'idle';
    updates.error = 'Could not check for updates. Check your internet connection and try again.';
  }
}

export async function initializeUpdates() {
  if (initialized || !native) return;
  initialized = true;
  try {
    updates.version = await getVersion();
    if (!await invoke<boolean>('updates_enabled')) { updates.phase = 'unconfigured'; return; }
    await checkUpdates();
  } catch {
    initialized = false;
    updates.error = 'Could not initialize updates. Reopen Settings to try again.';
  }
}

export function dismissUpdate() { updates.dismissedVersion = updates.nextVersion; }

export async function downloadUpdate() {
  if (!update || updates.phase !== 'available') return;
  updates.phase = 'downloading';
  updates.error = '';
  updates.downloaded = 0;
  updates.total = 0;
  try {
    await update.download(event => {
      if (event.event === 'Started') updates.total = event.data.contentLength ?? 0;
      if (event.event === 'Progress') updates.downloaded += event.data.chunkLength;
    }, { timeout: 120000 });
    updates.phase = 'ready';
  } catch {
    updates.phase = 'available';
    updates.error = 'The update could not be downloaded or verified. Try downloading it again.';
  }
}

export async function installUpdate() {
  if (!update || updates.phase !== 'ready') return;
  updates.phase = 'installing';
  updates.error = '';
  try {
    await update.install({ restartAfterInstall: true });
  } catch {
    updates.phase = 'ready';
    updates.error = 'The installer could not start. Try Restart to update again.';
  }
}
