export const BRIDGE_COMMAND: "extension_bridge";
export const COMMAND_EVENT: "luma:command";
export const SETTINGS_EVENT: "luma:settings";
export const PLAYBACK_EVENT: "luma:playback";

export type Permission =
  | "settings.read"
  | "network.get"
  | "playback.read"
  | "playback.control";

export const PERMISSIONS: readonly Permission[];

export type BridgeOperation =
  | "identity"
  | "settings.read"
  | "settings.subscribe"
  | "net.get"
  | "playback.read"
  | "playback.subscribe"
  | "playback.toggle"
  | "playback.next"
  | "playback.previous"
  | "surface.ready"
  | "surface.close"
  | "commands.pending"
  | "development.report";

export const BRIDGE_OPERATIONS: readonly BridgeOperation[];

export const OPERATION_PERMISSIONS: Readonly<Record<BridgeOperation, Permission | null>>;

export type HostBridgeErrorCode =
  | "invalid"
  | "denied"
  | "busy"
  | "stale"
  | "revoked"
  | "cancelled"
  | "timeout"
  | "too_large"
  | "malformed"
  | "unavailable";

export type BridgeErrorCode = HostBridgeErrorCode | "unknown";

export const BRIDGE_ERROR_CODES: readonly HostBridgeErrorCode[];

export interface BridgeErrorPayload {
  code: HostBridgeErrorCode;
  message: string;
}

export type JsonPrimitive = null | boolean | number | string;
export type JsonValue = JsonPrimitive | JsonValue[] | { [key: string]: JsonValue };

export type SettingValue = null | boolean | number | string;

export interface Identity {
  id: string;
  name: string;
  version: string;
  apiVersion: 1 | 2 | 3;
  runtime: "declarative" | "isolated-ui" | "background" | "main-page";
  surface: "desktop" | null;
  origin: string;
}

export interface SettingsSnapshot {
  values: Record<string, SettingValue>;
}

export interface NetworkRequest {
  origin: string;
  path: string;
  query?: ReadonlyArray<readonly [string, string]>;
}

export interface PlaybackState {
  connected: boolean;
  title: string | null;
  artist: string | null;
  playing: boolean;
}

export interface PlaybackCommandResult {
  accepted: true;
}

export interface SurfaceReadyResult {
  ready: true;
}

export interface SurfaceClosedResult {
  closed: true;
}

export interface CommandDelivery {
  command: string;
  invocationId?: string;
}

export interface DevelopmentOutcome {
  invocationId: string;
  success: boolean;
  message?: string;
}

export type CommandListener = (delivery: CommandDelivery) => void;
export type SettingsListener = (snapshot: SettingsSnapshot) => void;
export type PlaybackListener = (state: PlaybackState) => void;

export type BridgeInvoke = (
  command: string,
  args?: Record<string, unknown>,
) => Promise<unknown>;

export interface LumaBridge {
  request<T = JsonValue>(op: BridgeOperation, args?: JsonValue): Promise<T>;
  identity(): Promise<Identity>;
  readSettings(): Promise<SettingsSnapshot>;
  subscribeSettings(): Promise<SettingsSnapshot>;
  get(args: NetworkRequest): Promise<JsonValue>;
  readPlayback(): Promise<PlaybackState>;
  subscribePlayback(): Promise<PlaybackState>;
  togglePlayback(): Promise<PlaybackCommandResult>;
  nextTrack(): Promise<PlaybackCommandResult>;
  previousTrack(): Promise<PlaybackCommandResult>;
  ready(): Promise<SurfaceReadyResult>;
  close(): Promise<SurfaceClosedResult>;
  pendingCommand(): Promise<CommandDelivery | null>;
  reportDevelopmentOutcome(outcome: DevelopmentOutcome): Promise<{ accepted: true }>;
}

export class LumaBridgeError extends Error {
  readonly code: BridgeErrorCode;
  constructor(code: BridgeErrorCode, message: string, cause?: unknown);
}

export function isBridgeError(value: unknown): value is LumaBridgeError;

export function toBridgeError(cause: unknown): LumaBridgeError;

export function onCommand(listener: CommandListener): () => void;

export function onSettings(listener: SettingsListener): () => void;

export function onPlayback(listener: PlaybackListener): () => void;

export function createBridge(invoke?: BridgeInvoke): LumaBridge;

export const bridge: LumaBridge;
