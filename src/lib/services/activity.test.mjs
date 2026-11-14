import assert from 'node:assert/strict';
import { test } from 'node:test';
import {
  actionLabel, formatDuration, isContinueWatching, isRecentlyPlayed, progressPercent,
} from './activitySelectors.ts';

const base = {
  id: 'plex:1', provider: 'plex', providerItemId: '1', profileId: null,
  type: 'movie', title: 'Example', subtitle: null, artwork: null,
  lastActivityAt: '2026-01-01T00:00:00.000Z', progressSeconds: 300,
  durationSeconds: 1000, progressSource: 'provider', completed: false,
  canResume: false, launchTarget: 'https://app.plex.tv/', resumeKind: 'content',
  launchCount: null,
};

test('progress is shown only for valid, verified positions', () => {
  assert.equal(progressPercent(base), 30);
  assert.equal(progressPercent({ ...base, progressSource: 'unknown' }), null);
  assert.equal(progressPercent({ ...base, progressSeconds: null }), null);
  assert.equal(progressPercent({ ...base, durationSeconds: 0 }), null);
  assert.equal(progressPercent({ ...base, progressSeconds: 1000 }), null);
  assert.equal(progressPercent({ ...base, progressSeconds: -1 }), null);
});

test('continue watching excludes finished items and recent playback keeps music', () => {
  assert.equal(isContinueWatching(base), true);
  assert.equal(isContinueWatching({ ...base, completed: true }), false);
  assert.equal(isRecentlyPlayed(base), false);
  assert.equal(isRecentlyPlayed({ ...base, progressSeconds: null, type: 'track', resumeKind: 'service' }), true);
  assert.equal(isRecentlyPlayed({ ...base, type: 'application', progressSeconds: null }), false);
});

test('action labels follow the provider capability instead of always claiming resume', () => {
  assert.equal(actionLabel({ ...base, resumeKind: 'direct' }), 'Resume');
  assert.equal(actionLabel({ ...base, resumeKind: 'content' }), 'Open');
  assert.equal(actionLabel({ ...base, type: 'application', resumeKind: 'service' }), 'Open app');
  assert.equal(actionLabel({ ...base, type: 'track', resumeKind: 'service' }), 'Open Spotify');
});

test('durations remain readable in card metadata', () => {
  assert.equal(formatDuration(90), '2m');
  assert.equal(formatDuration(5400), '1h 30m');
  assert.equal(formatDuration(0), '');
});
