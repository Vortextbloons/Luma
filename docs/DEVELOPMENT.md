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

Plugin authoring uses the `luma-plugin` CLI for scaffolding, validation, packing, and isolated development sessions. See [plugin development](PLUGIN-DEVELOPMENT.md).

Rust builds use the toolchain pinned in `rust-toolchain.toml`, so a `rustup update` no longer invalidates the compiled dependencies. Cargo never prunes its own artifacts, so `npm run dev` sweeps them once `src-tauri/target` passes 20 GB (`cargo install cargo-sweep` once); `npm run clean` runs the same sweep on demand.

The `window_handoff` regression check opens isolated Windows test windows. It checks stationary-pointer hover targets and 40 fullscreen window handoffs, including WebView input focus, without starting the remote server.

## Versioning

Change the application version with `npm run version:set -- patch` (or `minor`, `major`, or an explicit version such as `0.2.0`). This updates the npm and Tauri manifests and both lockfiles together. Run `node scripts/version.mjs --check` to verify they agree.

Add a matching `## [x.y.z]` section to [CHANGELOG.md](../CHANGELOG.md) for each version. A release refuses to publish without one, uses that section as the GitHub release notes, and the same notes appear in the in-app update prompt.

## Releases and updates

The desktop header shows VPN detection from active Windows network profiles and refreshes on network changes without polling. Detection uses tunnel/PPP interface types and common VPN adapter names for profiles that report internet access, so a mesh VPN such as Tailscale without an exit node is ignored; it cannot verify that all traffic is protected, and some third-party VPNs may not be identifiable. Luma leaves internet traffic on Windows' configured route. Web remote and Display Box pairing advertise local Ethernet/Wi-Fi addresses instead of the default internet route, so a VPN tunnel address is not preferred. VPN settings must allow local network access for remote control and local media servers; Luma does not bypass a VPN's kill switch or LAN restrictions.

Settings → System → Updates checks for a newer release automatically once per app launch and provides a manual check. **Update now** downloads and verifies the signed installer; **Restart to update** applies it and reopens Luma. Leaving Settings preserves the download. Downloads stay in memory until installation or app exit; closing Luma before installing means downloading again. The updater downloads a full compressed installer, not binary deltas, and only downloads after you choose Update now. Preferences and application data stay in their existing locations.

Local builds have updates disabled. No repository or credentials are needed while developing. When ready to distribute:

