# Luma plugin development

The `luma-plugin` CLI builds, validates, packs, and runs an executable plugin package in a disposable Luma instance. The shipped plugin contract remains in [PLUGINS.md](PLUGINS.md); [PLUGIN-DEVELOPER-PLATFORM-SPEC.md](PLUGIN-DEVELOPER-PLATFORM-SPEC.md) describes the wider platform plan.

## Setup

Install Rust, Node.js LTS, and the Windows Tauri prerequisites. In the Luma checkout:

```powershell
npm install
npm link
```

`npm link` puts `luma-plugin` on your PATH. Without it, use `npm run plugin -- <command>` from the checkout.

Scaffold a plugin or start from the Weather example:

```powershell
luma-plugin init ..\my-luma-plugin
luma-plugin init ..\luma-weather --template weather
```

The TypeScript starter includes a build script, the editor manifest mapping, and a VS Code launch configuration. Install its dependencies with `npm install` in the new project.

## Validate and pack

Validate the built package directory or an existing ZIP:

```powershell
luma-plugin validate dist
luma-plugin validate dist --json
```

Pack the same directory into the distribution ZIP:

```powershell
luma-plugin pack dist
luma-plugin pack dist --out ..\releases
```

The output name is `<id>-<version>.luma-extension.zip`. Diagnostics name the manifest field or the package-relative file that failed. Validation runs through the same Rust manifest and archive rules as Settings import and MCP staging, so the three paths agree on file checks, limits, and cross-field constraints. The editor JSON Schema covers completion only; Rust remains authoritative.

## Run a development session

```powershell
luma-plugin dev
```

By default the CLI builds the current project, validates `dist`, starts a second Luma instance, and watches the project. The default host command is `npm run tauri dev` in the Luma checkout. Point the CLI at another checkout with `--luma`, or supply your own launch command with `--host`.

While the session runs:

- `r` rebuilds and reloads now.
- `d` opens DevTools for the active plugin surface.
- `s` prints activation state, granted access, app data, and the last error.
- `q` stops the development host. Ctrl+C does the same.

Build and validation errors appear in the terminal. The last working snapshot stays active after a failed build or validation. Successful reloads replace the active snapshot, restart the old runtime, and keep compatible setting values.

The development window title starts with **Luma Development**. The session uses its own app data, plugin settings, grants, and identity under the system temporary directory. It does not start the normal tray, autostart, updater checks, web/phone remote listeners, or MCP server. The normal Luma instance can keep running.

The first package loaded into a session is approved with exactly the access it declares. A later reload that changes permissions or network origins disables the package and requires review in the development window under **Settings > Plugins**. A package ID change stops the previous runtime, revokes its grant, and requires review for the new package. The previous package remains installed and disabled in the session. Reloads that keep the same identity and access reuse the existing approval. Same-version reloads are supported in the disposable profile. Settings import requires a version increment for changed content.

Only executable `isolated-ui` and `background` packages run in the development host. Declarative packages can be validated and packed, and continue to install through Settings.

## Project configuration

The CLI reads `plugin.dev.json` from the project root, then `package.json` under `lumaPlugin`:

```json
{
  "build": "npm run build",
  "out": "dist",
  "host": "npm run tauri dev"
}
```

Command-line flags override the file. `--no-build` uses the existing output, `--no-watch` disables the reload loop, and `--no-launch` validates and watches without starting Luma.

## Debugging and output

The starter VS Code configuration runs the CLI under the Node debugger in the integrated terminal. Build output, validation diagnostics, and host logs stream there. Host logs are prefixed with `[luma-dev]` and include the plugin ID, activation state, and bridge operation. Runtime bridge errors omit request arguments and values.

To debug the Rust side, start `luma-plugin dev` and attach your debugger to the running `luma.exe` process. The CLI does not own the debugger connection, so attach and detach while the session stays running. Use `d` for the plugin surface's own WebView DevTools. DevTools require a debug build.
