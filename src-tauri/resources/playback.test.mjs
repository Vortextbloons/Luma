import { readFileSync } from 'node:fs';
import vm from 'node:vm';
import test from 'node:test';
import assert from 'node:assert/strict';
const source = readFileSync(new URL('./playback.js', import.meta.url), 'utf8');
function player(video) { return vm.runInNewContext(`(${source})`, { document: { querySelector: () => video } }); }
test('remote toggles the video between pause and resume', async () => {
  const video = { paused: false, pause() { this.paused = true; }, async play() { this.paused = false; } };
  const control = player(video);
  await control('toggle'); assert.equal(video.paused, true);
  await control('toggle'); assert.equal(video.paused, false);
});
test('remote video skipping stays inside the playable duration', async () => {
  const video = { currentTime: 5, duration: 18 };
  const control = player(video);
  await control('previous'); assert.equal(video.currentTime, 0);
  await control('next'); assert.equal(video.currentTime, 10);
  await control('next'); assert.equal(video.currentTime, 18);
});
test('playback buttons tolerate a browsing screen without a video', async () => { await player(null)('toggle'); });