1. Push this project to a public GitHub repository.
2. Generate an updater signing key with `npm run tauri -- signer generate -w <path-outside-the-repository>`. Keep the private key and password backed up securely; future releases must use the same key.
3. In GitHub repository Actions settings, add the public key contents as variable `TAURI_SIGNING_PUBLIC_KEY`, the private key contents as secret `TAURI_SIGNING_PRIVATE_KEY`, and its password as secret `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` (omit for an unencrypted key). Never commit the private key.
4. Run `npm run version:set -- patch`, add the new section to `CHANGELOG.md`, commit the changed files, and push them.
5. Save the updater private key as a file outside the repository, then set `TAURI_SIGNING_PRIVATE_KEY` to that path and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` to its password for your user account. `npm run release` reads the public key from the repository variable and needs the GitHub CLI signed in.
6. Run `npm run release`. It stops unless the working tree is clean, the commit is pushed, and the version tag does not exist yet, then checks versions, types, tests, and the changelog entry, builds a signed Windows x64 NSIS installer, generates the updater manifest, and creates a draft GitHub release with `gh`. Review the assets and release notes, then publish the draft when ready. Keep `latest.json`, the installer, and its signature attached. Only published stable releases are offered to installed apps. The Release workflow can still build the current version manually from the Actions tab.

The release derives the updater URL from the repository and embeds the public key in the installer. Releases use a per-user installation and quiet updates, so Restart to update does not open an installer wizard. Install the first release manually to establish the update channel. Subsequent published releases appear in Settings. Test the full flow with two successive versions before distributing widely. Moving repositories or replacing signing keys requires a migration for existing installations. Updater signing verifies updates; Windows code signing is separate and SmartScreen may still prompt for an unsigned installer.

## Interface and navigation

The AI customization foundation gives Home/appearance configuration a native revisioned owner, an opt-in local MCP endpoint, ten saved versions, and a declarative plugin lifecycle. Version 3 supports one/two Home columns and per-section spans. Current and previous v1/v2 documents migrate together. Home layout edits use a separate draft, Preview, Keep, cancel, default-layout draft, and previous-configuration restore. Plugins can contribute appearance and Home fields through a manifest, declare bounded settings, and be imported/exported as portable ZIPs; isolated UI/background packages now require explicit access review and run through the supervisor. Stage 6/7 live acceptance remains open. See [plugins](PLUGINS.md), [server setup](CUSTOMIZATION-SERVER.md), and [the roadmap](CUSTOMIZATION-ROADMAP.md) for lifecycle rules, supported operations, and open manual acceptance gates.

Arrow keys move visible focus, Enter activates, Escape or Backspace returns, and Home opens the home screen. Tab and Shift+Tab wrap within the current modal. Text fields keep their normal editing keys. Keyboard and authenticated remote inputs share the same navigation dispatcher.

Settings groups are Appearance, Home, Display & Sound, Apps & Accounts, Controls, Watch History, System, and Add-ons. Saved versions and reset are under Appearance. Plugins and AI access are under Add-ons.
 Ordinary choices - theme, density, tile shape, text size, reduced motion, interface sounds, and sound level - appear in the desktop settings and in the shared remote setting contract used by the web remote and Display Box. Home layout, widget placement, theme, density, tile shape, text size, reduced motion, interface sounds, sound level, and app order are owned by the native customization store after migration, so remote changes persist and apply even when the launcher webview is absent. The remote setting catalog is derived from the same revisioned native document, so every surface reads and writes one owner. The WebView retains a legacy mirror for rollback plus explicitly local preferences (window size, fast app switching, and the Home shortcut) in local storage. App paths are saved separately in `apps.json` under Tauri's application configuration directory. Choose **Save app settings** after editing paths; other ordinary settings save automatically. Luma validates local absolute `.exe` paths and launches them directly without shell commands. Services are described by one registry in `apps.rs` - id, executable locations, and streaming URL - so adding a service is a data change rather than a new launch path.

Appearance is built on one token contract in `src/lib/services/theme.ts`. Each theme is a complete palette of semantic variables (`--bg`, `--surface`, `--text`, `--accent`, and so on), not just an accent color. The app applies the selected palette to the document root, and the same palette and theme list are sent to the Rust remote through `update_appearance`; the web remote follows the TV by default and can keep a local theme override in its settings. The Home layout is an ordered `home` array in settings, including the featured hero and every row that can appear on Home; hidden rows keep their position. App order is a separate `order` array. It is committed through the native customization store, returned by `/apps` in that order, included as `appOrder` in state, and can be edited from the web remote's App layout editor. The TV applies revisioned `customization-changed` events. Widgets are registered in `src/lib/services/widgets.ts`, with one Spotify widget for now, and their placement is stored per widget. The layout editor's preview uses the live catalog, activity, app, and widget data.

Spotify, Netflix, Disney+, and Prime Video are automatically detected at startup and whenever launched with an empty configured path. Detection checks standard per-user/system installation locations and Windows App Paths registrations. **Find installed apps** fills empty path fields without replacing manual overrides. Stremio opens inside Luma and has no executable path. The custom tile always requires a path.

A configured or detected executable always wins over the browser. When a service has no local application — Apple TV+, Hulu, Max, and Peacock, or any detected app that has been uninstalled — Luma opens its streaming URL in a Microsoft Edge app window:

```
msedge.exe --app=<url> --user-data-dir=<config>/services/<id> --no-first-run --no-default-browser-check --start-maximized
```

Edge app windows have no tabs, address bar, or browser chrome, and each service gets its own browser profile, so sign-ins are independent and a second launch reuses the window that profile already owns. These are the only arguments Luma passes to anything; they come from the registry, never from the remote or the WebView. Each service's URL is pinned in that profile, so a service that later changes its URL needs a registry edit rather than a browser bookmark.

## Music and Spotify

The Music view controls the Spotify desktop app on this PC. Playback and track information arrive through Windows' GlobalSystemMediaTransportControlsSessionManager, so Spotify must be running and exposing a compatible media session. Unsupported actions are disabled, and missing sessions are reported.

Music plays independently of video. Opening Spotify does not close a running movie or Stremio window, and the Music view is only raised when nothing else is playing. Both remotes show a music strip whenever Spotify has a session, with a play/pause, previous, and next control that target Spotify rather than the movie. The same strip appears on the Display Box TV Remote tab; the frame keeps its own App Remote integration for music it plays locally, while the strip controls the PC.

An optional Spotify account connection adds playlists, saved albums, liked songs, search, recently played, queue and device transfer. Luma uses the user's own Spotify developer app for this: sign in to the Spotify Developer Dashboard, create a Web API app, register the redirect URI `http://127.0.0.1:8888/callback` (Spotify does not allow `localhost`), and paste the Client ID into Settings → Apps & Services → Accounts → Spotify. No client secret is needed. Connection uses PKCE with a local loopback callback, and tokens are stored in Windows Credential Manager under `Luma/Spotify/OAuth`. Playback control through the account requires Spotify Premium. Luma does not bundle a Spotify streaming engine.

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

