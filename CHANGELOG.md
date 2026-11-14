# Changelog

Notable changes to Luma, newest first. The Release workflow publishes the matching section as the GitHub release notes and in-app update notes.

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
