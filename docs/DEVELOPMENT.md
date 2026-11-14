# Luma development notes

Architecture, integration and release notes for Luma. The main [README](../README.md) is the product overview.

## Requirements and commands

Use current Node.js LTS, npm, Rust stable, and the Windows Tauri prerequisites (MSVC build tools and WebView2).

```sh
npm install
npm run tauri dev
npm run check
npm test
npm run build
cargo check --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml --lib
cargo run --manifest-path src-tauri/Cargo.toml --example window_handoff
npm run tauri build
```

`npm run dev` runs a frontend browser preview. Native features require `npm run tauri dev`. Stop a separately running Vite process on port 1420 before starting the Tauri dev command. `npm run tauri build -- --no-bundle` produces the standalone executable without an installer.

The `window_handoff` regression check opens isolated Windows test windows. It checks stationary-pointer hover targets and 40 fullscreen window handoffs, including WebView input focus, without starting the remote server.

## Versioning

Change the application version with `npm run version:set -- patch` (or `minor`, `major`, or an explicit version such as `0.2.0`). This updates the npm and Tauri manifests and both lockfiles together. Run `node scripts/version.mjs --check` to verify they agree.

Add a matching `## [x.y.z]` section to [CHANGELOG.md](../CHANGELOG.md) for each version. The Release workflow refuses to publish without one, uses that section as the GitHub release notes, and the same notes appear in the in-app update prompt.

## Releases and updates

The desktop header shows VPN detection from active Windows network profiles and refreshes on network changes without polling. Detection uses tunnel/PPP interface types and common VPN adapter names for profiles that report internet access, so a mesh VPN such as Tailscale without an exit node is ignored; it cannot verify that all traffic is protected, and some third-party VPNs may not be identifiable. Luma leaves internet traffic on Windows' configured route. Web remote and Display Box pairing advertise local Ethernet/Wi-Fi addresses instead of the default internet route, so a VPN tunnel address is not preferred. VPN settings must allow local network access for remote control and local media servers; Luma does not bypass a VPN's kill switch or LAN restrictions.

Settings → Updates checks for a newer release automatically once per app launch and provides a manual check. **Update now** downloads and verifies the signed installer; **Restart to update** applies it and reopens Luma. Leaving Settings preserves the download. Downloads stay in memory until installation or app exit; closing Luma before installing means downloading again. The updater downloads a full compressed installer, not binary deltas, and only downloads after you choose Update now. Preferences and application data stay in their existing locations.

Local builds have updates disabled. No repository or credentials are needed while developing. When ready to distribute:

1. Push this project to a public GitHub repository.
2. Generate an updater signing key with `npm run tauri -- signer generate -w <path-outside-the-repository>`. Keep the private key and password backed up securely; future releases must use the same key.
3. In GitHub repository Actions settings, add the public key contents as variable `TAURI_SIGNING_PUBLIC_KEY`, the private key contents as secret `TAURI_SIGNING_PRIVATE_KEY`, and its password as secret `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` (omit for an unencrypted key). Never commit the private key.
4. Run `npm run version:set -- patch`, add the new section to `CHANGELOG.md`, commit the changed files, then push a matching tag such as `v0.1.1`. Alternatively run the Release workflow manually for the current version.
5. The workflow checks versions, types, tests, and the changelog entry, then builds a signed Windows x64 NSIS installer and updater manifest in a draft GitHub release. Review the assets and release notes, then publish the draft when ready. Keep `latest.json`, the installer, and its signature attached. Only published stable releases are offered to installed apps.

The workflow derives the updater URL from the GitHub repository running it and embeds the public key in the installer. Releases use a per-user installation and quiet updates, so Restart to update does not open an installer wizard. Install the first workflow-built release manually to establish the update channel. Subsequent published releases appear in Settings. Test the full flow with two successive versions before distributing widely. Moving repositories or replacing signing keys requires a migration for existing installations. Updater signing verifies updates; Windows code signing is separate and SmartScreen may still prompt for an unsigned installer.

## Interface and navigation

Arrow keys move visible focus, Enter activates, Escape or Backspace returns, and Home opens the home screen. Tab and Shift+Tab wrap within the current modal. Text fields keep their normal editing keys. Keyboard and authenticated remote inputs share the same navigation dispatcher.

Settings provide fullscreen, accent colors, wallpaper, density, section visibility, tile ordering, executable paths, TV screen and audio output, and startup options. Appearance preferences are stored in the WebView's local storage. App paths are saved separately in `apps.json` under Tauri's application configuration directory. Choose **Save app settings** after editing paths. Luma validates local absolute `.exe` paths and launches them directly without shell commands. Services are described by one registry in `apps.rs` — id, executable locations, and streaming URL — so adding a service is a data change rather than a new launch path.

