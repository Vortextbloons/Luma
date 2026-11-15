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

const timelineSource = readFileSync(new URL('./timeline.js', import.meta.url), 'utf8');
function timeline(video) {
  return vm.runInNewContext(`(${timelineSource})`, { document: { querySelectorAll: () => video ? [video] : [] } });
}
test('movie timeline previews position and clamps seeking to the available range', () => {
  const video = { currentTime: 30, duration: 120, getClientRects: () => [1], seekable: { length: 1, start: () => 5, end: () => 110 } };
  const read = timeline(video);
  assert.equal(read(null).position, 30);
  assert.equal(read(900).position, 110);
  assert.equal(read(0).position, 5);
  assert.equal(read(60).position, 60);
});
test('movie timeline is unavailable without a seekable finite video', () => {
  assert.equal(timeline(null)(null), null);
  assert.equal(timeline({ duration: Infinity, getClientRects: () => [1] })(null), null);
  assert.equal(timeline({ duration: 120, getClientRects: () => [1], seekable: { length: 0 } })(null), null);
});