Use Settings → Watch History to pause recording or clear history. Use Home → Services on Home to hide a service, or Apps & Accounts → Accounts to disable an integration. A single item can also be removed from a card. Cached history remains usable offline and survives restarts; clearing history also works without a network connection.

## Stremio account integration

Settings → Apps & Services → Accounts → Stremio links an account through Stremio's device-link service. Luma requests a short code from `https://link.stremio.com/api/v2/create?type=Create`, shows the code, link, and QR code, and polls `https://link.stremio.com/api/v2/read?type=Read&code=...` until the user approves it in Stremio. The link returns a one-time token, which Luma exchanges through `https://api.strem.io/api/loginWithToken` for the session auth key. No password is requested, and Luma never reads cookies or credentials from the installed Stremio application.

After pairing, Luma calls `https://api.strem.io/api/getUser` and `https://api.strem.io/api/datastoreGet` with collection `libraryItem` and `all: true` to load the account email and synchronized library. These requests run only in the Rust backend. The auth key is stored in Windows Credential Manager under `Luma/Stremio` and is never returned by a Tauri command, emitted to the WebView, or exposed by the local remote server. `stremio.json` under Tauri's application configuration directory holds the non-secret email, last synchronization time, and library metadata so Continue Watching can render immediately at startup. Disconnecting deletes only that credential and cache; it does not call Stremio's logout endpoint or modify the user's account.

Continue Watching uses Stremio's library state: titles with a positive `timeOffset`, excluding type `other`; progress is `timeOffset / duration`. Selecting a title opens the official Stremio Web app (`https://web.stremio.com/`) in a dedicated Luma window at `#/detail/{type}/{id}/{videoId}`. The Stremio tile, the catalog details dialog, and the hero title action use the same window. External links from Stremio open in the default browser; the embedded window only navigates Stremio hosts. `Ctrl+W`, Escape or Backspace at the Stremio home screen, or the remote Home command closes the window and returns to Luma.

The embedded window signs in with Luma's device-linked Stremio account through Stremio Web's authentication flow, which loads the account's library and installed add-ons. Its session persists in WebView2's application profile. Disconnecting in Luma closes the embedded window and clears its managed account the next time it opens. Luma does not bundle a streaming engine, Stremio Service, or a video player.

Stremio Web plays streams that its HTML5 player can handle directly, including HLS. Torrent, local-file, and transcoded playback require Stremio Service (or a running Stremio desktop installation, which provides the same service) at the default `http://127.0.0.1:11470/`; without it those streams are unavailable. Subscription services are not played by Stremio and its links to them leave the embedded window. WebView2 does not include Widevine, so DRM-protected playback from services such as Netflix or Disney+ is not available inside this window.

These endpoints, payload shapes, and web routes are implementation-specific. The integration follows Stremio's upstream `stremio-core` commit `2c89236b47dfcd5e844c1dfae5737164cb616fbf` (`src/models/link.rs`, `src/types/api/request.rs`, `src/deep_links/mod.rs`) and the current `stremio-web` hash routes (`/detail/:type?/:id?/:videoId?`). Re-check these interfaces when updating the integration; `stremio-core` itself is not bundled because its runtime, add-on system, and player dependencies are much larger than this adapter needs.

## Local remote server

