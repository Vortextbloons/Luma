# Changelog

Notable changes to Luma, newest first. Releases publish the matching section as the GitHub release notes and in-app update notes.

## [1.0.0] - 2026-11-27

### Core Changes

- **Game controller navigation & background guide button**: Full gamepad control (Xbox, PlayStation, Nintendo, generic) across the launcher and settings, plus an XInput background guide button that brings Luma to the front over fullscreen games.
- **Widget studio & Clock widget**: Place, resize, and style multiple widget instances with layout presets for Home, the player HUD, and idle screens, including a customizable Clock widget.
- **Steam non-Steam game shortcuts**: Launch non-Steam games alongside Steam titles with automatic artwork and binary `shortcuts.vdf` integration.
- **Streamlined 8-category TV Settings**: Settings reorganized for 10-foot TV viewing with controller bumper category cycling, responsive subpanels, and faster back navigation.
- **Live sports prediction odds**: Real-time probability lines, odds badges, and team win-rate split bars on sports cards and hero matchups via market plugins like Kalshi.
- **Ambient screensaver & 3 new themes**: Living-room idle screensaver with clock overlay and instant wake, paired with Aurora, Synthwave, and Biolume visual themes.
- **Display Box TV Remote & aggregated search**: Authenticated REST API routes for external phone remotes and parallel search across Cinemeta, YouTube, sports, and watch history.
- **Plugin developer platform & runtime diagnostics**: Sandboxed plugin runtime, developer SDK, live runtime log inspection in Settings, and sports badge contribution APIs.

### Detailed Changes

#### Added

- **Living-room setup**: Choose a theme, pair a phone remote, connect Stremio and Spotify, and follow sports leagues or teams before opening Home.
- **Quick menu and sign-in startup**: Open the controller binding guide, sleep the PC, switch displays, or exit Luma from the top bar. Windows sign-in can open Luma fullscreen on the selected TV.
- **Keep launcher ready**: Off-by-default System setting that retains a blank browser runtime while Luma sits in the tray, so reopening the launcher from the tray is faster. Turning it off closes the retained runtime.

- **Widget studio**: Place widgets using layout presets, customize sizes (compact, standard, expanded) and visual styles (glass, solid, minimal), configure presence triggers (always, music player, or idle-only), and add multiple widget instances with live preview across Home, the player HUD, and idle screens.
- **Clock widget**: Built-in customizable Clock widget supporting 12/24-hour time, optional seconds and date display, custom timezone offsets, and opt-in display inside the music player HUD.
- **Ambient screensaver**: Living-room idle screensaver with configurable timeout (1–30 minutes), clock overlay, and smooth visual backdrops that instantly wake on any remote, keyboard, or controller input.
- **New themes**: Added Aurora, Synthwave, and Biolume themes to Luma's built-in theme presets, featuring rich ambient gradients and accent styling across the launcher and web remote.
- **Game controller navigation & settings**: Full directional and action gamepad navigation for Xbox, PlayStation, Nintendo, and generic controllers across the launcher, with customizable button bindings, brand detection, and an interactive controller diagram in Settings.
- **XInput background home button**: Dedicated background listener for controller guide / home button input via Windows XInput, restoring and focusing Luma even while external games or applications are running in fullscreen.
- **Steam non-Steam game shortcuts**: Read, add, remove, and launch non-Steam shortcuts directly within Luma via Steam's binary `shortcuts.vdf` format, complete with automatic app ID calculation and artwork resolution.
- **Display Box TV Remote routes**: Frame-authenticated `/display/*` REST API routes for remote playback control, settings synchronization, app dock ordering, plugin panels, panel action execution, and Spotify album artwork.
- **Aggregated display search**: Unified search endpoint aggregating results across Cinemeta movies and series, YouTube videos, live sports streams (StreamEast and Sportsurge), and local watch history in parallel with bounded timeouts.
- **Expanded sports & prediction markets**: Live scores, game cards, and streaming shortcuts for College Football (CFB), NCAA Men's Basketball, NCAA Women's Basketball, and the WNBA, alongside live game badge contributions from plugins like Kalshi sports prediction markets.
- **Live sports prediction odds**: Event cards, the featured hero matchup, and game detail dialogs display live probability odds and visual comparison bars contributed by plugin badges (such as Kalshi prediction markets) alongside live scores and stream links.
- **Plugin runtime log inspection**: View real-time plugin stdout, stderr, and lifecycle diagnostic logs directly within Settings to troubleshoot and verify sandboxed background and UI plugins.
- **Plugin SDK sports event badges**: Added `luma.sports.onEventBadges()` and `luma.sports.setBadges()` to the Plugin SDK, enabling plugins to contribute live game badges, odds, and metadata.

