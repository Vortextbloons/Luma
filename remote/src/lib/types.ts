export type SoundPreset = "real" | "google" | "roku" | "off";

export type IconName =
  | "back"
  | "home"
  | "more"
  | "apps"
  | "keyboard"
  | "display"
  | "settings"
  | "up"
  | "down"
  | "left"
  | "right"
  | "previous"
  | "next"
  | "play"
  | "volume-down"
  | "volume-up"
  | "mute"
  | "stop"
  | "close"
  | "expand"
  | "phone"
  | "wifi-off"
  | "check"
  | "chevron-down";

export interface ThemeOption {
  id: string;
  name: string;
  tokens: Record<string, string>;
}

export interface Appearance {
  theme: string;
  tokens: Record<string, string>;
  themes: ThemeOption[];
}

export interface SettingOption {
  value: string;
  label: string;
}

export interface SettingChoice {
  key: string;
  group: string;
  label: string;
  description: string;
  options: SettingOption[];
  value: string;
}

export interface RemoteState {
  view: string;
  fullscreen: boolean;
  sound: SoundPreset;
  window: string | null;
  windowFullscreen: boolean | null;
  appearance?: Appearance;
  appOrder: string[];
  settings: SettingChoice[];
}

export interface StateResponse {
  pcName?: string;
  state?: RemoteState;
}

export interface CommandResponse {
  accepted?: boolean;
  error?: string | null;
  state?: RemoteState;
}

export interface SocketReply {
  id?: number;
  error?: string | null;
  state?: RemoteState;
  appearance?: Appearance;
  appOrder?: string[];
}

export interface AppEntry {
  command: string;
  name: string;
  icon: string;
}

export interface AppsResponse {
  apps?: AppEntry[];
}

export interface TimelineSnapshot {
  position?: number;
  duration?: number;
  error?: string;
}

export interface PlaybackResponse {
  timeline?: TimelineSnapshot | null;
  error?: string;
}