Spotify, Netflix, Disney+, and Prime Video are automatically detected at startup and whenever launched with an empty configured path. Detection checks standard per-user/system installation locations and Windows App Paths registrations. **Find installed apps** fills empty path fields without replacing manual overrides. Stremio opens inside Luma and has no executable path. The custom tile always requires a path.

A configured or detected executable always wins over the browser. When a service has no local application — Apple TV+, Hulu, Max, and Peacock, or any detected app that has been uninstalled — Luma opens its streaming URL in a Microsoft Edge app window:

```
msedge.exe --app=<url> --user-data-dir=<config>/services/<id> --no-first-run --no-default-browser-check --start-maximized
```

Edge app windows have no tabs, address bar, or browser chrome, and each service gets its own browser profile, so sign-ins are independent and a second launch reuses the window that profile already owns. These are the only arguments Luma passes to anything; they come from the registry, never from the remote or the WebView. Each service's URL is pinned in that profile, so a service that later changes its URL needs a registry edit rather than a browser bookmark.

## Music and Spotify

The Music view controls the Spotify desktop app on this PC. Playback and track information arrive through Windows' GlobalSystemMediaTransportControlsSessionManager, so Spotify must be running and exposing a compatible media session. Unsupported actions are disabled, and missing sessions are reported.

An optional Spotify account connection adds playlists, saved albums, liked songs, search, recently played, queue and device transfer. Luma uses the user's own Spotify developer app for this: sign in to the Spotify Developer Dashboard, create a Web API app, register the redirect URI `http://127.0.0.1:8888/callback` (Spotify does not allow `localhost`), and paste the Client ID into Settings → Integrations → Spotify. No client secret is needed. Connection uses PKCE with a local loopback callback, and tokens are stored in Windows Credential Manager under `Luma/Spotify/OAuth`. Playback control through the account requires Spotify Premium. Luma does not bundle a Spotify streaming engine.

## Movie catalog

Home and Watch load two public Cinemeta catalogs at startup: Popular movies and Top-rated movies. Titles, descriptions, artwork, year, runtime, and IMDb rating come from Cinemeta and MetaHub, so the rows stay current without an API key or Stremio account. An internet connection is required; when the catalog is unreachable, Luma reports the error and leaves the rows empty. Luma does not stream titles itself; the optional Stremio account integration and the local activity system supply Continue Watching.

## Unified activity

Luma keeps a local-first activity history in `activity.sqlite3` under Tauri's application data directory. The home screen adds Continue Watching, Recently played, and Recently used apps below the movie rows. Every card is built from stored records; Luma never invents history, artwork, positions, or resume support.

Real sources in this build:

- **This PC / launchers** — app launches through Luma, including Spotify, Stremio, Netflix, Disney+, Prime Video, Apple TV+, Hulu, Max, Peacock, YouTube, and a configured custom app. Only the launch is recorded, never what was watched.
- **YouTube inside Luma** — the injected player reports the current video ID, title, channel, position, and duration at a throttled rate while playback is visible. The thumbnail is derived from the video ID.
- **Spotify on Windows** — the current Spotify playback session observed by Luma, using the desktop media session or the connected Spotify integration. This is not a full Spotify listening history.
- **Plex** — authenticated `library/onDeck` and `library/recentlyViewed` requests.
- **Jellyfin** — authenticated resume and played-item requests for a configured user.
- **Trakt** — authorized watch history using a client ID and access token.

Plex, Jellyfin, and Trakt refresh only when the user selects **Check connection**, so startup does not wait for network services. A provider with saved credentials is shown as **Needs check** until a refresh succeeds; only then does it become **Connected**. Provider tokens are written to Windows Credential Manager and are never returned to the WebView, stored in the activity database, or sent to the photo-frame remote. The remote receives only navigation state.

Progress bars appear only when the provider or Luma supplied a valid position and duration. The card action reflects the stored capability: direct resume, open content, open the service, or open the app. This build stores open-content/service actions for the implemented providers because their direct-resume mechanisms have not been validated here, so no card displays a Resume button for an unverified mechanism. Netflix, Disney+, Prime Video, Apple TV+, Hulu, Max, and Peacock are launchers only because no legitimate watch-history source is assumed. Music and video records stay visually distinct, and provider activity is never merged across services by title alone.

