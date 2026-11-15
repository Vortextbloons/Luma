import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync, existsSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { registerHooks } from 'node:module';
import { compile } from 'svelte/compiler';
import { render } from 'svelte/server';
import { defaultHomeRows, moveHomeRowTo } from './home.ts';

registerHooks({
  resolve(specifier, context, next) {
    if (specifier.startsWith('.') && context.parentURL?.startsWith('file:')) {
      const candidate = new URL(specifier, context.parentURL);
      if (existsSync(`${fileURLToPath(candidate)}.ts`)) return next(`${candidate.href}.ts`, context);
    }
    return next(specifier, context);
  },
  load(url, context, next) {
    if (url.endsWith('.svelte')) {
      const filename = fileURLToPath(url);
      return { format: 'module', source: compile(readFileSync(filename, 'utf8'), { filename, generate: 'server' }).js.code, shortCircuit: true };
    }
    if (/\.(svg|png)$/.test(url)) return { format: 'module', source: `export default ${JSON.stringify(url)};`, shortCircuit: true };
    return next(url, context);
  },
});

const { default: HomeRenderer } = await import('../components/HomeRenderer.svelte');
const { default: HomePreview } = await import('../components/HomePreview.svelte');
const { default: HomeLayoutSettings } = await import('../components/HomeLayoutSettings.svelte');

function fixture() {
  const data = {
    heroSlides: [{ key: 'movie:one', eyebrow: 'FEATURED', title: 'A movie', chips: [], description: '', art: '', primaryLabel: 'Play', primary: () => {} }],
    heroSource: 'catalog', previewApp: null, stremioRecent: [], stremio: null, steamGames: [], activityItems: [], trackingEnabled: true,
    order: ['YouTube', 'Spotify', 'Custom'], launching: null, catalogVisible: true, youTubeFeed: [], previewMusic: [], steamRecommendations: [], steamRecommendationSource: null,
    previewActivity: [], musicLoading: false, popular: [], featured: [],
  };
  const actions = Object.fromEntries(['openStremio', 'openStremioPairing', 'previewStremio', 'openSteamGame', 'previewSteamGame', 'openItem', 'removeItem', 'previewActivityItem', 'launch', 'preview', 'openYouTubeItem', 'previewVideo', 'playMusic', 'previewMusicItem', 'openSteamStore', 'previewSteamRecommendation', 'select', 'previewMovie'].map(key => [key, () => {}]));
  return { rows: defaultHomeRows(), data, actions };
}

test('registered Home sections preserve order, the leading hero, and empty feedback', () => {
  const props = fixture();
  const html = render(HomeRenderer, { props }).body;
  const instances = [...html.matchAll(/data-home-instance="([^"]+)"/g)].map(match => match[1]);
  assert.deepEqual(instances, props.rows.map(row => row.id));
  assert.match(html, /A movie/);
  assert.doesNotMatch(html, /is-inset/);
  assert.match(html, /Popular movies/);
  assert.match(html, /Top-rated movies/);
  assert.match(html, /Nothing in progress/);
});

test('renamed instances retain component rendering and a moved hero becomes inset', () => {
  const props = fixture();
  props.rows = moveHomeRowTo(props.rows, 'apps', 0).map(row => row.component === 'apps' ? { ...row, id: 'my-dock' } : row);
  props.rows.find(row => row.component === 'popular').visible = false;
  const html = render(HomeRenderer, { props }).body;
  assert.ok(html.indexOf('data-home-instance="my-dock"') < html.indexOf('data-home-instance="hero"'));
  assert.match(html, /data-focus-key="Spotify"/);
  assert.match(html, /is-inset/);
  assert.doesNotMatch(html, /Popular movies/);
});

test('app previews preserve their content and suppress general catalog sections', () => {
  const props = fixture();
  props.data.previewApp = 'YouTube';
  props.data.youTubeFeed = [{ id: 'video-one', title: 'Recommended video' }];
  const html = render(HomeRenderer, { props }).body;
  assert.match(html, /Recommended video/);
  assert.doesNotMatch(html, /Popular movies|Top-rated movies/);
  props.data.previewApp = 'Stremio';
  props.data.youTubeFeed = [];
  assert.match(render(HomeRenderer, { props }).body, /Connect Stremio to continue watching/);
});

test('hidden sections and unavailable hero content do not leave interactive copies', () => {
  const props = fixture();
  props.data.heroSlides = [];
  props.rows = props.rows.map(row => ({ ...row, visible: false }));
  const html = render(HomeRenderer, { props }).body;
  assert.doesNotMatch(html, /data-home-instance|<button/);
});

test('the editor and preview resolve renamed instances through component metadata', () => {
  const props = fixture();
  const rows = props.rows.map(row => ({ ...row, id: `custom-${row.id}` }));
  const preview = { rows, popular: [], featured: [], activityItems: [], apps: [], widget: { enabled: false, placement: 'bar' }, highlight: 'custom-apps' };
  assert.match(render(HomePreview, { props: preview }).body, /Your apps/);
  const editor = render(HomeLayoutSettings, { props: { ...preview, onmove() {}, onmoveto() {}, ontoggle() {}, onreset() {} } }).body;
  assert.match(editor, /data-row="custom-apps"/);
  assert.match(editor, /aria-label="Move Your apps"/);
  assert.match(editor, /Show Featured hero on Home/);
});
