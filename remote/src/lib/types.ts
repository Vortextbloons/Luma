export type SoundPreset = "real" | "google" | "roku" | "off";

export type IconName =
  | "back"
  | "home"
  | "more"
  | "search"
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
  | "pause"
  | "music"
  | "volume-down"
  | "volume-up"
  | "mute"
  | "stop"
  | "close"
  | "expand"
  | "collapse"
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
  section?: string;
  kind?: "choice" | "toggle" | string;
  label: string;
  description: string;
  options: SettingOption[];
  value: string;
}

export interface MusicState {
  playing: boolean;
  title?: string;
  artist?: string;
  hasArtwork: boolean;
  positionMs?: number;
  durationMs?: number;
  updatedAt: number;
  canToggle: boolean;
  canSkipNext: boolean;
  canSkipPrevious: boolean;
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
  music?: MusicState | null;
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
  music?: MusicState | null;
}

export interface AppEntry {
  command: string;
  name: string;
  icon: string;
}

export interface AppsResponse {
  apps?: AppEntry[];
}

export interface PanelActionResponse {
  accepted?: boolean;
  status?: "success" | "failure" | "pending";
  code?: string;
  error?: string | null;
  music?: MusicState | null;
}

export interface PanelsResponse {
  panels?: RemotePanel[];
}

export type PanelAction = "playback.toggle" | "playback.next" | "playback.previous";

export interface PanelCommand {
  id: string;
  label: string;
}

export interface RemotePanel {
  id: string;
  name: string;
  title: string;
  actions: PanelAction[];
  version?: string;
  commands: PanelCommand[];
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