One local server starts with Luma at `0.0.0.0:9877` when local remote access is enabled in Settings → Remotes & Devices. It serves the Display Box frame protocol and a web remote on the same LAN, so both can stay connected at once. Disabling access stops the server. LAN traffic is unencrypted HTTP; use it only on a trusted private network, and allow Luma through Windows Firewall on private networks. No firewall rules are added automatically. Pairing tokens are stored in Windows Credential Manager; `display-box.json` holds only the enable flag. Settings lists the Display Box and web remote separately. **Forget** clears that device's token; **Forget all remotes** clears both, so every browser and Display Box has to pair again. Neither action changes preferences or accounts.

### Display Box frame

The frame keeps its existing protocol on port 9877 and authenticates with `Authorization: Bearer <frame-token>`:

- `POST /pair` accepts `{ "code": "123456" }` and returns `{ "token": "...", "pcName": "..." }`.
- `GET /state` returns the PC name, connection state, appearance, app order, and sound state.
- `POST /command` accepts `{ "command": "up" }`. Navigation commands are `up`, `down`, `left`, `right`, `activate`, `back`, and `home`. App commands are `youtube`, `spotify`, `stremio`, and `custom`; Sports shortcuts are `sports`, `sportsnba`, `sportsnfl`, `sportsmlb`, and `sportsnhl` and open that league in Luma. These shortcuts are included in `/apps` with `shortcut: true` and local league icons; playback commands are `toggle`, `previous`, and `next`; music-only commands are `musictoggle`, `musicprevious`, and `musicnext`; volume commands are `volumeup`, `volumedown`, and `mute`.
- `GET /apps` returns the app catalog in the current dock order; the order is shared with the web remote and the TV.
- `GET /phone/music/art` returns the current album artwork as image bytes for remotes that can draw it. It is not part of the state payload; the socket frames are capped at 4 KB.

`toggle`, `previous`, and `next` keep their original meaning: they follow whatever is in front, so a visible movie wins and Spotify receives them only when no video is open. The `music*` commands always target the Spotify media session regardless of what is on screen, so music can be paused, skipped, and resumed while a movie keeps playing. Music commands do not wake the launcher or hide the pointer. The state payload carries a compact `music` object (`playing`, `title`, `artist`, `hasArtwork`, `positionMs`, `durationMs`, `updatedAt`, and capability flags) and omits it when no Spotify session exists. Remotes interpolate playback position from `positionMs` and `updatedAt` instead of receiving a tick every second.

Frame credentials are persistent and are not replaced by web remote pairing. This protocol rejects requests that include an Origin header. Commands cannot carry executable paths or shell arguments. Settings can forget the frame or web remote separately; **Forget all remotes** clears both credentials.

### Web remote

Open `http://<PC-Wi-Fi-address>:9877/` in a browser on any device connected to the same network, or choose **Open remote in browser** in Settings to test the same page at `http://localhost:9877/` on this PC. Settings shows the exact web remote address. Choose **Connect** in Luma and scan the QR code; the code is carried in the URL fragment, so the page pairs as soon as it opens. The six-digit code can also be typed. Both addresses use the same server, pairing, and commands. The server sets an `HttpOnly`, `SameSite=Strict` browser cookie that expires after 400 days, and the page uses an authenticated WebSocket for commands and state checks while visible. It closes the socket in the background and reconnects when visible. HTTP commands remain available while the socket connects; interrupted commands are never replayed automatically. Web remote pairing is stored separately from the frame credential, so pairing or replacing either one does not disconnect the other. Luma must stay running; closing its window leaves it in the system tray.

The page is a standalone Svelte, TypeScript, and Tailwind app in `remote/`. `npm run build:remote` compiles it with Vite and inlines the result into the single `src-tauri/src/phone.html` file embedded in the Rust server; edit the source, not that generated file. `npm run build` rebuilds both the TV app and the remote. For remote UI work, `npm run dev:remote` serves the source at `http://localhost:1422` and proxies the phone API and WebSocket to a running Luma at `9877`.

