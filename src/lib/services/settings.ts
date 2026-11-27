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
  { key: 'appTileShape', group: 'Layout', section: 'layout', kind: 'choice', label: 'App tile shape', description: 'Wide cards suit artwork; squares fit more apps.', options: [{ value: 'wide', label: 'Pills' }, { value: 'square', label: 'Squares' }] },
  { key: 'density', group: 'Layout', section: 'layout', kind: 'choice', label: 'Layout density', description: 'Compact fits more rows on screen.', options: [{ value: 'comfortable', label: 'Comfortable' }, { value: 'compact', label: 'Compact' }] },
  { key: 'textSize', group: 'Layout', section: 'layout', kind: 'choice', label: 'Text size', description: 'Scales text and controls for easier reading from the sofa.', options: [{ value: 'standard', label: 'Standard' }, { value: 'large', label: 'Large' }, { value: 'larger', label: 'Extra large' }] },
  { key: 'reducedMotion', group: 'Layout', section: 'layout', kind: 'toggle', label: 'Reduce motion', description: 'Turns off screen transitions and animated height changes.', options: [{ value: 'true', label: 'On' }, { value: 'false', label: 'Off' }] },
  { key: 'smoothAppHover', group: 'Layout', section: 'layout', kind: 'toggle', label: 'Improve app hover', description: 'Waits briefly before switching app previews on hover, animates the active dock indicator, and fades preview tiles in.', options: [{ value: 'true', label: 'On' }, { value: 'false', label: 'Off' }] },
  { key: 'interfaceSounds', group: 'Sound', section: 'sound', kind: 'choice', label: 'Sound set', description: 'Feedback for focus, select, back, and app launch.', options: [{ value: 'real', label: 'Real hardware' }, { value: 'google', label: 'Google TV' }, { value: 'roku', label: 'Roku' }, { value: 'off', label: 'Off' }] },
  { key: 'soundLevel', group: 'Sound', section: 'sound', kind: 'choice', label: 'Sound level', description: 'Standard is tuned for a quiet room.', options: [{ value: 'soft', label: 'Soft' }, { value: 'standard', label: 'Standard' }, { value: 'loud', label: 'Loud' }] },
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
