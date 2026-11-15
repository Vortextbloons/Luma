import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { Settings } from '../types';

export type CustomizationConfig = Pick<Settings, 'theme' | 'density' | 'appTileShape' | 'home' | 'widgets' | 'order'>;
export interface CustomizationDocument { version: number; revision: number; config: CustomizationConfig }

export function customizationConfig(settings: Settings): CustomizationConfig {
  return JSON.parse(JSON.stringify({ theme: settings.theme, density: settings.density, appTileShape: settings.appTileShape, home: settings.home, widgets: settings.widgets, order: settings.order }));
}

function errorMessage(error: unknown): string {
  return error && typeof error === 'object' && 'message' in error ? String(error.message) : String(error);
}

export function createCustomizationSync(settings: Settings, report: (message: string) => void) {
  let current: CustomizationDocument | null = null;
  let desired = customizationConfig(settings);
  let ready = false;
  let disposed = false;
  let saving = false;
  let remote: CustomizationDocument | null = null;
  let stop: (() => void) | undefined;
  const same = (a: CustomizationConfig, b: CustomizationConfig) => JSON.stringify(a) === JSON.stringify(b);
  function apply(document: CustomizationDocument) {
    if (current && document.revision < current.revision) return;
    current = document;
    desired = JSON.parse(JSON.stringify(document.config));
    Object.assign(settings, JSON.parse(JSON.stringify(document.config)));
  }
  async function save() {
    if (!ready || disposed || saving || !current || same(desired, current.config)) return;
    saving = true;
    const candidate = desired;
    try {
      const document = await invoke<CustomizationDocument>('customization_commit', { request: { expectedRevision: current.revision, config: candidate } });
      if (disposed) return;
      current = document;
      if (remote && remote.revision > document.revision) {
        apply(remote);
        report('Customization changed elsewhere. The latest saved layout has been loaded.');
      } else if (same(desired, candidate)) apply(document);
    } catch (error) {
      if (disposed) return;
      if (error && typeof error === 'object' && 'code' in error && error.code === 'conflict') {
        try {
          const document = await invoke<CustomizationDocument | null>('customization_read');
          if (document && !disposed) apply(document);
        } catch { }
      }
      report(`Customization could not be saved. ${errorMessage(error)}`);
      return;
    } finally {
      saving = false;
      remote = null;
    }
    void save();
  }
  return {
    async start() {
      try {
        const unsubscribe = await listen<CustomizationDocument>('customization-changed', event => {
          if (disposed) return;
          if (current && event.payload.revision <= current.revision) return;
          if (!ready || saving) {
            if (!remote || event.payload.revision > remote.revision) remote = event.payload;
          } else apply(event.payload);
        });
        if (disposed) { unsubscribe(); return; }
        stop = unsubscribe;
        const document = await invoke<CustomizationDocument>('customization_initialize', { config: desired });
        if (disposed) return;
        apply(remote && remote.revision > document.revision ? remote : document);
        remote = null;
        ready = true;
      } catch (error) {
        if (!disposed) report(`Customization storage is unavailable. ${errorMessage(error)}`);
      }
    },
    update(config: CustomizationConfig) {
      if (same(desired, config)) return;
      desired = config;
      void save();
    },
    dispose() { disposed = true; stop?.(); },
  };
}