The remote keeps its own settings under **This remote** (Controls, This browser, Connection). The Luma app settings follow the desktop categories wherever the shared contract supports them: **Appearance & Home** (theme, layout, Home columns), **Display & Sound** (sound set and level, the controlled PC window), and **Apps & Services** (shared dock order). Sections and toggles come from the `settings` payload's `section` and `kind`, so new ordinary settings appear without a page change. Settings that require the desktop UI — app paths, accounts, history, TV and audio devices, startup, updates, and fast app switching — remain on the PC and are called out in the remote.

The page links a web app manifest, 192/512 icons, a maskable icon, and an Apple touch icon, so it can live on a phone's home screen. On iPhone and iPad, use Share → Add to Home Screen and keep **Open as Web App** on if the option appears. iOS 17.2 and later copies the browser's cookies into the installed app, so pairing in Safari normally carries over; earlier versions may ask for the code once. On Android over HTTP, Chrome adds a home-screen shortcut that opens in the browser, because a standalone Web App install requires HTTPS. The page shows the matching steps after pairing and in Settings → Remotes & Devices. Saved shortcuts are tied to the PC's current address: reserve its LAN address in the router, or pair again from a new QR code if the address changes. Luma cannot rediscover a changed address from a saved shortcut. If a loaded remote loses the PC, it shows a retry banner and reconnects with backoff; a cold launch while the PC is off is handled by the browser, so keep Luma available in the tray.

- `GET /` serves the web remote page.
- `GET /manifest.webmanifest` serves the install manifest.
- `GET /icons/{name}` serves `luma-192.png`, `luma-512.png`, `luma-maskable-512.png`, and `apple-touch-icon.png`.
- `POST /phone/pair` accepts `{ "code": "123456" }` and sets the web remote cookie.
- `GET /phone/state` returns the PC name, current Luma view, appearance, app order, and sound state.
- `POST /phone/command` accepts the same command objects as Display Box.
- `POST /phone/layout` accepts `{ "order": ["Netflix", "YouTube", "...", "Custom"] }` and applies the dock order to the TV and both remotes.

Web remote commands require the server's own origin; cross-origin and missing-origin POSTs are rejected. The separate port 9876 HTTP/WebSocket remote, its manually configured allowed origin, and its per-session credential are replaced by this consolidated server. Browsers can now control navigation, app launches, playback, and PC volume from the page Luma serves.

## Remote control of launched applications

While an application started from Luma is still running, remote commands go to it instead of the launcher. The four arrow commands move the Windows pointer inside that application's window, `activate` clicks at the pointer, and `back` sends Escape to the focused window. `home`, Ctrl+W, and the Home button close the launched application window and restore Luma, and Luma also restores itself if the tracked process exits. Only one application is tracked at a time, because launching an app minimizes Luma. The window is matched by the tracked process or its children, falling back to a visible window whose title contains the application name, which is what resolves Store-packaged apps hosted by `ApplicationFrameHost`.

There is no directional focus model inside those applications. Their keyboard shortcuts and layout differ per service and change without notice, so directional navigation stays a pointer rather than D-pad focus. Media keys are not sent, so playback is controlled through the service's own interface.

## System tray and Windows startup

Closing the main window asks whether to keep Luma running in the system tray or quit completely. Keeping it running destroys the launcher and embedded YouTube and Stremio webviews, and leaves the Display Box and web remote available; quitting stops the server and tray. The remote's Stop command does the same without asking. **Keep launcher ready**, off by default, retains one blank WebView2 window while Luma is in the tray. The next launch reuses the browser runtime and closes the blank window. Returning Home or switching away closes unused embedded webviews immediately. With Fast app switching enabled in Settings → System → Startup & background, YouTube and Stremio are paused and kept hidden instead so they can be reused, using more memory until the setting is turned off, Luma is hidden to the tray, or the remote Stop command runs. Background startup creates no launcher webview. Left-click the tray icon or choose **Open Luma TV** to rebuild the launcher, or choose **Exit Luma…** to confirm a full exit.

In Settings → System → Startup & background, enable **Start Luma when I sign in** to open fullscreen on the selected TV display. Display Box must be enabled separately under Remotes & Devices for the photo frame to connect. A single-instance guard prevents a second copy from occupying the server ports; opening Luma again restores the running instance. Configure startup from the installed application so Windows points to its permanent executable location.

## Assets

