import type { HomeRow, HomeRowId } from '../types';

export const homeRowIds: HomeRowId[] = ['hero', 'continue', 'apps', 'popular', 'featured', 'recent', 'recentApps'];

export const homeRowMeta: Record<HomeRowId, { label: string; detail: string }> = {
  hero: { label: 'Featured hero', detail: 'Large spotlight for the movie, show, or track you are previewing.' },
  continue: { label: 'Continue Watching', detail: 'In-progress movies and shows from this PC and connected services.' },
  apps: { label: 'Your apps', detail: 'The app dock on the home screen.' },
  popular: { label: 'Popular movies', detail: 'The shared movie catalog.' },
  featured: { label: 'Top-rated movies', detail: 'Movies ranked by IMDb rating.' },
  recent: { label: 'Recently played', detail: 'A quick row of the titles you played most recently.' },
  recentApps: { label: 'Recently used apps', detail: 'Apps you opened from Luma on this PC.' },
};

const defaultVisible: Record<HomeRowId, boolean> = {
  hero: true, continue: true, apps: true, popular: true, featured: true, recent: true, recentApps: true,
};

export function defaultHomeRows(): HomeRow[] {
  return homeRowIds.map(id => ({ id, component: id, span: 2, visible: defaultVisible[id] }));
}

export function legacyHomeRows(sections: unknown): HomeRow[] {
  const values = sections && typeof sections === 'object' ? sections as Partial<Record<HomeRowId, unknown>> : {};
  return homeRowIds.map(id => ({ id, component: id, span: 2, visible: typeof values[id] === 'boolean' ? values[id] : defaultVisible[id] }));
}

export function sanitizeHomeRows(value: unknown): HomeRow[] | null {
  if (!Array.isArray(value)) return null;
  const seen = new Set<HomeRowId>();
  const instances = new Set<string>();
  const rows: HomeRow[] = [];
  for (const entry of value) {
    if (!entry || typeof entry !== 'object') continue;
    const id = (entry as { id?: unknown }).id;
    const component = (entry as { component?: unknown }).component ?? id;
    if (typeof id !== 'string' || !/^[A-Za-z0-9_-]{1,64}$/.test(id) || typeof component !== 'string' || !homeRowIds.includes(component as HomeRowId) || seen.has(component as HomeRowId) || instances.has(id)) continue;
    seen.add(component as HomeRowId);
    instances.add(id);
    rows.push({ id, component: component as HomeRowId, span: (entry as { span?: unknown }).span === 1 ? 1 : 2, visible: (entry as { visible?: unknown }).visible !== false });
  }
  if (!rows.length) return null;
  if (!seen.has('hero')) {
    const id = unusedInstance('hero', instances);
    rows.unshift({ id, component: 'hero', span: 2, visible: true });
    instances.add(id);
    seen.add('hero');
  }
  for (const component of homeRowIds) if (component !== 'hero' && !seen.has(component)) {
    const id = unusedInstance(component, instances);
    rows.push({ id, component, span: 2, visible: defaultVisible[component] });
    instances.add(id);
  }
  return rows;
}

function unusedInstance(component: HomeRowId, instances: Set<string>): string {
  let id: string = component;
  for (let index = 1; instances.has(id); index++) id = `${component}-${index}`;
  return id;
}

export function moveHomeRow(rows: HomeRow[], id: string, direction: -1 | 1): HomeRow[] {
  const index = rows.findIndex(row => row.id === id);
  const target = index + direction;
  if (index < 0 || target < 0 || target >= rows.length) return rows;
  const next = [...rows];
  [next[index], next[target]] = [next[target], next[index]];
  return next;
}

export function moveHomeRowTo(rows: HomeRow[], id: string, index: number): HomeRow[] {
  const from = rows.findIndex(row => row.id === id);
  if (from < 0) return rows;
  const target = Math.max(0, Math.min(rows.length - 1, index));
  if (from === target) return rows;
  const next = [...rows];
  const [row] = next.splice(from, 1);
  next.splice(target, 0, row);
  return next;
}

export function toggleHomeRow(rows: HomeRow[], id: string): HomeRow[] {
  return rows.map(row => row.id === id ? { ...row, visible: !row.visible } : row);
}
