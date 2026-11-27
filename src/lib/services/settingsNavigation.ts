export type SettingsSection = { id: string; name: string; description: string };
export type SettingsCategory = { name: string; description: string; sections: SettingsSection[] };

export const settingsCategories: SettingsCategory[] = [
  {
    name: 'Appearance & Home',
    description: 'Theme, layout, widgets, and what appears on Home.',
    sections: [
      { id: 'theme', name: 'Theme', description: 'Backgrounds, surfaces, and accent together.' },
      { id: 'layout', name: 'Home layout', description: 'Density, tile shape, text size, motion, and the order of Home rows.' },
      { id: 'widgets', name: 'Widgets', description: 'Now playing and future widgets.' },
    ],
  },
  {
    name: 'Display & Sound',
    description: 'Fullscreen, TV screen, audio output, and interface sounds.',
    sections: [
      { id: 'display', name: 'Display', description: 'Fullscreen and screen fit.' },
      { id: 'tv', name: 'TV & audio', description: 'TV screen and sound output.' },
      { id: 'sound', name: 'Sound', description: 'Interface sounds and level.' },
    ],
  },
  {
    name: 'Apps & Services',
    description: 'Apps, locations, and connected accounts.',
    sections: [
      { id: 'apps', name: 'Apps', description: 'Find, arrange, and locate apps.' },
      { id: 'accounts', name: 'Accounts', description: 'Spotify, Stremio, and activity services.' },
    ],
  },
  {
    name: 'Remotes & Devices',
    description: 'Phone pairing, Display Box, controller binds, and the keyboard shortcut.',
    sections: [
      { id: 'remote', name: 'Remotes', description: 'Phone, Display Box, and connected devices.' },
      { id: 'controller', name: 'Controller', description: 'Gamepad navigation and custom button binds.' },
      { id: 'keyboard', name: 'Keyboard', description: 'Return-home shortcut.' },
    ],
  },
  {
    name: 'History & Privacy',
    description: 'Recording, provider visibility, and clearing history.',
    sections: [
      { id: 'recording', name: 'Recording', description: 'What Luma remembers and shows on Home.' },
      { id: 'clear', name: 'Clear history', description: 'Remove history from this PC.' },
    ],
  },
  {
    name: 'System',
    description: 'Startup, tray, memory, and updates.',
    sections: [
      { id: 'startup', name: 'Startup & memory', description: 'Startup, tray, and fast app switching.' },
      { id: 'updates', name: 'Updates', description: 'Version, downloads, and restart.' },
    ],
  },
  {
    name: 'Plugins',
    description: 'Import, export, and manage plugins.',
    sections: [
      { id: 'installed', name: 'Installed plugins', description: 'Import, enable, export, or remove plugins. New plugins start disabled.' },
    ],
  },
  {
    name: 'MCP customization',
    description: 'AI access, saved versions, and reset defaults.',
    sections: [
      { id: 'ai', name: 'AI access', description: 'Connect an MCP client with review-first access.' },
      { id: 'versions', name: 'Saved versions', description: 'Restore an earlier appearance, Home layout, or app arrangement.' },
      { id: 'reset', name: 'Reset', description: 'Return appearance, Home, and widgets to defaults.' },
    ],
  },
];

const legacyTargets: Record<string, [string, string]> = {
  Display: ['Display & Sound', 'display'],
  Theme: ['Appearance & Home', 'theme'],
  Layout: ['Appearance & Home', 'layout'],
  'Home layout': ['Appearance & Home', 'layout'],
  Widgets: ['Appearance & Home', 'widgets'],
  'TV setup': ['Display & Sound', 'tv'],
  Sound: ['Display & Sound', 'sound'],
  Activity: ['History & Privacy', 'recording'],
  Apps: ['Apps & Services', 'apps'],
  Integrations: ['Apps & Services', 'accounts'],
  Updates: ['System', 'updates'],
  Performance: ['System', 'startup'],
  'Remote & Startup': ['Remotes & Devices', 'remote'],
};

export function settingsCategory(name: string) {
  return settingsCategories.find(item => item.name === name) ?? settingsCategories[0];
}

export function resolveSettingsTarget(value: string) {
  const direct = settingsCategories.find(item => item.name === value);
  if (direct) return { category: direct.name, section: direct.sections[0].id };
  const legacy = legacyTargets[value];
  if (legacy) return { category: legacy[0], section: legacy[1] };
  for (const item of settingsCategories) {
    const match = item.sections.find(section => section.id === value || section.name === value);
    if (match) return { category: item.name, section: match.id };
  }
  return { category: settingsCategories[0].name, section: settingsCategories[0].sections[0].id };
}