#### Changed

- Reorganized Settings into 8 TV-first categories (Appearance, Home, Display & Sound, Apps & Accounts, Controls, Watch History, System, and Add-ons) with controller bumper category cycling and dedicated subpanel back navigation.
- Overhauled the Widget Studio in Settings with a responsive container layout, collapsible per-widget options, and cleaner placement selectors.
- Realigned the web phone remote settings to mirror the TV launcher categories (Appearance, Home, Display & Sound) with simplified living-room descriptions and shared app dock ordering directly under Home.
- Widgets sharing a corner or rail stack in order rather than overlapping.
- Widget settings store presentation per instance (`type`, placement, size, look, presence, order, and widget-specific settings), decoupling widget definitions from rigid Rust and MCP schemas.
- Added smooth app hover animation toggle in Appearance settings.
- Screensaver visuals support reduced motion preferences, smooth backdrop transitions, and refined clock typography.
- Activity history automatically prunes records older than 90 days or exceeding 500 items while preserving installed application entries, followed by automatic WAL checkpoint truncation.
- Refreshed Stremio library rows and cards with item count badges and refined layout styling.
- Updated plugin development guides, developer tooling prompts, and MCP agent instructions to match the new `Settings > Add-ons > Plugins` location.

#### Fixed

- Settings subpanels cleanly trap back navigation and preserve previous focus when closing nested editors.
- Sports badge subscriptions automatically clean up when unmounting or leaving the Sports screen to prevent background listener leaks.

## [0.9.0] - 2026-11-20

### Added

- **Plugin Developer Platform & SDK**: Build custom plugins for Luma using the new `@luma/plugin-sdk`, `luma-plugin` CLI, and starter templates for TypeScript and Svelte 5. Test live with an isolated dev host, DevTools inspection, hot reloading, and automated manifest validation.
- **Executable Plugin Runtime & Remote Panels**: Run isolated-ui and background plugins under sandboxed capability grants and supervision. Plugins can declare remote commands and host-rendered panels directly on the web phone remote for interactive living-room control.
- **MCP Plugin Development Tools**: AI coding agents can manage plugin dev sessions, reload code, inspect state, and test command invocations through dedicated MCP developer tools.
- **Progressive Catalog Streaming**: Home screen catalog rows now stream in progressively as each source responds, avoiding long blocking spinners.

### Changed

- The Search tab loads popular shows lazily on demand, and poster images use responsive srcsets to save bandwidth and memory.
- Remote API requests use bounded timeouts (15s standard, 30s search) with automatic recovery to keep the phone remote responsive during network drops.
- Activity synchronization safely isolates settings updates, avoids clearing resume progress prematurely, and fetches Plex and Jellyfin libraries concurrently.

### Fixed

- Single-mode viewing arrangement and external window detachment properly manage window state and restore window placement.

## [0.8.0] - 2026-11-15

### Added

- **Multi Watch** puts up to two players on screen side by side, on separate displays, or in a floating window that stays above Luma while you browse. Add a Luma source or any open app or browser window, choose which player the remote controls, and remove a window to restore its previous placement.
- Move widgets anywhere on Home: drag the Spotify bar with a pointer or the remote, or pin it to a corner. Widgets can always show, appear only when Spotify is available, or appear only during playback, and custom positions stay on screen when the display size changes.
- Search on the phone remote covers Movies, Shows, YouTube, Sports, and History in source tabs, with result counts and live sports cards. Results open on the display from the same search.
- Show posters use TVmaze artwork when Stremio artwork is missing, and cards show a clear placeholder instead of a broken image when nothing is available.

### Changed

- Focus rings use Luma's accent color across cards and controls, and settings categories have their own icons.
- Press Ctrl+Alt+M, or use **Settings → Display & Sound → Switch display**, to move Luma and its current layout to the next monitor without changing the saved startup display.

## [0.7.0] - 2026-11-15

### Added

- Watch a league game from Home: the sports hero offers **Watch live** or **Watch game**, opens the matching stream, and keeps the score updating while the game plays. Opening the league itself stays available as a secondary action.
- The pairing screen shows the PC's other local addresses when more than one is available.

### Changed

- Luma pauses its Spotify polling and activity progress while its window is hidden or minimized, then resumes when it returns.
- Pairing prefers an adapter with internet access and ignores virtual adapters from Hyper-V, VirtualBox, VMware, and Docker.

### Fixed

- Appearance and Home settings no longer report unsaved changes when the stored document differs only in key order.

## [0.6.1] - 2026-11-15

### Added

