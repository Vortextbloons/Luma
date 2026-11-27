import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { stripTypeScriptTypes } from 'node:module';

const source = stripTypeScriptTypes(readFileSync(new URL('./settingsNavigation.ts', import.meta.url), 'utf8'));
const { settingsCategories, resolveSettingsTarget } = await import(`data:text/javascript;base64,${Buffer.from(source).toString('base64')}`);

test('settings shortcuts resolve to their new pages', () => {
  const shortcuts = [
    ['Theme', 'Appearance', 'theme'],
    ['Home layout', 'Home', 'layout'],
    ['Widgets', 'Home', 'widgets'],
    ['Display', 'Display & Sound', 'display'],
    ['viewing', 'Display & Sound', 'viewing'],
    ['TV setup', 'Display & Sound', 'tv'],
    ['Apps', 'Apps & Accounts', 'apps'],
    ['Integrations', 'Apps & Accounts', 'accounts'],
    ['Activity', 'Watch History', 'recording'],
    ['Clear history', 'Watch History', 'recording'],
    ['Updates', 'System', 'updates'],
    ['Remote & Startup', 'Controls', 'remote'],
    ['Plugins', 'Add-ons', 'installed'],
    ['MCP customization', 'Add-ons', 'ai'],
    ['Saved versions', 'Appearance', 'versions'],
    ['Reset', 'Appearance', 'versions'],
  ];
  for (const [input, category, section] of shortcuts) {
    assert.deepEqual(resolveSettingsTarget(input), { category, section }, input);
  }
});

test('every settings page has an unambiguous route and unknown entries open Appearance', () => {
  for (const { name, sections } of settingsCategories) {
    assert.deepEqual(resolveSettingsTarget(name), { category: name, section: sections[0].id });
    for (const { id } of sections) {
      assert.deepEqual(resolveSettingsTarget(id), { category: name, section: id });
    }
  }
  assert.deepEqual(resolveSettingsTarget('Unknown entry'), { category: 'Appearance', section: 'theme' });
});
