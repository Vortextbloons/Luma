import type { ThemeId } from '../types';

export const tokenNames = [
  '--bg', '--bg-soft', '--ambient', '--surface', '--surface-2', '--surface-3',
  '--line', '--line-strong', '--text', '--text-dim', '--muted', '--dim',
  '--accent', '--accent-strong', '--accent-ink', '--accent-soft',
  '--good', '--warn', '--danger', '--rating',
] as const;

export type ThemeToken = (typeof tokenNames)[number];
export type ThemeTokens = Record<ThemeToken, string>;

export interface ThemeDefinition {
  id: ThemeId;
  name: string;
  description: string;
  tokens: ThemeTokens;
}

export interface ThemeOption {
  id: ThemeId;
  name: string;
  tokens: ThemeTokens;
}

export interface Appearance {
  theme: ThemeId;
  tokens: ThemeTokens;
  themes: ThemeOption[];
}

const semantic: Pick<ThemeTokens, '--good' | '--warn' | '--danger' | '--rating'> = {
  '--good': '#7fe0ae',
  '--warn': '#ffd28d',
  '--danger': '#ffb4b4',
  '--rating': '#ffd98a',
};

const violetNight: ThemeTokens = {
  ...semantic,
  '--bg': '#0b0d12',
  '--bg-soft': '#111217',
  '--ambient': '#2a2438',
  '--surface': '#1d212a',
  '--surface-2': '#15171e',
  '--surface-3': '#242633',
  '--line': '#ffffff17',
  '--line-strong': '#ffffff26',
  '--text': '#f8f8fa',
  '--text-dim': '#d5d8e2',
  '--muted': '#a9aab5',
  '--dim': '#8f96a7',
  '--accent': '#a59bff',
  '--accent-strong': '#bdb4ff',
  '--accent-ink': '#151925',
  '--accent-soft': '#a59bff42',
};

const sapphire: ThemeTokens = {
  ...semantic,
  '--bg': '#080f18',
  '--bg-soft': '#0d1721',
  '--ambient': '#16324a',
  '--surface': '#15202d',
  '--surface-2': '#0f1822',
  '--surface-3': '#1c2a3a',
  '--line': '#ffffff17',
  '--line-strong': '#ffffff26',
  '--text': '#f6f9ff',
  '--text-dim': '#d3deef',
  '--muted': '#a5b2c6',
  '--dim': '#8b9bb2',
  '--accent': '#72bfff',
  '--accent-strong': '#9ed4ff',
  '--accent-ink': '#08111a',
  '--accent-soft': '#72bfff3d',
};

const mint: ThemeTokens = {
  ...semantic,
  '--bg': '#07110f',
  '--bg-soft': '#0b1815',
  '--ambient': '#163b31',
  '--surface': '#13221e',
  '--surface-2': '#0d1916',
  '--surface-3': '#1a2d27',
  '--line': '#ffffff17',
  '--line-strong': '#ffffff26',
  '--text': '#f4fbf8',
  '--text-dim': '#d0e6de',
  '--muted': '#a3b8b0',
  '--dim': '#8aa198',
  '--accent': '#88e6c5',
  '--accent-strong': '#b2f3dc',
  '--accent-ink': '#07130f',
  '--accent-soft': '#88e6c53d',
};

const ember: ThemeTokens = {
  ...semantic,
  '--bg': '#120d0b',
  '--bg-soft': '#1a1310',
  '--ambient': '#3d241b',
  '--surface': '#241b18',
  '--surface-2': '#1a1311',
  '--surface-3': '#30231f',
  '--line': '#ffffff17',
  '--line-strong': '#ffffff26',
  '--text': '#fdf8f5',
  '--text-dim': '#e8d8cf',
  '--muted': '#bba69c',
  '--dim': '#a18b81',
  '--accent': '#ffae92',
  '--accent-strong': '#ffc9b5',
  '--accent-ink': '#1a0e08',
  '--accent-soft': '#ffae923d',
};

const graphite: ThemeTokens = {
  ...semantic,
  '--bg': '#0c0d0f',
  '--bg-soft': '#121317',
  '--ambient': '#2a2c33',
  '--surface': '#1b1d22',
  '--surface-2': '#14161a',
  '--surface-3': '#25272d',
  '--line': '#ffffff17',
  '--line-strong': '#ffffff26',
  '--text': '#f7f7f9',
  '--text-dim': '#d8d9df',
  '--muted': '#aaaeb8',
  '--dim': '#8f939e',
  '--accent': '#d7dae2',
  '--accent-strong': '#eef0f5',
  '--accent-ink': '#15171c',
  '--accent-soft': '#d7dae23d',
};

