import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { Settings } from '../types';

export type CustomizationConfig = Pick<Settings, 'theme' | 'density' | 'appTileShape' | 'textSize' | 'reducedMotion' | 'interfaceSounds' | 'soundLevel' | 'home' | 'homeColumns' | 'widgets' | 'order'>;
export interface CustomizationDocument { version: number; revision: number; config: CustomizationConfig }
export interface CustomizationPreview { id: string; baseRevision: number; config: CustomizationConfig }
export interface HomeDraft { home: Settings['home']; homeColumns: 1 | 2; baseRevision: number }

export function customizationConfig(settings: Settings): CustomizationConfig {
  return JSON.parse(JSON.stringify({ theme: settings.theme, density: settings.density, appTileShape: settings.appTileShape, textSize: settings.textSize, reducedMotion: settings.reducedMotion, interfaceSounds: settings.interfaceSounds, soundLevel: settings.soundLevel, home: settings.home, homeColumns: settings.homeColumns, widgets: settings.widgets, order: settings.order }));
}

function errorMessage(error: unknown): string {
  return error && typeof error === 'object' && 'message' in error ? String(error.message) : String(error);
}

export function createCustomizationSync(settings: Settings, report: (message: string) => void, callbacks: { document?: (document: CustomizationDocument) => void; preview?: (preview: CustomizationPreview | null) => void } = {}) {
  let current: CustomizationDocument | null = null;
  let desired = customizationConfig(settings);
  let ready = false;
  let disposed = false;
  let saving = false;
  let remote: CustomizationDocument | null = null;
  let stop: (() => void) | undefined;
  let stopPreview: (() => void) | undefined;
  let activePreview: CustomizationPreview | null = null;
  let previewRead = 0;
  function setPreview(preview: CustomizationPreview | null) {
    if (preview && current && preview.baseRevision < current.revision) return;
    activePreview = preview;
    callbacks.preview?.(preview);
  }
  async function refreshPreview() {
    const request = ++previewRead;
    const preview = await invoke<CustomizationPreview | null>('customization_preview_read');
    if (!disposed && request === previewRead) setPreview(preview);
  }
  async function reconcileFailure(error: unknown) {
    try {
      if (error && typeof error === 'object' && 'code' in error && (error.code === 'conflict' || error.code === 'preview_mismatch')) {
        const document = await invoke<CustomizationDocument | null>('customization_read');
        if (document && !disposed) apply(document);
      }
      await refreshPreview();
    } catch { }
  }
  const same = (a: CustomizationConfig, b: CustomizationConfig) => JSON.stringify(a) === JSON.stringify(b);
  function apply(document: CustomizationDocument) {
    if (current && document.revision < current.revision) return;
    current = document;
    desired = JSON.parse(JSON.stringify(document.config));
    Object.assign(settings, JSON.parse(JSON.stringify(document.config)));
    callbacks.document?.(document);
    if (activePreview && activePreview.baseRevision < document.revision) setPreview(null);
  }
  async function save() {
    if (!ready || disposed || saving || !current || same(desired, current.config)) return;
    saving = true;
    const candidate = desired;
    try {
      const document = await invoke<CustomizationDocument>('customization_commit', { request: { expectedRevision: current.revision, config: candidate } });
      if (disposed) return;
      current = document;
      callbacks.document?.(document);
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
        const unsubscribePreview = await listen('customization-preview-changed', () => {
          if (ready && !disposed) void refreshPreview().catch(error => { if (!disposed) report(errorMessage(error)); });
        });
        if (disposed) { unsubscribePreview(); return; }
        stopPreview = unsubscribePreview;
        const document = await invoke<CustomizationDocument>('customization_initialize', { config: desired });
        if (disposed) return;
        apply(remote && remote.revision > document.revision ? remote : document);
        remote = null;
        ready = true;
        await refreshPreview();
      } catch (error) {
        if (!disposed) report(`Customization storage is unavailable. ${errorMessage(error)}`);
      }
    },
    update(config: CustomizationConfig) {
      if (same(desired, config)) return;
      desired = config;
      void save();
    },
    async preview(config: CustomizationConfig, expectedRevision: number) {
      if (!ready || saving) throw new Error('Wait for preferences to finish saving, then preview again.');
      try {
        const candidate = await invoke<CustomizationPreview>('customization_preview', { request: { expectedRevision, config } });
        if (!disposed) setPreview(candidate);
        await refreshPreview();
      } catch (error) { await reconcileFailure(error); throw error; }
    },
    async keep() {
      if (!activePreview) throw new Error('This preview is no longer active. Preview your draft again.');
      try {
        const document = await invoke<CustomizationDocument>('customization_preview_keep', { request: { previewId: activePreview.id } });
        if (!disposed) apply(document);
        await refreshPreview();
      } catch (error) { await reconcileFailure(error); throw error; }
    },
    async cancel() {
      if (!activePreview) return;
      try {
        await invoke('customization_preview_cancel', { request: { previewId: activePreview.id } });
        await refreshPreview();
      } catch (error) { await reconcileFailure(error); throw error; }
    },
    async restore() {
      if (!ready || saving || !current) throw new Error('Wait for preferences to finish saving, then restore again.');
      try {
        const document = await invoke<CustomizationDocument>('customization_restore', { expectedRevision: current.revision });
        if (!disposed) apply(document);
        await refreshPreview();
      } catch (error) { await reconcileFailure(error); throw error; }
    },
    async restoreLayout() {
      if (!ready || saving || !current) throw new Error('Wait for preferences to finish saving, then restore the layout again.');
      try {
        const document = await invoke<CustomizationDocument>('customization_restore_layout', { expectedRevision: current.revision });
        if (!disposed) apply(document);
        await refreshPreview();
      } catch (error) { await reconcileFailure(error); throw error; }
    },
    dispose() { disposed = true; previewRead++; stop?.(); stopPreview?.(); },
  };
}
