# @luma/plugin-sdk

Typed client for Luma's `extension_bridge` protocol. The package ships as plain ES module JavaScript with TypeScript declarations. Bundle it into the package entry, or call the bridge command directly from unbundled JavaScript.

## Operations

| Call | Permission |
| --- | --- |
| `identity()` | none |
| `readSettings()` | `settings.read` |
| `subscribeSettings()` | `settings.read` |
| `get({ origin, path, query })` | `network.get` |
| `readPlayback()` | `playback.read` |
| `subscribePlayback()` | `playback.read` |
| `togglePlayback()` | `playback.control` |
| `nextTrack()` | `playback.control` |
| `previousTrack()` | `playback.control` |
| `readActiveSports()` | `sports.read` |
| `readSportsScores(league)` | `sports.read` |
| `publishSportsBadges({ badges })` | `sports.read` |
| `publishWidgets({ widgets })` | none |
| `ready()` | none |
| `close()` | none |
| `pendingCommand()` | none, API v3 only |

API v3 packages can use `onCommand()` to receive the host's `luma:command` event. Register the listener before calling `ready()`. Use `onSettings()` and `onPlayback()` with the matching subscribe calls for host-pushed snapshots; delivery stops when the session stops.

## Example

```ts
import { bridge } from "@luma/plugin-sdk";

const { values } = await bridge.readSettings();
document.querySelector("h1")!.textContent = String(values.greeting ?? "Hello");
await bridge.ready();
```

Host failures reject with `LumaBridgeError`. Use `isBridgeError()` or `toBridgeError()` to read `code` and `message`.

See [the repository SDK guide](../../docs/PLUGIN-SDK.md) for the raw JavaScript protocol, editor schema, starter, and integration notes.
