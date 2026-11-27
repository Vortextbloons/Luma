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
const { default: LayoutReview } = await import('../components/LayoutReview.svelte');

function fixture() {
  const data = {
    heroSlides: [{ key: 'movie:one', eyebrow: 'FEATURED', title: 'A movie', chips: [], description: '', art: '', primaryLabel: 'Play', primary: () => {} }],
    heroSource: 'catalog', previewApp: null, stremioRecent: [], stremio: null, steamGames: [], activityItems: [], trackingEnabled: true,
    order: ['YouTube', 'Spotify', 'Custom'], launching: null, catalogVisible: true, youTubeFeed: [], previewMusic: [], steamRecommendations: [], steamRecommendationSource: null,
    previewActivity: [], musicLoading: false, popular: [], featured: [], previewLeague: null, leagueGames: [], leagueGamesLoading: false,
    homeCards: [], pluginCardPending: null, pluginCardFailure: null,
  };
  const actions = Object.fromEntries(['openStremio', 'openStremioPairing', 'previewStremio', 'openSteamGame', 'previewSteamGame', 'openItem', 'removeItem', 'previewActivityItem', 'launch', 'preview', 'watchLeagueGame', 'previewLeagueGame', 'openYouTubeItem', 'previewVideo', 'playMusic', 'previewMusicItem', 'openSteamStore', 'previewSteamRecommendation', 'select', 'previewMovie', 'openPluginCard', 'retryPluginCard'].map(key => [key, () => {}]));
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
  assert.doesNotMatch(html, /data-home-instance="popular"|data-home-instance="featured"/);
  props.rows = moveHomeRowTo(props.rows, 'popular', 0);
  assert.match(render(HomeRenderer, { props }).body, /is-inset/);
  props.data.previewApp = 'Stremio';
  props.data.youTubeFeed = [];
  assert.match(render(HomeRenderer, { props }).body, /Connect Stremio to continue watching/);
});

test('sports app previews show the league games instead of unrelated Continue Watching', () => {
  const props = fixture();
  props.data.previewApp = 'NBA';
  props.data.previewLeague = 'NBA';
  props.data.leagueGames = [{
    id: '401', league: 'NBA', leagueLogo: '', date: '2026-11-15T00:00Z', state: 'in', status: '4th · 2:31', venue: 'Garden',
    teams: [
      { name: 'Boston Celtics', abbreviation: 'BOS', logo: '', score: '102', record: '9-2', color: '#007a33' },
      { name: 'Los Angeles Lakers', abbreviation: 'LAL', logo: '', score: '98', record: '7-4', color: '#552583' },
    ],
  }];
  const html = render(HomeRenderer, { props }).body;
  assert.match(html, /Live on NBA/);
  assert.match(html, /Boston Celtics/);
  assert.match(html, /● LIVE/);
  assert.doesNotMatch(html, /Nothing in progress|No recommendations available yet/);
  props.data.leagueGames = [];
  props.data.leagueGamesLoading = true;
  assert.match(render(HomeRenderer, { props }).body, /Looking up NBA games/);
  props.data.previewApp = 'MLB';
  props.data.previewLeague = 'MLB';
  props.data.leagueGamesLoading = false;
  assert.match(render(HomeRenderer, { props }).body, /No MLB games listed right now/);
  assert.doesNotMatch(render(HomeRenderer, { props }).body, /Boston Celtics|Nothing in progress/);
});

test('plugin cards use a stable host section with pending and retry feedback', () => {
  const props = fixture();
  const card = { id: 'com.example.weather:forecast', pluginId: 'com.example.weather', title: 'Weather', commandId: 'openForecast' };
  props.data.homeCards = [card];
  let html = render(HomeRenderer, { props }).body;
  assert.ok(html.indexOf('data-home-instance="recentApps"') < html.indexOf('data-home-instance="plugins"'));
  assert.match(html, /From your plugins/);
  assert.match(html, /data-plugin-card="com.example.weather:forecast"/);
  assert.match(html, /data-focus-key="com.example.weather:forecast"/);
  assert.match(html, />Weather</);

  props.data.pluginCardPending = card.id;
  html = render(HomeRenderer, { props }).body;
  assert.match(html, /aria-busy="true"/);
  assert.match(html, /Opening Weather…/);

  props.data.pluginCardPending = null;
  props.data.pluginCardFailure = { card, message: 'Weather did not start in time. Try again.' };
  html = render(HomeRenderer, { props }).body;
  assert.match(html, /role="alert"/);
  assert.match(html, /Weather couldn't open\. Weather did not start in time\. Try again\./);
  assert.match(html, />Retry</);

  props.data.homeCards = [];
  html = render(HomeRenderer, { props }).body;
  assert.doesNotMatch(html, /data-home-instance="plugins"|data-plugin-card|From your plugins/);
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
  const preview = { rows, popular: [], featured: [], activityItems: [], apps: [], widgets: {}, highlight: 'custom-apps' };
  assert.match(render(HomePreview, { props: preview }).body, /Your apps/);
  const editor = render(HomeLayoutSettings, { props: { ...preview, onmove() {}, onmoveto() {}, ontoggle() {}, onreset() {} } }).body;
  assert.match(editor, /data-row="custom-apps"/);
  assert.match(editor, /aria-label="Move Your apps"/);
  assert.match(editor, /Show Featured hero on Home/);
});

test('two-column rendering and miniature preview preserve section widths and the leading hero', () => {
  const props = fixture();
  props.columns = 2;
  props.rows.find(row => row.component === 'apps').span = 1;
  const html = render(HomeRenderer, { props }).body;
  assert.match(html, /class="resizing-content[^"]* columns"/);
  assert.match(html, /data-home-instance="apps" data-span="1"/);
  assert.match(html, /data-home-instance="popular" data-span="2"/);
  assert.ok(html.indexOf('data-home-instance="hero"') < html.indexOf('resizing-content'));
  const preview = render(HomePreview, { props: { rows: props.rows, columns: 2, popular: [], featured: [], activityItems: [], apps: [], widgets: {} } }).body;
  assert.match(preview, /grid-column: span 1;/);
  assert.match(preview, /columns/);
});

test('preview recovery stays available while saving or when Keep fails', () => {
  const html = render(LayoutReview, { props: { busy: false, editing: true, keep() {}, cancel() {}, error: 'Disk is full' } }).body;
  assert.match(html, /Keep layout/);
  assert.match(html, /Back to editor/);
  assert.match(html, /role="alert"[^>]*>Disk is full/);
  const busy = render(LayoutReview, { props: { busy: true, editing: false, keep() {}, cancel() {} } }).body;
  assert.equal([...busy.matchAll(/<button[^>]*disabled/g)].length, 2);
  assert.match(busy, /Cancel preview/);
});
