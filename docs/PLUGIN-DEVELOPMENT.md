# Luma plugin development

Use Luma MCP to author plugins with an agent. The `luma-plugin` CLI provides the same package checks and a disposable development instance for local terminal and VS Code workflows. See [PLUGINS.md](PLUGINS.md) for the package contract and [PLUGIN-DEVELOPER-PLATFORM-SPEC.md](PLUGIN-DEVELOPER-PLATFORM-SPEC.md) for the wider plan.

## Develop with an MCP agent

1. Read `luma_capabilities_get` and the plugin resources in the MCP resource list. Choose a complete example and give your draft a stable plugin ID.
2. Call `luma_plugin_validate` with the manifest and built package files. A coding agent can edit and build its sources in its own workspace and send the resulting files to Luma.
3. For an executable draft, call `luma_plugin_dev_start` with the validated package. Review and enable it in **Settings → Plugins** in the development window. The development profile has its own settings and grants. An approval there applies to that session.
4. For packages with commands, call `luma_plugin_dev_invoke` with a declared command. Use `luma_plugin_dev_status` to inspect readiness and diagnostics. When a plugin reports its feature outcome, match `commandRun.invocationId` to the ID returned by invoke and read `commandRun.outcome`. Weather reports whether the forecast loaded.
5. Edit and validate the package again, then call `luma_plugin_dev_reload` for the same session. A failed validation leaves the last working draft available. A change to requested access requires another review.
6. Call `luma_plugin_stage` with the final package and `luma_plugin_dev_stop` to close the development instance. Review and enable the final package in your everyday launcher. Declarative packages can go straight from validation to staging and preview.

An example request to an agent is “Build a Weather Home card from the Luma MCP template. Open a development session for me to review its access, test the forecast command, then stage the working package for approval.”

Use `luma://plugins/templates/svelte` for a standalone source project with Svelte, Tailwind, TypeScript, and the SDK. Write its files into a coding workspace, run its install and build commands there, then send the built package in `dist` to Luma. The declarative, Weather, and phone-command templates return packages ready for MCP submission without a build step.

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
luma-plugin init ..\my-svelte-plugin --template svelte
luma-plugin init ..\luma-weather --template weather
```

The recommended Svelte starter (`starters/svelte`) pairs Svelte 5 runes, Tailwind CSS, TypeScript, and Vite. The plain TypeScript starter (`starters/typescript`) uses esbuild. Both starters include local SDK/schema vendoring for out-of-checkout projects, an editor manifest mapping, and a VS Code launch configuration. Install dependencies with `npm install` in the new project.

### Source template build and file boundaries

Compiled source templates (`starters/svelte` and `examples/plugins/com.example.weather-svelte`) distinguish between development sources and the validated distribution package:

* **Source project root**: Holds source files, build config, and dependencies (`src/`, `ui/`, `vite.config.ts`, `svelte.config.js`, `package.json`, `tsconfig.json`, `luma-extension.json`).
* **Build boundary**: `npm run build` runs Vite to emit module scripts and extracted stylesheets into `dist/`, along with `dist/ui/index.html` and `dist/luma-extension.json`.
* **Package boundary**: The `dist/` directory forms the self-contained static package passed to `luma-plugin validate`, `luma-plugin pack`, `luma-plugin dev`, or MCP staging.
* **Shell-free templates**: Plain JavaScript packages like `examples/plugins/com.example.weather` require no build tooling and remain directly runnable from source.

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

The CLI approves the first package in its own development session with exactly the access it declares. An MCP development session starts with the draft disabled for user review. A later reload that changes permissions or network origins disables the package and requires review in the development window under **Settings → Plugins**. A package ID change stops the previous runtime, revokes its grant, and requires review for the new package. The previous package remains installed and disabled in the session. Reloads that keep the same identity and access reuse the existing approval. Same-version reloads are supported in the disposable profile. Settings import requires a version increment for changed content.

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
