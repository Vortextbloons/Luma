import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { stripTypeScriptTypes } from 'node:module';

const source = stripTypeScriptTypes(readFileSync(new URL('./settingsNavigation.ts', import.meta.url), 'utf8'));
const { settingsCategories, resolveSettingsTarget } = await import(`data:text/javascript;base64,${Buffer.from(source).toString('base64')}`);

test('legacy appearance entries open the built-in plugin inside Plugins', () => {
  assert.deepEqual(resolveSettingsTarget('Theme'), { category: 'Plugins', section: 'appearance' });
  assert.deepEqual(resolveSettingsTarget('Home layout'), { category: 'Plugins', section: 'layout' });
  assert.deepEqual(resolveSettingsTarget('Layout'), { category: 'Plugins', section: 'layout' });
  assert.deepEqual(resolveSettingsTarget('Widgets'), { category: 'Plugins', section: 'widgets' });
  assert.deepEqual(resolveSettingsTarget('Appearance & Home'), { category: 'Plugins', section: 'appearance' });
  assert.deepEqual(resolveSettingsTarget('Plugins'), { category: 'Plugins', section: 'installed' });
});

test('plugins category owns appearance sections while MCP customization stays last', () => {
  assert.equal(settingsCategories[0].name, 'Plugins');
  assert.deepEqual(settingsCategories[0].sections.map(section => section.id), ['installed', 'appearance', 'layout', 'widgets']);
  assert.equal(settingsCategories.at(-1).name, 'MCP customization');
  assert.equal(settingsCategories.at(-1).sections[0].id, 'ai');
  assert.ok(!settingsCategories.some(category => category.name === 'Appearance & Home'));
});

test('unrelated settings entry points are preserved', () => {
  assert.deepEqual(resolveSettingsTarget('Display'), { category: 'Display & Sound', section: 'display' });
  assert.deepEqual(resolveSettingsTarget('Apps'), { category: 'Apps & Services', section: 'apps' });
  assert.deepEqual(resolveSettingsTarget('Remote & Startup'), { category: 'Remotes & Devices', section: 'remote' });
  assert.deepEqual(resolveSettingsTarget('Unknown entry'), { category: 'Plugins', section: 'installed' });
});
