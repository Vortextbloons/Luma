<p align="center">
  <img src="docs/branding/luma-logo.png" width="112" alt="Luma logo" />
</p>

<h1 align="center">Luma TV</h1>

<p align="center">
  <strong>Your entertainment, together.</strong><br />
  A remote-first home screen that turns a Windows PC into the heart of the living room.
</p>

<p align="center">
  <img alt="Windows 10 and 11" src="https://img.shields.io/badge/Windows-10%20%7C%2011-0b0d12?style=flat-square&logo=windows11&logoColor=white" />
  <img alt="Tauri 2" src="https://img.shields.io/badge/Tauri-2-24C8DB?style=flat-square&logo=tauri&logoColor=white" />
  <img alt="Svelte 5" src="https://img.shields.io/badge/Svelte-5-FF3E00?style=flat-square&logo=svelte&logoColor=white" />
  <img alt="Rust" src="https://img.shields.io/badge/Rust-stable-000000?style=flat-square&logo=rust&logoColor=white" />
</p>

<p align="center">
  <img src="docs/screenshots/home.webp" width="100%" alt="The Luma home screen, with Continue Watching, the app dock, and rows of movie posters" />
  <br />
  <em>Everything you watch and play, one screen away.</em>
</p>

## One screen for everything

Luma is a TV launcher for Windows. It doesn't sell you another subscription or ask you to rebuild your library somewhere else — it takes the apps and services already on your PC and puts them behind one big, calm, remote-friendly screen.

Launch Netflix, Disney+, Prime Video, Stremio or YouTube. Pick a film from a live catalog. Jump back into something you started on Plex or Jellyfin. Put on an album in Spotify. Then drive it all with the arrow keys on a couch keyboard, or with the phone already in your hand.

## What it's trying to do

- **One calm place.** Your apps, films, shows and music belong together, not scattered across a dozen windows and browser tabs.
- **Built for the couch.** Big artwork, clear focus, useful feedback, and a complete journey using only arrow keys, Enter and Back — from across the room.
- **Use what you already have.** No new account is required, no second watch history is invented, and no streaming engine is bundled. Luma connects to the real services you use.
- **Yours stays yours.** History is kept locally, credentials stay in Windows Credential Manager, and the remote never leaves your own network.
- **Quiet in the background.** It starts with Windows if you want, waits in the tray, and gets out of the way when you're watching something.

## Features

| | |
| --- | --- |
| 🕘 **Continue Watching** | One row of unfinished films, episodes and tracks from Luma launches, YouTube, Spotify, Plex, Jellyfin and Trakt — plus a dedicated Stremio row when an account is linked. |
| 📺 **Every app, one dock** | Netflix, Disney+, Prime Video, Apple TV+, Hulu, Max, Peacock, YouTube, Spotify, Stremio and a custom tile. Installed apps are found automatically, and services without a desktop app open in a clean fullscreen window. |
| 🎬 **Browse without opening anything** | Popular and top-rated movies from Stremio's public Cinemeta catalog, with live posters, ratings, runtimes, descriptions and search. |
| 🍿 **Stremio, built in** | Link an account with a short device code, sync its library and Continue Watching, and open any title in the official Stremio Web player inside Luma. |
| 🎧 **Music from the sofa** | Browse playlists, saved albums and liked songs, search, switch playback devices, and control Spotify without leaving the big screen. |
| 📱 **A remote in every pocket** | Pair any phone on the same Wi-Fi with a six-digit code for a full D-pad, playback, volume and app shortcuts. A Display Box photo frame can pair alongside it. |
| 🕹️ **Control the apps you launched** | When a launched app is in front, the remote moves its pointer and clicks inside it — then Home brings Luma back. |
| ✨ **Made for the biggest screen** | A 10-foot interface with four accent colors, your own wallpaper, comfortable or compact density, and separate TV screen and audio outputs. |
| 🔒 **Local and quiet** | Watch history lives in a local SQLite file, tokens stay in Windows Credential Manager, and the remote is LAN-only by design. |
| 🌙 **Everyday comfort** | System tray, optional startup with Windows, single-instance guard, and signed in-app updates. |

## A closer look

<p align="center">
  <img src="docs/screenshots/details.webp" width="49%" alt="A movie details dialog with artwork, description, rating and an Open in Stremio action" />
  <img src="docs/screenshots/settings.webp" width="49%" alt="Appearance settings with accent colors, wallpaper and home row options" />
</p>

<p align="center">
  <em>Rich title details and a picture-perfect home are only a couple of arrow presses apart.</em>
</p>

## Run it

Requires Node.js LTS, Rust stable, and the standard Tauri Windows prerequisites (WebView2 and MSVC build tools).

```sh
npm install
npm run tauri dev
```

`npm run dev` opens a browser preview of the interface, but app launching, the remote, TV setup and music controls need the desktop app. `npm run tauri build` produces the Windows installer.

## Built with

[Tauri 2](https://tauri.app) · [Rust](https://www.rust-lang.org) · [Svelte 5](https://svelte.dev) · [TypeScript](https://www.typescriptlang.org) · [Tailwind CSS 4](https://tailwindcss.com)

## Good to know

- Luma launches apps and opens web services; it does not stream or host video itself. DRM-protected playback stays in the app that provides it.
- Torrent and transcoded Stremio streams need Stremio Service, which Luma can use if it is running on the same PC.
- The phone remote is served over plain HTTP for your private network. Use it on a network you trust, and allow Luma through Windows Firewall on private networks.
- Movie metadata and artwork come from Stremio's public Cinemeta and MetaHub catalogs. Streaming service icons remain the trademarks of their respective owners.

---

<p align="center">
  Looking for architecture, the remote protocol, release steps or validation notes?<br />
  See <a href="docs/DEVELOPMENT.md">docs/DEVELOPMENT.md</a>.
</p>
