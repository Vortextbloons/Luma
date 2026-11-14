# Luma TV

A Windows TV launcher built with Tauri 2, Svelte 5, TypeScript, Tailwind CSS 4, and Rust. Movie rows and the hero feature load real titles from Stremio's public Cinemeta catalog, with artwork from MetaHub. Luma launches installed applications rather than streaming movies itself. An optional Stremio account connection adds Continue Watching, a synchronized library, and deep links into the installed Stremio desktop app.

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

Settings provide fullscreen, accent colors, wallpaper, density, section visibility, tile ordering, and executable paths. Appearance preferences are stored in the WebView's local storage. Approved app paths are saved separately in `apps.json` under Tauri's application configuration directory. Choose **Save approved app paths** after editing paths. Luma validates local absolute `.exe` paths and launches them directly without shell commands. Services are described by one registry in `apps.rs` — id, executable locations, and streaming URL — so adding a service is a data change rather than a new launch path.

Spotify, Stremio, Netflix, Disney+, and Prime Video are automatically detected at startup and whenever launched with an empty configured path. Detection checks standard per-user/system installation locations and Windows App Paths registrations. **Find installed apps** fills empty path fields without replacing manual overrides. The custom tile always requires a path.

A configured or detected executable always wins over the browser. When a service has no local application — Apple TV+, Hulu, Max, and Peacock, or any detected app that has been uninstalled — Luma opens its streaming URL in a Microsoft Edge app window:

```
msedge.exe --app=<url> --user-data-dir=<config>/services/<id> --no-first-run --no-default-browser-check --start-maximized
```

Edge app windows have no tabs, address bar, or browser chrome, and each service gets its own browser profile, so sign-ins are independent and a second launch reuses the window that profile already owns. These are the only arguments Luma passes to anything; they come from the registry, never from the remote or the WebView. The plan's services are pinned in that profile, so a service that later changes its URL needs a registry edit rather than a browser bookmark.

The Music view controls only a Spotify Windows media session, using GlobalSystemMediaTransportControlsSessionManager. Track information and playback changes arrive through Windows events. Spotify must be running and have exposed a compatible media session; unsupported actions are disabled and missing sessions are reported. No Spotify OAuth or streaming engine is included.

## Movie catalog

Home and Watch load two public Cinemeta catalogs at startup: Popular movies and Featured for you. Titles, descriptions, artwork, year, runtime, and IMDb rating come from Cinemeta and MetaHub, so the rows stay current without an API key or Stremio account. An internet connection is required; when the catalog is unreachable, Luma reports the error and leaves the rows empty. Luma does not stream titles itself; the optional Stremio account integration and the local activity system supply Continue Watching.

## Unified activity

Luma keeps a local-first activity history in `activity.sqlite3` under Tauri's application data directory. The home screen adds Continue Watching, Recently played, and Recently used apps below the movie rows. Every card is built from stored records; Luma never invents history, artwork, positions, or resume support.

Real sources in this build:

- **This PC / launchers** — app launches through Luma, including Spotify, Stremio, Netflix, Disney+, Prime Video, YouTube, and a configured custom app. Only the launch is recorded, never what was watched.
- **YouTube inside Luma** — the injected player reports the current video ID, title, channel, position, and duration at a throttled rate while playback is visible. The thumbnail is derived from the video ID.
- **Spotify on Windows** — the current Spotify playback session observed by Luma, using the desktop media session or the connected Spotify integration. This is not a full Spotify listening history.
- **Plex** — authenticated `library/onDeck` and `library/recentlyViewed` requests.
- **Jellyfin** — authenticated resume and played-item requests for a configured user.
- **Trakt** — authorized watch history using a client ID and access token.

Plex, Jellyfin, and Trakt refresh only when the user selects **Refresh now**, so startup does not wait for network services. A provider with saved credentials is shown as **Not validated** until a refresh succeeds; only then does it become **Connected**. Provider tokens are written to Windows Credential Manager and are never returned to the WebView, stored in the activity database, or sent to the photo-frame remote. The remote receives only navigation state.

Progress bars appear only when the provider or Luma supplied a valid position and duration. The card action reflects the stored capability: direct resume, open content, open the service, or open the app. This build stores open-content/service actions for the implemented providers because their direct-resume mechanisms have not been validated here, so no card displays a Resume button for an unverified mechanism. Netflix, Disney+, Prime Video, Apple TV+, Hulu, Max, and Peacock are launchers only because no legitimate watch-history source is assumed. Music and video records stay visually distinct, and provider activity is never merged across services by title alone.

Settings → Activity lets you stop recording, hide a provider on Home, disable an integration, clear one provider or all history, or remove a single item from a card. Cached history remains usable offline and survives restarts; clearing history also works without a network connection.

