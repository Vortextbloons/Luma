export type SettingsSection = { id: string; name: string };
export type SettingsCategory = { name: string; sections: SettingsSection[] };

export const settingsCategories: SettingsCategory[] = [
  {
    name: 'Appearance',
    sections: [
      { id: 'theme', name: 'Theme' },
      { id: 'readability', name: 'Text & motion' },
      { id: 'screensaver', name: 'Screensaver' },
      { id: 'versions', name: 'Restore or reset' },
    ],
  },
  {
    name: 'Home',
    sections: [
      { id: 'layout', name: 'Home rows' },
      { id: 'order', name: 'App order' },
      { id: 'widgets', name: 'Widgets' },
      { id: 'providers', name: 'Services on Home' },
    ],
  },
  {
    name: 'Display & Sound',
    sections: [
      { id: 'display', name: 'Window' },
      { id: 'tv', name: 'TV & audio output' },
      { id: 'sound', name: 'Menu sounds' },
      { id: 'viewing', name: 'Multi Watch' },
    ],
  },
  {
    name: 'Apps & Accounts',
    sections: [
      { id: 'apps', name: 'Installed apps' },
      { id: 'accounts', name: 'Accounts' },
      { id: 'steam', name: 'Steam shortcuts' },
    ],
  },
  {
    name: 'Controls',
    sections: [
      { id: 'remote', name: 'Phone & Display Box' },
      { id: 'controller', name: 'Game controller' },
      { id: 'keyboard', name: 'Keyboard' },
    ],
  },
  {
    name: 'Watch History',
    sections: [{ id: 'recording', name: 'Watch history' }],
  },
  {
    name: 'System',
    sections: [
      { id: 'startup', name: 'Startup & background' },
      { id: 'updates', name: 'Updates' },
    ],
  },
  {
    name: 'Add-ons',
    sections: [
      { id: 'installed', name: 'Plugins' },
      { id: 'ai', name: 'AI access' },
    ],
  },
];

const legacyTargets: Record<string, string> = {
  'Appearance & Home': 'theme',
  Layout: 'layout',
  'Home layout': 'layout',
  Display: 'display',
  'TV setup': 'tv',
  Sound: 'sound',
  Activity: 'recording',
  'History & Privacy': 'recording',
  'Clear history': 'recording',
  clear: 'recording',
  Apps: 'apps',
  'Apps & Services': 'apps',
  Integrations: 'accounts',
  Performance: 'startup',
  'Remote & Startup': 'remote',
  'Remotes & Devices': 'remote',
  'MCP customization': 'ai',
  'Saved versions': 'versions',
  Reset: 'versions',
  reset: 'versions',
};

export function settingsCategory(name: string) {
  return settingsCategories.find(item => item.name === name) ?? settingsCategories[0];
}

export function resolveSettingsTarget(value: string) {
  const direct = settingsCategories.find(item => item.name === value);
  if (direct) return { category: direct.name, section: direct.sections[0].id };
  const target = Object.hasOwn(legacyTargets, value) ? legacyTargets[value] : value;
  for (const item of settingsCategories) {
    const match = item.sections.find(section => section.id === target || section.name === target);
    if (match) return { category: item.name, section: match.id };
  }
  return { category: settingsCategories[0].name, section: settingsCategories[0].sections[0].id };
}
