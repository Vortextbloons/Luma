import test from 'node:test';
import assert from 'node:assert/strict';
import { parseStreamEast, mergeSportsSources, streamEastUrl } from './sportsSources.ts';
const primary = (teams = ['Boston Celtics', 'Miami Heat']) => parseStreamEast({ matches: [{ id: 1, api_id: 'event-espn-league-basketball-nba-123', home: { name: teams[0] }, away: { name: teams[1] }, starts_at: '2026-11-15T20:00:00Z', live: true, streams: [{ url: 'https://streamec.st/embed/ch73.php' }] }] });
const fallback = (teams = ['Miami Heat', 'Boston Celtics'], id = 'surge') => ({ id, url: 'https://isportsurge.ws/watch/nba/'+id, league: 'NBA', teams: teams.map(name => ({ name, logo: '' })), title: teams.join(' vs '), time: 'In Progress', live: true });
test('both providers contribute games and matching teams share one ordered source plan', () => {
  const games = mergeSportsSources(primary(), [fallback(), fallback(['Phoenix Suns', 'Los Angeles Lakers'], 'second')]);
  assert.equal(games.length, 2);
  assert.equal(games[0].provider, 'StreamEast');
  assert.deepEqual(games[0].sources, ['https://streamec.st/embed/ch73.php', 'https://isportsurge.ws/watch/nba/surge']);
  assert.equal(games[1].id, 'second');
  assert.equal(mergeSportsSources(primary(), []).length, 1);
  assert.equal(mergeSportsSources([], [fallback()]).length, 1);
});
test('ambiguous games and different teams never borrow another game source', () => {
  const games = mergeSportsSources(primary(), [fallback(), fallback(undefined, 'doubleheader')]);
  assert.equal(games.length, 3);
  assert.equal(games[0].sources.length, 1);
  assert.equal(mergeSportsSources(primary(), [fallback(['Boston Celtics', 'Chicago Bulls'])]).length, 2);
});
test('source parser drops events without approved playback URLs', () => {
  for (const url of ['http://golygoals.com/streame.html?key=x', 'https://golygoals.com/advert', 'https://streamec.st/other.php', 'https://user@streamec.st/embed/ch73.php']) assert.equal(streamEastUrl(url), '');
  assert.equal(primary()[0].league, 'NBA');
  assert.equal(parseStreamEast({ matches: [{ id: 1, title: 'Game', streams: [{ url: 'https://example.com/' }] }] }).length, 0);
});
