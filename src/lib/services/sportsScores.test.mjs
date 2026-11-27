import assert from 'node:assert/strict';
import test from 'node:test';
import { badgeSideValue, indexMatchups, matchSportsBadge, parseScoreboard, scoreSummary, scoreTeam } from './sportsScores.ts';

const makeGame = (id = '1') => ({ id, date: '2026-11-15T18:00Z', competitions: [{
  status: { type: { state: 'in', shortDetail: '3rd 6:28' } }, venue: { fullName: 'Home arena' },
  competitors: [
    { team: { displayName: 'Philadelphia 76ers', abbreviation: 'PHI' }, score: '69' },
    { team: { displayName: 'Indiana Pacers', abbreviation: 'IND' }, score: '71' },
  ],
}] });
const event = { league: 'NBA', teams: [{ name: 'Indiana Pacers' }, { name: 'Philadelphia 76ers' }] };

test('scores match both teams independent of provider ordering', () => {
  const scores = parseScoreboard({ events: [makeGame()] }, 'NBA');
  const game = indexMatchups(scores)(event);
  assert.ok(game);
  assert.equal(scoreTeam(game, event.teams[0].name)?.score, '71');
  assert.equal(game.status, '3rd 6:28');
  assert.equal(game.venue, 'Home arena');
});

test('ambiguous doubleheaders and partial team matches never show the wrong score', () => {
  const scores = parseScoreboard({ events: [makeGame(), makeGame('2')] }, 'NBA');
  const lookup = indexMatchups(scores);
  assert.equal(lookup(event), undefined);
  assert.equal(lookup({ ...event, teams: [{ name: 'Indiana Pacers' }, { name: 'Boston Celtics' }] }), undefined);
  assert.equal(lookup({ ...event, league: 'NFL' }), undefined);
});

test('hide scores keeps spoiler information out of the playback header', () => {
  const game = parseScoreboard({ events: [makeGame()] }, 'NBA')[0];
  assert.equal(scoreSummary(game, true), 'Scores hidden');
  assert.match(scoreSummary(game, false), /PHI 69/);
  assert.match(scoreSummary(game, false), /3rd 6:28/);
});

test('known full city naming variants match while missing data stays empty', () => {
  const game = parseScoreboard({ events: [makeGame()] }, 'NBA')[0];
  game.teams[0].name = 'LA Clippers';
  assert.ok(indexMatchups([game])({ ...event, teams: [{ name: 'Los Angeles Clippers' }, { name: 'Indiana Pacers' }] }));
  assert.deepEqual(parseScoreboard({ events: [] }, 'NBA'), []);
  assert.throws(() => parseScoreboard({}, 'NBA'));
});

test('a scoreboard game maps back to its single stream event', () => {
  const game = parseScoreboard({ events: [makeGame()] }, 'NBA')[0];
  const streams = [
    { id: 'a', league: 'NBA', teams: [{ name: 'Indiana Pacers' }, { name: 'Philadelphia 76ers' }] },
    { id: 'b', league: 'NBA', teams: [{ name: 'Indiana Pacers' }, { name: 'Boston Celtics' }] },
  ];
  const lookup = indexMatchups(streams);
  assert.equal(lookup(game)?.id, 'a');
  assert.equal(lookup({ ...game, league: 'NFL' }), undefined);
  assert.equal(indexMatchups([...streams, { ...streams[0] }, { ...streams[0] }])(game), undefined);
});

test('incomplete matchups cannot supply scores or streams for a valid game', () => {
  const repeated = { ...event, teams: [event.teams[0], event.teams[0]] };
  const unnamed = { ...event, teams: [event.teams[0], { name: '' }] };
  const lookup = indexMatchups([repeated, unnamed, event]);
  assert.equal(lookup(repeated), undefined);
  assert.equal(lookup(unnamed), undefined);
  assert.equal(lookup(event), event);
});

test('matchSportsBadge matches live games across league aliases and partial names', () => {
  const badges = [
    { pluginId: 'kalshi', league: 'CFB', teams: ['Ole Miss', 'Mississippi St.'], label: 'Kalshi', sides: [{ team: 'Ole Miss', value: '75%', pct: 75 }, { team: 'Mississippi St.', value: '25%', pct: 25 }], updatedAt: 1000, showOnCards: true, showInPlayer: true },
    { pluginId: 'kalshi', league: 'NCAAB', teams: ['Iowa State', "St. John's"], label: 'Kalshi', sides: [{ team: 'Iowa State', value: '60%', pct: 60 }, { team: "St. John's", value: '40%', pct: 40 }], updatedAt: 1000, showOnCards: true, showInPlayer: true },
    { pluginId: 'kalshi', league: 'NHL', teams: ['Boston', 'Toronto'], label: 'Kalshi', sides: [{ team: 'Boston', value: '70%', pct: 70 }, { team: 'Toronto', value: '30%', pct: 30 }], updatedAt: 1000, showOnCards: true, showInPlayer: true },
  ];
  const cfbGame = { league: 'CFB', teams: [{ name: 'Ole Miss Rebels' }, { name: 'Mississippi State Bulldogs' }] };
  const matchedCfb = matchSportsBadge(cfbGame, badges);
  assert.ok(matchedCfb);
  assert.equal(badgeSideValue(matchedCfb, 'Ole Miss Rebels')?.value, '75%');
  assert.equal(badgeSideValue(matchedCfb, 'Mississippi State Bulldogs')?.value, '25%');

  const ncaaGame = { league: 'NCAA Men', teams: [{ name: 'Iowa State Cyclones' }, { name: "St. John's Red Storm" }] };
  const matchedNcaa = matchSportsBadge(ncaaGame, badges);
  assert.ok(matchedNcaa);
  assert.equal(badgeSideValue(matchedNcaa, 'Iowa State Cyclones')?.value, '60%');

  const nhlGame = { league: 'NHL', teams: [{ name: 'Boston Bruins' }, { name: 'Toronto Maple Leafs' }] };
  const matchedNhl = matchSportsBadge(nhlGame, badges);
  assert.ok(matchedNhl);
  assert.equal(badgeSideValue(matchedNhl, 'Boston Bruins')?.value, '70%');
});
