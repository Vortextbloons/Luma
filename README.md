# Luma TV

A Windows TV launcher built with Tauri 2, Svelte 5, TypeScript, Tailwind CSS 4, and Rust. Artwork comes from the supplied UI prototype and is bundled locally. Movie rows and progress bars are clearly labeled demo content; Luma launches installed applications rather than streaming movies itself.

Use current Node.js LTS, npm, Rust stable, and the Windows Tauri prerequisites (MSVC build tools and WebView2).

```sh
npm install
npm run tauri dev
npm run check
npm test
npm run build
cargo check --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml --lib
npm run tauri build
```

`npm run dev` runs a frontend browser preview. Native features require `npm run tauri dev`. Stop a separately running Vite process on port 1420 before starting the Tauri dev command. `npm run tauri build -- --no-bundle` produces the standalone executable without an installer.

Arrow keys move visible focus, Enter activates, Escape or Backspace returns, and Home opens the home screen. Tab and Shift+Tab wrap within the current modal. Text fields keep their normal editing keys. Keyboard and authenticated remote inputs share the same navigation dispatcher.

Settings provide fullscreen, accent colors, wallpaper, density, section visibility, tile ordering, and executable paths. Appearance preferences are stored in the WebView's local storage. Approved app paths are saved separately in `apps.json` under Tauri's application configuration directory. Choose **Save approved app paths** after editing paths. Luma validates local absolute `.exe` paths and launches them directly without shell commands or arguments.

Spotify and Stremio are automatically detected at startup and whenever launched with an empty configured path. Detection checks standard per-user/system installation locations and Windows App Paths registrations. **Find installed apps** fills empty path fields without replacing manual overrides. Nonstandard installations or Store packages without an executable alias may need a manual path. The custom tile always requires a path.

The Music view controls only a Spotify Windows media session, using GlobalSystemMediaTransportControlsSessionManager. Track information and playback changes arrive through Windows events. Spotify must be running and have exposed a compatible media session; unsupported actions are disabled and missing sessions are reported. No Spotify OAuth or streaming engine is included.

## Remote protocol

The Rust HTTP/WebSocket server starts with Luma at `127.0.0.1:9876` and stops with the process. LAN access is off after every restart. Settings can explicitly enable binding to `0.0.0.0:9876` and set an exact allowed HTTP(S) origin for the remote client. Every request requires a random 256-bit session credential. Restarting or reconfiguring the server rotates that credential and disconnects existing WebSockets. The credential is visible only in desktop Settings and is never returned by HTTP endpoints.

LAN mode uses unencrypted HTTP/WebSocket traffic. Use it only on a trusted private network; TLS, six-digit pairing, and remembered remote credentials are outside v0.1. No firewall rules are added automatically. Remote commands cannot supply executable paths or shell arguments.

- `GET /state`: returns `{ "view": "Home", "fullscreen": false }`.
- `POST /command`: accepts `{ "command": "up" }`. Valid commands: `up`, `down`, `left`, `right`, `activate`, `back`, `home`. HTTP 202 means queued for frontend navigation.
- `GET /ws`: WebSocket upgrade, accepting the same JSON command objects. Replies include `accepted` and a snapshot of basic state, or an error. The snapshot may precede the queued navigation's render.

For HTTP, send `Authorization: Bearer <session-token>`. An Origin header, when present, must match the configured origin exactly. HTTP clients should be native clients; cross-origin browser HTTP requests are not enabled.

WebSocket handshakes must include the exact configured `Origin`, plus either the bearer header or `Sec-WebSocket-Protocol: luma.<session-token>`. Browser clients can use:

```js
const socket = new WebSocket('ws://PC_ADDRESS:9876/ws', ['luma.' + token]);
socket.onopen = () => socket.send(JSON.stringify({ command: 'right' }));
```

Serve the browser remote client from the origin configured in Settings. `file://` and `null` origins are rejected. WebSockets are limited to four clients, 512-byte messages, and approximately 33 commands per second per connection. Invalid command types and extra command fields are rejected.

## Assets

Spotify's local icon comes from its [official design asset download](https://developer.spotify.com/documentation/design). Stremio's icon comes from the [official brand repository](https://github.com/Stremio/stremio-brand/blob/master/logos/SVG/stremio-logo-icon-only-fullcolor.svg). Brand assets remain their respective owners' trademarks. YouTube's local icon comes from its [official favicon asset](https://www.gstatic.com/youtube/img/branding/favicon/favicon_144x144.png). The custom app tile uses a neutral icon.

## Validation limits

Automated checks cover frontend types/build, directional focus/modal navigation, executable-path rejection, and remote authentication/origin/command validation. Visual 1080p/4K behavior and playback against a live Spotify installation should also be checked on the target TV/PC. Packaged Store applications depend on Windows exposing a usable executable alias or manually configured executable.

## System tray and Windows startup

Closing the main window hides Luma in the system tray while its servers keep running. Left-click the tray icon or choose **Open Luma TV** to restore the launcher. Choose **Quit Luma and stop server** to exit completely.

In Settings → Background server, enable **Start server when Windows starts** to launch quietly after Windows sign-in. Startup runs the same application with `--background`, without showing the launcher. Display Box must be enabled separately for the photo frame to connect. A single-instance guard prevents a second copy from occupying the server ports; opening Luma again restores the running instance. Configure startup from the installed application so Windows points to its permanent executable location.
