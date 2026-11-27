import type { Settings } from '../types';
import { defaultThemeId, themes } from './theme';

export type ChoiceKey = 'theme' | 'density' | 'appTileShape' | 'textSize' | 'reducedMotion' | 'smoothAppHover' | 'interfaceSounds' | 'soundLevel';
export type SettingKind = 'choice' | 'toggle';
export interface SettingChoice {
  key: ChoiceKey;
  group: string;
  section: string;
  kind: SettingKind;
  label: string;
  description: string;
  options: { value: string; label: string }[];
}
export const settingChoices: SettingChoice[] = [
  { key: 'theme', group: 'Appearance', section: 'theme', kind: 'choice', label: 'App theme', description: 'The look of Luma on the TV and connected displays.', options: themes.map(theme => ({ value: theme.id, label: theme.name })) },
  { key: 'appTileShape', group: 'Layout', section: 'layout', kind: 'choice', label: 'App tile shape', description: 'Choose wide or square app tiles.', options: [{ value: 'wide', label: 'Pills' }, { value: 'square', label: 'Squares' }] },
  { key: 'density', group: 'Layout', section: 'layout', kind: 'choice', label: 'Spacing', description: 'Compact fits more rows on screen.', options: [{ value: 'comfortable', label: 'Comfortable' }, { value: 'compact', label: 'Compact' }] },
  { key: 'textSize', group: 'Layout', section: 'layout', kind: 'choice', label: 'Text size', description: 'Make text and controls easier to read from the sofa.', options: [{ value: 'standard', label: 'Standard' }, { value: 'large', label: 'Large' }, { value: 'larger', label: 'Extra large' }] },
  { key: 'reducedMotion', group: 'Layout', section: 'layout', kind: 'toggle', label: 'Reduce motion', description: 'Use fewer animations.', options: [{ value: 'true', label: 'On' }, { value: 'false', label: 'Off' }] },
  { key: 'smoothAppHover', group: 'Layout', section: 'layout', kind: 'toggle', label: 'Smooth app previews', description: 'Fade between previews when pointing at apps.', options: [{ value: 'true', label: 'On' }, { value: 'false', label: 'Off' }] },
  { key: 'interfaceSounds', group: 'Sound', section: 'sound', kind: 'choice', label: 'Menu sounds', description: 'Play a sound when moving or selecting.', options: [{ value: 'real', label: 'Real hardware' }, { value: 'google', label: 'Google TV' }, { value: 'roku', label: 'Roku' }, { value: 'off', label: 'Off' }] },
  { key: 'soundLevel', group: 'Sound', section: 'sound', kind: 'choice', label: 'Menu volume', description: 'Standard is tuned for a quiet room.', options: [{ value: 'soft', label: 'Soft' }, { value: 'standard', label: 'Standard' }, { value: 'loud', label: 'Loud' }] },
];
const defaults: Record<ChoiceKey, string | boolean> = {
  theme: defaultThemeId,
  density: 'comfortable',
  appTileShape: 'wide',
  textSize: 'standard',
  reducedMotion: false,
  smoothAppHover: true,
  interfaceSounds: 'real',
  soundLevel: 'standard',
};
export function settingValue(settings: Settings, key: ChoiceKey) {
  const value = settings[key];
  return typeof value === 'boolean' ? String(value) : value;
}
export function applySettingValue(settings: Settings, key: string, value: string) {
  const choice = settingChoices.find(choice => choice.key === key);
  if (!choice?.options.some(option => option.value === value)) return false;
  Object.assign(settings, { [choice.key]: choice.kind === 'toggle' ? value === 'true' : value });
  return true;
}
export function resetSettingChoices(settings: Settings, keys: ChoiceKey[]) {
  for (const key of keys) Object.assign(settings, { [key]: defaults[key] });
}