Spotify's local icon comes from its [official design asset download](https://developer.spotify.com/documentation/design). Stremio's icon comes from the [official brand repository](https://github.com/Stremio/stremio-brand/blob/master/logos/SVG/stremio-logo-icon-only-fullcolor.svg). Brand assets remain their respective owners' trademarks. YouTube's local icon comes from its [official favicon asset](https://www.gstatic.com/youtube/img/branding/favicon/favicon_144x144.png). The custom app tile uses a neutral icon. The web remote's manifest and Apple touch icons are derived from `src-tauri/icons/icon.png` and embedded from `src-tauri/src/web/`.

## Validation limits

Automated checks cover frontend types/build, directional focus/modal navigation, activity progress/capability selection and filtering, executable-path rejection, remote authentication/origin/command validation, the web app manifest and QR pairing handoff, the Svelte remote's command acknowledgments, pointer gestures, recents, appearance resolution, install guidance, and timeline seeking, Stremio library/deep-link mapping, and pointer clamping to the tracked window. Visual 1080p/4K behavior and playback against a live Spotify installation should also be checked on the target TV/PC. The web remote's portrait, landscape, and wide layouts, both control modes, remote-local settings, and every dynamic Luma app setting group should be checked against a running Luma before release. Home-screen installation, iOS standalone safe areas, and Android's browser shortcut need a real iPhone/iPad and Android phone. Window matching and pointer control should be checked against each installed service, since foreground behavior differs between desktop and Store-packaged applications. Stremio device linking and account synchronization require a live Stremio account. Embedded Stremio Web loading, its sign-in, external-link handling, and playback were not validated in this checkout; playback additionally requires a reachable Stremio Service for torrent or transcoded streams and is subject to WebView2 codec and Widevine limits. Plex, Jellyfin, and Trakt imports require a reachable server or account and were not validated in this checkout; they remain **Needs check** until a real refresh succeeds. YouTube progress reporting and direct app launches need a live Luma desktop session. Packaged Store applications depend on Windows exposing a usable executable alias or manually configured executable.


### Widgets and viewing layouts

Spotify supports the bottom bar and all four corners, or a custom position. Move on the widget provides dragging with edge snapping and remote-friendly directional buttons; Done saves and Back cancels. Visibility can be Always, When Spotify is available, or Only during playback. Positions are normalized integer x/y values from 0 through 1000 across the available widget area; null uses the selected preset. The native customization document owns visibility and position, including saved versions, preview, reset, and MCP writes. Existing documents default to available visibility and preset positioning.

Built-in widgets register their component, placements, and visibility predicate in `src/lib/services/widgets.ts`. The shared `WidgetHost` owns positioning, movement, and focus return; widget components consume existing service state and actions. Register new built-ins through the shared type, defaults, native validation, and settings contract as well. This does not enable executable third-party widgets.

Multi Watch is under the header button or Settings > Display & Sound > Display. Enable Side by side, Separate displays, or Floating video, then add up to two players. The source picker is derived from the app registry, including YouTube, Stremio, sports shortcuts, streaming services, and the configured Custom app. App launches from Home, Apps, activity, and the remote share the same native-window joining path. Add open window also accepts other app/browser windows, including separate windows from the same player. Built-in YouTube, Stremio, and Sports still have one instance each.

External sources retain their original app or browser playback environment. Multi Watch stores the selected window's process identity, position, placement, and borders, and restores them on removal, tray stop, or exit. Window selection is refreshed on demand; there is no background window scan. The picker excludes Luma's own windows and tool windows. Closed/replaced windows are pruned before operations. Use windowed mode in apps that control their own fullscreen bounds. Selecting Single player restores external windows and maximizes the selected one; their own playback can continue until paused by the user.

Managed Luma players follow the selected player for audio. External windows use their own audio controls, avoiding process-wide muting of other tabs or windows. Known Edge services retain their existing remote navigation and playback integration; arbitrary external players receive directional keys and use their own playback controls. Home returns to browsing without closing Multi Watch sources. The remote More menu switches the selected player. Floating external windows are raised above Luma without changing their owner or making them globally always on top.


Switch display is available in the header, Display settings, and the remote More menu. Ctrl+Alt+M is a Windows global shortcut. It cycles connected displays without changing the saved TV startup preference or audio output. New player and external-app launches use the current session's display. The shortcut reports registration conflicts and is unregistered at exit.
