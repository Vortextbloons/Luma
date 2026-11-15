# Changelog

Notable changes to Luma, newest first. The Release workflow publishes the matching section as the GitHub release notes and in-app update notes.

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
