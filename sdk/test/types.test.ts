import {
  bridge,
  createBridge,
  onCommand,
  onPlayback,
  onSettings,
  type CommandDelivery,
  type Identity,
  type NetworkRequest,
  type Permission,
  type PlaybackState,
  type SettingsSnapshot,
  type SettingValue,
} from "../index.js";

const permission: Permission = "settings.read";
const request: NetworkRequest = {
  origin: "https://api.open-meteo.com",
  path: "/v1/forecast",
  query: [["latitude", "47.6062"]],
};
const value: SettingValue = true;

const client = createBridge(async () => undefined);

void bridge.identity().then((identity: Identity) => identity.version);
void client.readSettings().then((settings: SettingsSnapshot) => settings.values.level);
void client.subscribeSettings().then((settings: SettingsSnapshot) => settings.values.level);
void client.get(request).then(payload => payload);
void client.readPlayback().then((playback: PlaybackState) => playback.playing);
void client.subscribePlayback().then((playback: PlaybackState) => playback.playing);
void client.togglePlayback().then(result => result.accepted);
void client.ready().then(result => result.ready);
void client.close().then(result => result.closed);
void client.pendingCommand().then((delivery: CommandDelivery | null) => delivery?.command);
void onCommand(delivery => delivery.command);
void onSettings(snapshot => snapshot.values);
void onPlayback(state => state.playing);
void permission;
void value;
