import assert from 'node:assert/strict';
import test from 'node:test';
import { matchEvent, matchScore, parseScoreboard, scoreSummary, scoreTeam } from './sportsScores.ts';

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
  const game = matchScore(event, scores);
  assert.ok(game);
  assert.equal(scoreTeam(game, event.teams[0].name)?.score, '71');
  assert.equal(game.status, '3rd 6:28');
  assert.equal(game.venue, 'Home arena');
});

test('ambiguous doubleheaders and partial team matches never show the wrong score', () => {
  const scores = parseScoreboard({ events: [makeGame(), makeGame('2')] }, 'NBA');
  assert.equal(matchScore(event, scores), undefined);
  assert.equal(matchScore({ ...event, teams: [{ name: 'Indiana Pacers' }, { name: 'Boston Celtics' }] }, scores), undefined);
  assert.equal(matchScore({ ...event, league: 'NFL' }, scores), undefined);
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
  assert.ok(matchScore({ ...event, teams: [{ name: 'Los Angeles Clippers' }, { name: 'Indiana Pacers' }] }, [game]));
  assert.deepEqual(parseScoreboard({ events: [] }, 'NBA'), []);
  assert.throws(() => parseScoreboard({}, 'NBA'));
});

test('a scoreboard game maps back to its single stream event', () => {
  const game = parseScoreboard({ events: [makeGame()] }, 'NBA')[0];
  const streams = [
    { id: 'a', league: 'NBA', teams: [{ name: 'Indiana Pacers' }, { name: 'Philadelphia 76ers' }] },
    { id: 'b', league: 'NBA', teams: [{ name: 'Indiana Pacers' }, { name: 'Boston Celtics' }] },
  ];
  assert.equal(matchEvent(game, streams)?.id, 'a');
  assert.equal(matchEvent({ ...game, league: 'NFL' }, streams), undefined);
  assert.equal(matchEvent(game, [...streams, { ...streams[0] }]), undefined);
});
