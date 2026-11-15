import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { stripTypeScriptTypes } from 'node:module';

const source = stripTypeScriptTypes(readFileSync(new URL('./settingsNavigation.ts', import.meta.url), 'utf8'));
const { settingsCategories, resolveSettingsTarget } = await import(`data:text/javascript;base64,${Buffer.from(source).toString('base64')}`);

test('legacy appearance entries open the built-in appearance category', () => {
  assert.deepEqual(resolveSettingsTarget('Theme'), { category: 'Appearance & Home', section: 'theme' });
  assert.deepEqual(resolveSettingsTarget('Home layout'), { category: 'Appearance & Home', section: 'layout' });
  assert.deepEqual(resolveSettingsTarget('Layout'), { category: 'Appearance & Home', section: 'layout' });
  assert.deepEqual(resolveSettingsTarget('Widgets'), { category: 'Appearance & Home', section: 'widgets' });
  assert.deepEqual(resolveSettingsTarget('Appearance & Home'), { category: 'Appearance & Home', section: 'theme' });
  assert.deepEqual(resolveSettingsTarget('Plugins'), { category: 'Plugins', section: 'installed' });
});

test('appearance stays its own category and Plugins only lists installed plugins', () => {
  assert.equal(settingsCategories[0].name, 'Appearance & Home');
  assert.deepEqual(settingsCategories[0].sections.map(section => section.id), ['theme', 'layout', 'widgets']);
  const plugins = settingsCategories.find(category => category.name === 'Plugins');
  assert.deepEqual(plugins?.sections.map(section => section.id), ['installed']);
});

test('Plugins sits directly above MCP customization and MCP stays last', () => {
  const names = settingsCategories.map(category => category.name);
  assert.equal(names.at(-1), 'MCP customization');
  assert.equal(names.at(-2), 'Plugins');
  assert.equal(settingsCategories.at(-1)?.sections[0].id, 'ai');
});

test('unrelated settings entry points are preserved', () => {
  assert.deepEqual(resolveSettingsTarget('Display'), { category: 'Display & Sound', section: 'display' });
  assert.deepEqual(resolveSettingsTarget('Apps'), { category: 'Apps & Services', section: 'apps' });
  assert.deepEqual(resolveSettingsTarget('Remote & Startup'), { category: 'Remotes & Devices', section: 'remote' });
  assert.deepEqual(resolveSettingsTarget('Unknown entry'), { category: 'Appearance & Home', section: 'theme' });
});
