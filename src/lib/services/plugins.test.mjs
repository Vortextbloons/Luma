import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { stripTypeScriptTypes } from 'node:module';

const source = stripTypeScriptTypes(readFileSync(new URL('./plugins.ts', import.meta.url), 'utf8').replace(/^import .*;\r?\n/gm, ''));
const { pluginControlSummary, pluginFieldLabel, pluginSettingValues } = await import(`data:text/javascript;base64,${Buffer.from(source).toString('base64')}`);

const record = {
  id: 'com.example.lounge',
  name: 'Lounge',
  controls: ['theme', 'homeColumns', 'home'],
  settings: [
    { key: 'theme', value: 'ember' },
    { key: 'showFeatured', value: false },
    { key: 'columns', value: 2 },
  ],
};

test('plugin control summaries name the appearance fields', () => {
  assert.equal(pluginFieldLabel('homeColumns'), 'Home columns');
  assert.equal(pluginFieldLabel('unknown'), 'unknown');
  assert.equal(pluginControlSummary(record), 'Controls theme, Home columns, and Home layout');
  assert.equal(pluginControlSummary({ ...record, controls: [] }), 'No appearance controls attached');
  assert.equal(pluginControlSummary({ ...record, controls: ['theme'] }), 'Controls theme');
});

test('plugin settings values can be round-tripped through the editor', () => {
  assert.deepEqual(pluginSettingValues(record), { theme: 'ember', showFeatured: false, columns: 2 });
});
