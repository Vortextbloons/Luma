# Changelog

Notable changes to Luma, newest first. Releases publish the matching section as the GitHub release notes and in-app update notes.

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
