export type SaverMode = 'movies' | 'smart' | 'photos' | 'music' | 'clock' | 'black' | 'room' | 'video';
export type SaverSettings = {
  enabled: boolean; mode: SaverMode; minutes: number; seconds: number;
  shuffle: boolean; fit: 'contain' | 'cover'; clock: boolean; music: boolean;
  brightness: number; player: boolean; placement: 'left' | 'right';
};

const defaults: SaverSettings = {
  enabled: true, mode: 'movies', minutes: 5, seconds: 15, shuffle: true,
  fit: 'contain', clock: false, music: true, brightness: 80, player: true, placement: 'left',
};

export function loadSaver(): SaverSettings {
  try { return { ...defaults, ...JSON.parse(localStorage.getItem('luma.screensaver') ?? '{}') }; }
  catch { return { ...defaults }; }
}

export type SaverLibrary = { photos: File[]; video?: File };

export async function saverLibrary(value?: SaverLibrary): Promise<SaverLibrary> {
  const db = await new Promise<IDBDatabase>((resolve, reject) => {
    const request = indexedDB.open('luma-screensaver', 1);
    request.onupgradeneeded = () => request.result.createObjectStore('library');
    request.onsuccess = () => resolve(request.result);
    request.onerror = () => reject(request.error);
  });
  try {
    return await new Promise((resolve, reject) => {
      const transaction = db.transaction('library', value ? 'readwrite' : 'readonly');
      const store = transaction.objectStore('library');
      const request = value ? store.put(value, 'media') : store.get('media');
      transaction.oncomplete = () => resolve(value ?? request.result ?? { photos: [] });
      transaction.onabort = () => reject(transaction.error);
    });
  } finally { db.close(); }
}
