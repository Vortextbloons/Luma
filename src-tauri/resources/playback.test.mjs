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
  return vm.runInNewContext(`(${timelineSource})`, { document: { querySelectorAll: selector => selector === 'video' && video ? [video] : [] } });
}
test('movie timeline previews position and clamps seeking to movie duration', () => {
  const video = { currentTime: 30, duration: 120, getClientRects: () => [1], seekable: { length: 1, start: () => 5, end: () => 110 } };
  const read = timeline(video);
  assert.equal(read(null).position, 30);
  assert.equal(read(900).position, 120);
  assert.equal(read(0).position, 0);
  assert.equal(read(60).position, 60);
});
test('movie timeline is unavailable without a finite video', () => {
  assert.equal(timeline(null)(null), null);
  assert.equal(timeline({ duration: Infinity, getClientRects: () => [1] })(null), null);
  assert.equal(timeline({ duration: 120, currentTime: 0, getClientRects: () => [1], seekable: { length: 0 } })(60).position, 60);
});

test('movie seeking can jump beyond the currently reported media range', () => {
  const video = { currentTime: 10, duration: 7200, getClientRects: () => [1], seekable: { length: 1, start: () => 0, end: () => 40 } };
  assert.equal(timeline(video)(3600).position, 3600);
});

test('movie timeline finds players inside shadow roots and embedded documents', () => {
  for (const kind of ['shadowRoot', 'contentDocument']) {
    const video = { currentTime: 30, duration: 120, getClientRects: () => [1] };
    const nested = { querySelectorAll: selector => selector === 'video' ? [video] : [] };
    const container = { tagName: kind === 'contentDocument' ? 'IFRAME' : 'DIV', [kind]: nested };
    const document = { querySelectorAll: selector => selector === '*' ? [container] : [] };
    const read = vm.runInNewContext(`(${timelineSource})`, { document });
    assert.equal(read(90).position, 90);
  }
});
test('movie timeline chooses visible playback instead of hidden preview video', () => {
  const preview = { currentTime: 0, duration: 30, paused: false, getClientRects: () => [] };
  const movie = { currentTime: 20, duration: 7200, paused: false, getClientRects: () => [1] };
  const document = { querySelectorAll: selector => selector === 'video' ? [preview, movie] : [] };
  const read = vm.runInNewContext(`(${timelineSource})`, { document });
  assert.equal(read(3600).position, 3600);
  assert.equal(preview.currentTime, 0);
});
test('movie timeline returns a useful error when the player rejects seeking', () => {
  const video = { duration: 120, getClientRects: () => [1], set currentTime(value) { throw new Error('rejected'); } };
  assert.match(timeline(video)(90).error, /could not change playback position/);
});