- Declarative plugins: import, preview, enable, disable, export, and remove `.luma-extension.zip` packages from **Settings → Plugins**, with bounded plugin settings and clear handling when a plugin changes a setting it owns.
- AI customization: **Settings → MCP customization** turns on a local MCP endpoint where an agent can read, validate, and preview changes, and stage plugins, alongside ten saved versions, restore, and reset. An opt-in local REST API offers the same reach for scripted changes.
- A rebuilt web remote with QR pairing, add-to-home-screen guidance and icons, and touch-friendly settings that follow Luma's own settings payload.
- Fast app switching keeps YouTube and Stremio loaded in the background so they reopen without reloading.
- Sports with scores for the NBA, NFL, MLB, and NHL, and StreamEast listings beside Sportsurge sources with automatic fallback.
- Sports shortcuts in the app catalog and phone remote that open the league's games.
- A music strip on the web remote and the Display Box TV Remote tab with album art, play/pause, previous, and next, controlling Spotify on the PC without disturbing the movie.

### Changed

- Music keeps playing while a movie or Stremio runs, instead of closing the video window when Spotify opens.
- Play/pause, previous, and next follow the window in front as before; the new music controls always target Spotify.
- Phone remote settings are grouped into Appearance & Home, Display & Sound, and Apps & Services, with dock ordering and PC window actions in place, and clearer touch targets, switches, and dialogs.
- Split Settings into **Appearance & Home** for Luma's own theme, Home layout, and widgets, and a separate **Plugins** category for importing, exporting, and managing plugins. Plugins sits above MCP customization.
- Home renders through a registry-driven renderer, and existing Home layouts migrate automatically.
- Import and export reject oversized or mispackaged plugin archives instead of failing partway through.

### Fixed

- Plugin errors offer a retry action, name detached fields by their plugin setting, and stop repeating the same message twice.
- Leftover plugin staging files are cleaned up, and invalid plugin contributions are repaired rather than left blocking settings.
- Sports playback keeps stream selection, source fallback, and progress consistent across providers.

## [0.4.0] - 2026-11-14

### Added

- Change Luma's theme, app tile shape, layout density, sound set, and sound level from the phone remote, with settings synchronized with the TV app.
- Separate phone settings into This remote and Luma app tabs, with save feedback and rollback on failure.

### Changed

- Refreshed phone remote styling, mobile settings layout, and keyboard focus indicators.

### Fixed

- Allow same-origin connections in the app's Content Security Policy.

## [0.3.0] - 2026-11-14

### Added

- Steam Big Picture integration with remote controls, recent games, game launching, store actions, recommendations, and header artwork.
- Coordinated themes across Luma and the web remote, with an optional local theme override on the phone.
- Home layout editor with live preview, reorderable and hideable rows, a movable featured hero, and reset.
- Spotify now-playing widget with bottom-bar and screen-corner placement.
- UI sound presets, remote sound playback, and volume feedback.
- Shared app dock ordering across Luma, the web remote, and Display Box, including drag-and-drop and remote-friendly ordering controls.

### Changed

- Create the launcher webview on demand and unload embedded YouTube and Stremio webviews when leaving them.
- Refreshed Steam game cards, Settings header, and app-order controls with live previews.
- Tray exit choices support prompting, hiding to the tray, or quitting.
- Stream appearance changes to the phone remote and deduplicate remote state updates.

### Fixed

- Prevent stale remote state and app responses with no-store caching headers.
- Correct YouTube gamepad polling on focus changes and clean up sound resources.

## [0.1.0] - 2026-11-14

### Added

- TV-first launcher with 10-foot typography, accent colors, wallpapers, and navigation from a keyboard, remote, or phone.
- App dock for Netflix, Disney+, Prime Video, Apple TV+, Hulu, Max, Peacock, YouTube, Spotify, Stremio, and a custom app, with Windows app detection and fullscreen browser fallbacks.
- Movie catalog browsing and search from Cinemeta, with title details and Stremio playback.
- Spotify account connection with transport controls, playlists, saved albums, liked songs, search, and device transfer.
- Stremio account linking with library sync, Continue Watching, and an embedded Stremio Web window.
- Unified activity history in local SQLite: Continue Watching, recently played, and recent apps, plus Plex, Jellyfin, and Trakt imports.
- Local remote server with a browser remote and Display Box frame pairing, app catalog, and control of launched applications.
- Signed in-app updates with automatic checks, release notes, download progress, and restart to install.
- System tray, optional Windows startup, single-instance guard, VPN detection, and TV display and audio output selection.
- Secure credential storage in Windows Credential Manager.

### Changed

- Consolidated the local remote server on one port and moved remote credentials to Windows Credential Manager.
- Redesigned Home around a slide-based hero, backdrop catalog rows, and a refreshed mini player.

### Fixed

- Window handoff focus between Luma and launched applications, and Spotify browsing stability.