## Stremio account integration

Settings → Integrations → Stremio links an account through Stremio's device-link service. Luma requests a short code from `https://link.stremio.com/api/v2/create?type=Create`, shows the code, link, and QR code, and polls `https://link.stremio.com/api/v2/read?type=Read&code=...` until the user approves it in Stremio. The link returns a one-time token, which Luma exchanges through `https://api.strem.io/api/loginWithToken` for the session auth key. No password is requested, and Luma never reads cookies or credentials from the installed Stremio application.

After pairing, Luma calls `https://api.strem.io/api/getUser` and `https://api.strem.io/api/datastoreGet` with collection `libraryItem` and `all: true` to load the account email and synchronized library. These requests run only in the Rust backend. The auth key is stored in Windows Credential Manager under `Luma/Stremio` and is never returned by a Tauri command, emitted to the WebView, or exposed by the remote HTTP/WebSocket server. `stremio.json` under Tauri's application configuration directory holds the non-secret email, last synchronization time, and library metadata so Continue Watching can render immediately at startup. Disconnecting deletes only that credential and cache; it does not call Stremio's logout endpoint or modify the user's account.

Continue Watching uses Stremio's library state: titles with a positive `timeOffset`, excluding type `other`; progress is `timeOffset / duration`. Selecting a title opens `stremio:///detail/{type}/{id}/{videoId}` through the installed Stremio protocol handler. Luma does not bundle Stremio Web, a streaming engine, or a video player.

These endpoints, payload shapes, and deep-link format are implementation-specific. The integration follows Stremio's upstream `stremio-core` commit `2c89236b47dfcd5e844c1dfae5737164cb616fbf` (`src/models/link.rs`, `src/types/api/request.rs`, `src/deep_links/mod.rs`) and was verified against the live link service. Re-check these interfaces when updating the integration; `stremio-core` itself is not bundled because its runtime, add-on system, and player dependencies are much larger than this adapter needs.

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

## Remote control of launched applications

While an application started from Luma is still running, remote commands go to it instead of the launcher. The four arrow commands move the Windows pointer inside that application's window, `activate` clicks at the pointer, and `back` sends Escape to the focused window. `home` minimizes the application and restores Luma, and Luma also restores itself if the tracked process exits. Only one application is tracked at a time, because launching an app minimizes Luma. The window is matched by the tracked process or its children, falling back to a visible window whose title contains the application name, which is what resolves Store-packaged apps hosted by `ApplicationFrameHost`.

There is no directional focus model inside those applications. Their keyboard shortcuts and layout differ per service and change without notice, so directional navigation stays a pointer rather than D-pad focus. Media keys are not sent, so playback is controlled through the service's own interface.

## Assets

Spotify's local icon comes from its [official design asset download](https://developer.spotify.com/documentation/design). Stremio's icon comes from the [official brand repository](https://github.com/Stremio/stremio-brand/blob/master/logos/SVG/stremio-logo-icon-only-fullcolor.svg). Brand assets remain their respective owners' trademarks. YouTube's local icon comes from its [official favicon asset](https://www.gstatic.com/youtube/img/branding/favicon/favicon_144x144.png). The custom app tile uses a neutral icon.

## Validation limits

Automated checks cover frontend types/build, directional focus/modal navigation, activity progress/capability selection and filtering, executable-path rejection, remote authentication/origin/command validation, Stremio library/deep-link mapping, and pointer clamping to the tracked window. Visual 1080p/4K behavior and playback against a live Spotify installation should also be checked on the target TV/PC. Window matching and pointer control should be checked against each installed service, since foreground behavior differs between desktop and Store-packaged applications. Stremio device linking, account synchronization, and deep-link launching require a live Stremio account and an installed desktop app. Plex, Jellyfin, and Trakt imports require a reachable server or account and were not validated in this checkout; they remain **Not validated** until a real refresh succeeds. YouTube progress reporting and direct app launches need a live Luma desktop session. Packaged Store applications depend on Windows exposing a usable executable alias or manually configured executable.

## System tray and Windows startup

Closing the main window hides Luma in the system tray while its servers keep running. Left-click the tray icon or choose **Open Luma TV** to restore the launcher. Choose **Quit Luma and stop server** to exit completely.

In Settings → Background server, enable **Start server when Windows starts** to launch quietly after Windows sign-in. Startup runs the same application with `--background`, without showing the launcher. Display Box must be enabled separately for the photo frame to connect. A single-instance guard prevents a second copy from occupying the server ports; opening Luma again restores the running instance. Configure startup from the installed application so Windows points to its permanent executable location.
