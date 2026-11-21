# Luma plugin SDK

The SDK is the typed client for the existing `extension_bridge` command used by API v2 and v3 `isolated-ui` and `background` packages. The Rust bridge in `src-tauri/src/extension_spike.rs` remains authoritative for authorization, limits, and error responses.

The runtime package is plain ES module JavaScript with TypeScript declarations. JavaScript packages can import the same functions; no build step is required for the SDK itself.

## Operations and permissions

| SDK call | Bridge operation | Required permission |
| --- | --- | --- |
| `identity()` | `identity` | none |
| `readSettings()` | `settings.read` | `settings.read` |
| `subscribeSettings()` | `settings.subscribe` | `settings.read` |
| `get(request)` | `net.get` | `network.get` |
| `readPlayback()` | `playback.read` | `playback.read` |
| `subscribePlayback()` | `playback.subscribe` | `playback.read` |
| `togglePlayback()` | `playback.toggle` | `playback.control` |
| `nextTrack()` | `playback.next` | `playback.control` |
| `previousTrack()` | `playback.previous` | `playback.control` |
| `ready()` | `surface.ready` | none |
| `close()` | `surface.close` | none |
| `pendingCommand()` | `commands.pending` | none, API v3 only |

`apiVersion: 1` declarative packages do not execute and cannot use the bridge. API v3 command delivery is available only to enabled `isolated-ui` packages selected from a Home card or an enabled paired-phone panel.

## Usage

```ts
import { bridge, isBridgeError } from "@luma/plugin-sdk";

const identity = await bridge.identity();
const settings = await bridge.readSettings();
const forecast = await bridge.get({
  origin: "https://api.open-meteo.com",
  path: "/v1/forecast",
  query: [
    ["latitude", "47.6062"],
    ["longitude", "-122.3321"],
  ],
});
await bridge.ready();
```

Plain JavaScript uses the same module path when the package is bundled. Unbundled packages can call the bridge command directly with the protocol below.

```js
import { bridge } from "@luma/plugin-sdk";

const settings = await bridge.readSettings();
```

The protocol is also usable without the SDK. Every operation sends one invoke to the bridge command:

```js
window.__TAURI_INTERNALS__.invoke("extension_bridge", {
  op: "settings.read",
});
```

Operations with arguments add an `args` object:

```js
window.__TAURI_INTERNALS__.invoke("extension_bridge", {
  op: "net.get",
  args: {
    origin: "https://api.open-meteo.com",
    path: "/v1/forecast",
    query: [["latitude", "47.6062"]],
  },
});
```

## Results

- `identity()` returns `{ id, name, version, apiVersion, runtime, surface, origin }`.
- `readSettings()` returns `{ values }` containing only this package's resolved settings.
- `subscribeSettings()` registers for this package's settings changes and returns the same `{ values }` snapshot.
- `get()` returns the parsed JSON response, bounded by the host network bridge.
- `readPlayback()` returns `{ connected, title, artist, playing }`.
- `subscribePlayback()` registers for playback changes and returns the same playback snapshot.
- Playback commands return `{ accepted: true }`.
- `ready()` returns `{ ready: true }` and marks the surface interactive.
- `close()` returns `{ closed: true }` and asks the host to close the surface.
- `pendingCommand()` returns `{ command }` or `null` for API v3 Home command delivery.

## Commands

API v3 packages declare Home cards and commands in the manifest. When the user selects a card, the host starts the isolated surface, waits for `ready()`, then delivers one command. The bridge injects a `commands.pending` pull and dispatches the event:

```ts
import { onCommand, type CommandDelivery } from "@luma/plugin-sdk";

const stop = onCommand(({ command }: CommandDelivery) => {
  if (command === "openForecast") {
    // Show the forecast surface.
  }
});
```

The event name is `luma:command` and its `detail` is `{ command }`. `onCommand()` returns an unsubscribe function. Register the listener before calling `ready()`. The host sends one pending command per dispatch; a stale or revoked session rejects `commands.pending` instead of delivering it.