Settings → Activity lets you stop recording, hide a provider on Home, disable an integration, clear one provider or all history, or remove a single item from a card. Cached history remains usable offline and survives restarts; clearing history also works without a network connection.

## Stremio account integration

Settings → Integrations → Stremio links an account through Stremio's device-link service. Luma requests a short code from `https://link.stremio.com/api/v2/create?type=Create`, shows the code, link, and QR code, and polls `https://link.stremio.com/api/v2/read?type=Read&code=...` until the user approves it in Stremio. The link returns a one-time token, which Luma exchanges through `https://api.strem.io/api/loginWithToken` for the session auth key. No password is requested, and Luma never reads cookies or credentials from the installed Stremio application.

After pairing, Luma calls `https://api.strem.io/api/getUser` and `https://api.strem.io/api/datastoreGet` with collection `libraryItem` and `all: true` to load the account email and synchronized library. These requests run only in the Rust backend. The auth key is stored in Windows Credential Manager under `Luma/Stremio` and is never returned by a Tauri command, emitted to the WebView, or exposed by the local remote server. `stremio.json` under Tauri's application configuration directory holds the non-secret email, last synchronization time, and library metadata so Continue Watching can render immediately at startup. Disconnecting deletes only that credential and cache; it does not call Stremio's logout endpoint or modify the user's account.

Continue Watching uses Stremio's library state: titles with a positive `timeOffset`, excluding type `other`; progress is `timeOffset / duration`. Selecting a title opens the official Stremio Web app (`https://web.stremio.com/`) in a dedicated Luma window at `#/detail/{type}/{id}/{videoId}`. The Stremio tile, the catalog details dialog, and the hero title action use the same window. External links from Stremio open in the default browser; the embedded window only navigates Stremio hosts. `Ctrl+W`, Escape or Backspace at the Stremio home screen, or the remote Home command closes the window and returns to Luma.

The embedded window signs in with Luma's device-linked Stremio account through Stremio Web's authentication flow, which loads the account's library and installed add-ons. Its session persists in WebView2's application profile. Disconnecting in Luma closes the embedded window and clears its managed account the next time it opens. Luma does not bundle a streaming engine, Stremio Service, or a video player.

Stremio Web plays streams that its HTML5 player can handle directly, including HLS. Torrent, local-file, and transcoded playback require Stremio Service (or a running Stremio desktop installation, which provides the same service) at the default `http://127.0.0.1:11470/`; without it those streams are unavailable. Subscription services are not played by Stremio and its links to them leave the embedded window. WebView2 does not include Widevine, so DRM-protected playback from services such as Netflix or Disney+ is not available inside this window.

These endpoints, payload shapes, and web routes are implementation-specific. The integration follows Stremio's upstream `stremio-core` commit `2c89236b47dfcd5e844c1dfae5737164cb616fbf` (`src/models/link.rs`, `src/types/api/request.rs`, `src/deep_links/mod.rs`) and the current `stremio-web` hash routes (`/detail/:type?/:id?/:videoId?`). Re-check these interfaces when updating the integration; `stremio-core` itself is not bundled because its runtime, add-on system, and player dependencies are much larger than this adapter needs.

## Local remote server

One local server starts with Luma at `0.0.0.0:9877` when local remote access is enabled in Settings → Remote & Startup. It serves the Display Box frame protocol and a web remote on the same LAN, so both can stay connected at once. Disabling access stops the server. LAN traffic is unencrypted HTTP; use it only on a trusted private network, and allow Luma through Windows Firewall on private networks. No firewall rules are added automatically. Pairing tokens are stored in Windows Credential Manager; `display-box.json` holds only the enable flag. **Forget paired remotes** clears both tokens, so every browser and Display Box has to pair again.

### Display Box frame

The frame keeps its existing protocol on port 9877 and authenticates with `Authorization: Bearer <frame-token>`:

- `POST /pair` accepts `{ "code": "123456" }` and returns `{ "token": "...", "pcName": "..." }`.
- `GET /state` returns the PC name and connection state.
- `POST /command` accepts `{ "command": "up" }`. Navigation commands are `up`, `down`, `left`, `right`, `activate`, `back`, and `home`. App commands are `youtube`, `spotify`, `stremio`, and `custom`; playback commands are `toggle`, `previous`, and `next`; volume commands are `volumeup`, `volumedown`, and `mute`.

Frame credentials are persistent and are not replaced by web remote pairing. This protocol rejects requests that include an Origin header. Commands cannot carry executable paths or shell arguments. **Forget paired remotes** clears the frame and web remote credentials.

