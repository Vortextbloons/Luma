import test from 'node:test';
import assert from 'node:assert/strict';
import { showPoster } from './artwork.ts';

test('show artwork deduplicates requests, limits concurrency, and caches missing IDs', async t => {
  const requests = [];
  t.mock.method(globalThis, 'fetch', url => new Promise(resolve => requests.push({ url, resolve })));
  assert.equal(await showPoster('not-an-imdb-id'), null);
  assert.equal(requests.length, 0);
  const first = showPoster('tt1000001');
  assert.equal(showPoster('tt1000001'), first);
  const rest = [2, 3, 4, 5, 6].map(id => showPoster(`tt100000${id}`));
  assert.equal(requests.length, 4);
  requests[0].resolve(new Response(JSON.stringify({ image: { medium: 'https://static.tvmaze.com/poster.jpg', original: 'https://static.tvmaze.com/large.jpg' } })));
  assert.equal(await first, 'https://static.tvmaze.com/poster.jpg');
  assert.equal(requests.length, 5);
  for (const request of requests.slice(1)) request.resolve(new Response('', { status: 404 }));
  await new Promise(resolve => setImmediate(resolve));
  requests[5].resolve(new Response('', { status: 404 }));
  assert.deepEqual(await Promise.all(rest), [null, null, null, null, null]);
  assert.equal(await showPoster('tt1000002'), null);
  assert.equal(await showPoster('tt1000001'), 'https://static.tvmaze.com/poster.jpg');
  assert.equal(requests.length, 6);
});