const onyx: ThemeTokens = {
  ...semantic,
  '--bg': '#000000',
  '--bg-soft': '#000000',
  '--ambient': '#000000',
  '--surface': '#0a0a0a',
  '--surface-2': '#040404',
  '--surface-3': '#131313',
  '--line': '#ffffff16',
  '--line-strong': '#ffffff2b',
  '--text': '#f7f7f7',
  '--text-dim': '#cfcfcf',
  '--muted': '#9c9c9c',
  '--dim': '#828282',
  '--accent': '#ededed',
  '--accent-strong': '#ffffff',
  '--accent-ink': '#000000',
  '--accent-soft': '#ffffff0a',
};

const aurora: ThemeTokens = {
  ...semantic,
  '--bg': '#040b10',
  '--bg-soft': '#08141c',
  '--ambient': '#12384a',
  '--surface': '#101f28',
  '--surface-2': '#0a161d',
  '--surface-3': '#182c38',
  '--line': '#ffffff17',
  '--line-strong': '#ffffff26',
  '--text': '#f2fbfb',
  '--text-dim': '#cde5e8',
  '--muted': '#9db8be',
  '--dim': '#849ea4',
  '--accent': '#5ee8d4',
  '--accent-strong': '#9af6e8',
  '--accent-ink': '#041018',
  '--accent-soft': '#5ee8d43d',
};

const synthwave: ThemeTokens = {
  ...semantic,
  '--bg': '#0b0614',
  '--bg-soft': '#120a1f',
  '--ambient': '#3a1858',
  '--surface': '#1a1028',
  '--surface-2': '#12091c',
  '--surface-3': '#251538',
  '--line': '#ffffff17',
  '--line-strong': '#ffffff26',
  '--text': '#faf5ff',
  '--text-dim': '#e2d0f5',
  '--muted': '#b39ac9',
  '--dim': '#9a82b0',
  '--accent': '#ff5ec8',
  '--accent-strong': '#ff96de',
  '--accent-ink': '#0a0412',
  '--accent-soft': '#ff5ec83d',
};

const biolume: ThemeTokens = {
  ...semantic,
  '--bg': '#020908',
  '--bg-soft': '#051210',
  '--ambient': '#0a3d32',
  '--surface': '#0c1e1a',
  '--surface-2': '#071412',
  '--surface-3': '#122a24',
  '--line': '#ffffff17',
  '--line-strong': '#ffffff26',
  '--text': '#effcf8',
  '--text-dim': '#c8ebe0',
  '--muted': '#98b8ad',
  '--dim': '#7e9d93',
  '--accent': '#3dffc8',
  '--accent-strong': '#88ffd9',
  '--accent-ink': '#02100c',
  '--accent-soft': '#3dffc83d',
};

export const themes: ThemeDefinition[] = [
  { id: 'violet-night', name: 'Violet Night', description: 'Lavender glow on a midnight base.', tokens: violetNight },
  { id: 'sapphire', name: 'Sapphire', description: 'Cool blue light over deep navy.', tokens: sapphire },
  { id: 'mint', name: 'Mint', description: 'A soft jade atmosphere.', tokens: mint },
  { id: 'ember', name: 'Ember', description: 'Warm peach light over charcoal.', tokens: ember },
  { id: 'graphite', name: 'Graphite', description: 'Neutral, quiet, and distraction-free.', tokens: graphite },
  { id: 'onyx', name: 'Onyx', description: 'Pure black with no ambient glow.', tokens: onyx },
  { id: 'aurora', name: 'Aurora', description: 'Polar teal light across a frozen night sky.', tokens: aurora },
  { id: 'synthwave', name: 'Synthwave', description: 'Neon magenta over a retro violet dusk.', tokens: synthwave },
  { id: 'biolume', name: 'Biolume', description: 'Abyssal dark with a living green glow.', tokens: biolume },
];

export const defaultThemeId: ThemeId = 'violet-night';

const legacyAccents: Record<string, ThemeId> = {
  violet: 'violet-night',
  blue: 'sapphire',
  mint: 'mint',
  peach: 'ember',
};

export function isThemeId(value: unknown): value is ThemeId {
  return typeof value === 'string' && themes.some(theme => theme.id === value);
}

export function resolveThemeId(value: unknown, legacy?: unknown): ThemeId {
  if (isThemeId(value)) return value;
  if (typeof legacy === 'string' && legacyAccents[legacy]) return legacyAccents[legacy];
  return defaultThemeId;
}

export function themeById(id: ThemeId): ThemeDefinition {
  return themes.find(theme => theme.id === id) ?? themes[0];
}

export function applyTheme(tokens: ThemeTokens, root: HTMLElement = document.documentElement) {
  for (const name of tokenNames) root.style.setProperty(name, tokens[name]);
}

export function appearanceFor(theme: ThemeDefinition): Appearance {
  return {
    theme: theme.id,
    tokens: theme.tokens,
    themes: themes.map(({ id, name, tokens }) => ({ id, name, tokens })),
  };
}