### Web remote

Open `http://<PC-Wi-Fi-address>:9877/` in a browser on any device connected to the same network, or choose **Open remote in browser** in Settings to test the same page at `http://localhost:9877/` on this PC. Settings shows the exact web remote address. Choose **Connect** in Luma, then enter the six-digit code in the browser you opened. Both addresses use the same server, pairing, and commands. The server sets an `HttpOnly`, `SameSite=Strict` browser cookie that expires after 30 days, and the page uses an authenticated WebSocket for commands and state checks while visible. It closes the socket in the background and reconnects when visible. HTTP commands remain available while the socket connects; interrupted commands are never replayed automatically. Web remote pairing is stored separately from the frame credential, so pairing or replacing either one does not disconnect the other. Luma must stay running; closing its window leaves it in the system tray.

- `GET /` serves the web remote page.
- `POST /phone/pair` accepts `{ "code": "123456" }` and sets the web remote cookie.
- `GET /phone/state` returns the PC name and the current Luma view.
- `POST /phone/command` accepts the same command objects as Display Box.

Web remote commands require the server's own origin; cross-origin and missing-origin POSTs are rejected. The separate port 9876 HTTP/WebSocket remote, its manually configured allowed origin, and its per-session credential are replaced by this consolidated server. Browsers can now control navigation, app launches, playback, and PC volume from the page Luma serves.

## Remote control of launched applications

While an application started from Luma is still running, remote commands go to it instead of the launcher. The four arrow commands move the Windows pointer inside that application's window, `activate` clicks at the pointer, and `back` sends Escape to the focused window. `home`, Ctrl+W, and the Home button close the launched application window and restore Luma, and Luma also restores itself if the tracked process exits. Only one application is tracked at a time, because launching an app minimizes Luma. The window is matched by the tracked process or its children, falling back to a visible window whose title contains the application name, which is what resolves Store-packaged apps hosted by `ApplicationFrameHost`.

There is no directional focus model inside those applications. Their keyboard shortcuts and layout differ per service and change without notice, so directional navigation stays a pointer rather than D-pad focus. Media keys are not sent, so playback is controlled through the service's own interface.

## System tray and Windows startup

Closing the main window asks whether to keep Luma running in the system tray or quit completely. Keeping it running hides the window, destroys the embedded YouTube and Stremio webviews, and leaves the Display Box and web remote available; quitting stops the server and tray. The remote's Stop command does the same without asking, freeing the webview memory until the next launch rebuilds them. Left-click the tray icon or choose **Open Luma TV** to restore the launcher, or choose **Exit Luma…** to confirm a full exit.

In Settings → Remote & Startup, enable **Start Luma when I sign in** to launch quietly after Windows sign-in. Startup runs the same application with `--background`, without showing the launcher. Display Box must be enabled separately for the photo frame to connect. A single-instance guard prevents a second copy from occupying the server ports; opening Luma again restores the running instance. Configure startup from the installed application so Windows points to its permanent executable location.

## Assets

Spotify's local icon comes from its [official design asset download](https://developer.spotify.com/documentation/design). Stremio's icon comes from the [official brand repository](https://github.com/Stremio/stremio-brand/blob/master/logos/SVG/stremio-logo-icon-only-fullcolor.svg). Brand assets remain their respective owners' trademarks. YouTube's local icon comes from its [official favicon asset](https://www.gstatic.com/youtube/img/branding/favicon/favicon_144x144.png). The custom app tile uses a neutral icon.

## Validation limits

Automated checks cover frontend types/build, directional focus/modal navigation, activity progress/capability selection and filtering, executable-path rejection, remote authentication/origin/command validation, Stremio library/deep-link mapping, and pointer clamping to the tracked window. Visual 1080p/4K behavior and playback against a live Spotify installation should also be checked on the target TV/PC. Window matching and pointer control should be checked against each installed service, since foreground behavior differs between desktop and Store-packaged applications. Stremio device linking and account synchronization require a live Stremio account. Embedded Stremio Web loading, its sign-in, external-link handling, and playback were not validated in this checkout; playback additionally requires a reachable Stremio Service for torrent or transcoded streams and is subject to WebView2 codec and Widevine limits. Plex, Jellyfin, and Trakt imports require a reachable server or account and were not validated in this checkout; they remain **Needs check** until a real refresh succeeds. YouTube progress reporting and direct app launches need a live Luma desktop session. Packaged Store applications depend on Windows exposing a usable executable alias or manually configured executable.