API v3 packages can also declare a paired-phone panel with `remote.commands`. The remote sends the version of the panel it displayed, and Luma rejects actions from replaced versions. Register `onCommand()` before `ready()` so the same handler can run in a visible surface or a hidden session. The action response carries `status: "pending" | "success" | "failure"`. A pending response means another command is still starting.

## Settings and playback notifications

`settings.subscribe` and `playback.subscribe` are available to active API v2 and v3 sessions that hold the corresponding read permission. They never start an idle plugin. Register the event listener first, then subscribe:

```ts
import { bridge, onSettings, onPlayback } from "@luma/plugin-sdk";

const stopSettings = onSettings(({ values }) => renderSettings(values));
const initialSettings = await bridge.subscribeSettings();

const stopPlayback = onPlayback(state => renderPlayback(state));
const initialPlayback = await bridge.subscribePlayback();
```

Contract:

- `subscribeSettings()` resolves with `{ values }`, the resolved values at subscription time. Every later host change that alters those values emits `luma:settings` with the same `{ values }` shape.
- `subscribePlayback()` resolves with `{ connected, title, artist, playing }`, the playback state at subscription time. Later changes emit `luma:playback` with the same shape.
- Events are whole snapshots. If an event arrives before the subscribe call resolves, keep the event value and skip rendering the initial return value.
- The host suppresses an event when the new snapshot is byte-for-byte identical to the last value delivered to that session. Re-subscribing returns the current snapshot and does not replay history.
- Delivery follows host changes. Values committed before subscription are in the initial snapshot. Changes after registration are pushed and can arrive before the subscribe call resolves.
- There is no unsubscribe operation. Disposing the session, disabling or updating the plugin, revoking the grant, or stopping Luma ends delivery. Event listeners in the page are released when the surface closes.
- Missing or revoked read permission rejects the subscribe call with `denied`; a stopped session rejects with `revoked` or `stale`.

## Errors

Host rejections become `LumaBridgeError` with a `code` and `message`. `toBridgeError()` normalizes object, string, and thrown errors. Unknown host codes use `unknown` while preserving the message.

Known codes: `invalid`, `denied`, `busy`, `stale`, `revoked`, `cancelled`, `timeout`, `too_large`, `malformed`, and `unavailable`.

```ts
try {
  await bridge.togglePlayback();
} catch (error) {
  if (isBridgeError(error) && error.code === "busy") {
    // Keep the previous control state and let the user retry.
  }
}
```

## Editor schema

`schemas/luma-extension.schema.json` provides completion and early diagnostics for `luma-extension.json`. It follows the current Rust manifest shape:

- Required fields, supported API versions, runtime kinds, and unknown-field rejection.
- Setting defaults, option limits, and bound contribution values.
- Executable permissions, network origins, and v1/v2/v3 cross-field rules.
- API v3 `minHostVersion`, Home cards, command declarations, and paired-phone command panels.

The schema is for editors. It cannot prove that declared files exist or enforce archive limits. The CLI, Settings import, and MCP staging must use the Rust validators for those checks.

Do not add a `$schema` property to the manifest. The native manifest rejects unknown fields. Associate the schema from editor settings instead, as the starter does with `.vscode/settings.json`.

## Starter

`starters/typescript` is an API v2 `isolated-ui` package. It imports the SDK, builds `dist/`, and includes schema completion plus a VS Code launch configuration.

The launch configuration expects the CLI command `luma-plugin dev` from the plugin developer platform milestone. The CLI owns the development host, reload loop, and session diagnostics. The starter consumes only its built package directory:

```powershell
cd starters\typescript
npm install
npm run build
luma-plugin validate dist
luma-plugin pack dist
```

`dist/` contains `luma-extension.json` and the declared `ui/` assets. Point the CLI development host at that directory. When scaffolding outside this checkout, keep `@luma/plugin-sdk` available to the project or vendor the runtime file, and copy or rewrite the editor schema path.

## Checks

```powershell
npm --prefix sdk install
npm --prefix sdk test
npm --prefix sdk run typecheck
```

The tests exercise the editor schema against the existing example manifests and the starter, compare SDK permissions, bridge operations, manifest limits, and enumerations with the Rust contracts, and cover the bridge protocol, command delivery, subscription events, and error normalization.
