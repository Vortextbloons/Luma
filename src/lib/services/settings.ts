import type { Settings } from '../types';
import { themes } from './theme';

type ChoiceKey = 'theme' | 'density' | 'appTileShape' | 'interfaceSounds' | 'soundLevel';
export interface SettingChoice {
  key: ChoiceKey;
  group: string;
  label: string;
  description: string;
  options: { value: string; label: string }[];
}
export const settingChoices: SettingChoice[] = [
  { key: 'theme', group: 'Theme', label: 'App theme', description: 'The look of Luma on the TV and connected displays.', options: themes.map(theme => ({ value: theme.id, label: theme.name })) },
  { key: 'appTileShape', group: 'Display', label: 'App tile shape', description: 'Wide cards suit artwork; squares fit more apps.', options: [{ value: 'wide', label: 'Pills' }, { value: 'square', label: 'Squares' }] },
  { key: 'density', group: 'Display', label: 'Layout density', description: 'Compact fits more rows on screen.', options: [{ value: 'comfortable', label: 'Comfortable' }, { value: 'compact', label: 'Compact' }] },
  { key: 'interfaceSounds', group: 'Sound', label: 'Sound set', description: 'Feedback for focus, select, back, and app launch.', options: [{ value: 'real', label: 'Real hardware' }, { value: 'google', label: 'Google TV' }, { value: 'roku', label: 'Roku' }, { value: 'off', label: 'Off' }] },
  { key: 'soundLevel', group: 'Sound', label: 'Sound level', description: 'Standard is tuned for a quiet room.', options: [{ value: 'soft', label: 'Soft' }, { value: 'standard', label: 'Standard' }, { value: 'loud', label: 'Loud' }] },
];
export function remoteSettings(settings: Settings) {
  return settingChoices.map(choice => ({ ...choice, value: settings[choice.key] }));
}
export function applyRemoteSetting(settings: Settings, patch: { key: string; value: string }) {
  const choice = settingChoices.find(choice => choice.key === patch.key);
  if (!choice?.options.some(option => option.value === patch.value)) return;
  Object.assign(settings, { [choice.key]: patch.value });
}
