# Luma plugins

Status (2026-11-20): declarative packages, explicit executable grants, isolated UI and background supervision, API v3 Home and paired-phone command panels, and subscribed settings/playback notifications are implemented. The real WebView2 harness verifies core lifecycle, command denial, CSP denial and cleanup; interactive TV/phone/tray and resource-budget acceptance remain open. This file owns the implemented package/API contract; [the runtime record](CUSTOMIZATION-RUNTIME-SPIKE.md) owns validation evidence and remaining limits.

## What ships

Luma installs, validates, enables, disables, exports, and removes plugin packages. Declarative packages use Preview and Keep; executable packages require a separate access review. A plugin can contribute appearance and Home layout, and can declare bounded settings that control those contributions.

Import never runs code or install hooks. API v2 `isolated-ui` and `background` packages and API v3 Home card packages can run packaged HTML/JavaScript after the user approves their declared access in Settings. Native code and the `main-page` runtime remain unsupported. See the [development workflow](PLUGIN-DEVELOPMENT.md) and [typed SDK](PLUGIN-SDK.md) to build and debug packages.

Luma's own Theme, Home layout, and widgets are built into the app under **Settings → Appearance & Home**. They are not a plugin: they cannot be removed, and they stay usable when another plugin is disabled or broken. The old `Theme`, `Layout`, `Home layout`, and `Widgets` entry points resolve to those sections.

**Settings → Add-ons → Plugins** imports, previews, enables, disables, exports, and removes packages. AI access is a separate page under Add-ons. Saved versions and reset are under Appearance.

Luma also ships built-in packages. They use the same manifest, contribution, ownership, preview, and disable path, but are installed from Luma's own code and can be switched on or off directly. Built-in packages cannot be removed. `com.luma.apple-tv-look` is an off-by-default declarative appearance pack.

## Package format

A package is an ordinary ZIP named `<id>-<version>.luma-extension.zip`. `luma-extension.json` must be at the archive root or inside one top-level folder. The archive may contain readable source, assets, and a manifest-declared runtime entry. Do not include credentials, machine paths, user history, caches, or runtime storage.

The recipient chooses **Import plugin** and selects the ZIP. Luma copies the package into its application-data directory as `plugins/<id>/<version>`; the shared ZIP can then be moved or deleted. Import does not execute anything.

Export writes the installed package files and manifest to a new ZIP through a native Save dialog. It does not include user plugin setting values, Luma history, accounts, credentials, machine paths, caches, or runtime storage. Declared setting defaults and the settings schema travel in the manifest.

## Manifest

```json
{
  "id": "com.example.cinema-lounge",
  "name": "Cinema Lounge",
  "version": "1.0.0",
  "apiVersion": 1,
  "description": "Warm two-column Home with a controllable featured row.",
  "author": "Example",
  "settings": [
    {
      "key": "theme",
      "label": "Theme",
      "description": "Palette used by the pack.",
      "type": "enum",
      "default": "ember",
      "options": [
        { "value": "ember", "label": "Ember" },
        { "value": "onyx", "label": "Onyx" }
      ]
    },
    {
      "key": "showFeatured",
      "label": "Show featured movies",
      "type": "boolean",
      "default": true
    },
    {
      "key": "columns",
      "label": "Home columns",
      "type": "number",
      "default": 2,
      "min": 1,
      "max": 2,
      "integer": true
    }
  ],
  "contributes": {
    "theme": { "setting": "theme" },
    "homeColumns": { "setting": "columns" },
    "home": {
      "rows": {
        "featured": { "visible": { "setting": "showFeatured" } }
      }
    }
  },
  "runtime": { "kind": "declarative" },
  "assets": [],
  "source": []
}
```

Required fields are `id`, `name`, `version`, and `apiVersion`. API v1 defaults to a declarative runtime and requires appearance/Home contributions or a `remote` panel. Executable packages require `runtime.kind` and `runtime.entry`. `description`, `author`, `settings`, `assets`, and `source` default to empty. Unknown fields are rejected.

- `id` is a lowercase vendor-qualified id such as `com.example.pack`, 3–96 characters. `core` is reserved. Plugin ids are case-sensitive in storage and lowercase-only in validation.
- `version` is numeric `major.minor.patch` without leading zeros.
- API v1 is declarative. API v2 supports standalone isolated UI or background packages. API v3 supports on-demand commands and Home cards.
- `settings` declares at most 24 host-rendered values. Keys match `[a-z][A-Za-z0-9_-]{0,63}`. Types are `boolean`, `enum`, `number`, and `string`:
  - `boolean` default must be true or false.
  - `enum` requires 1–20 unique string options and a default in those options.
  - `number` accepts finite numbers, optional `min`/`max`, and optional `integer: true`.
  - `string` accepts optional `minLength`/`maxLength` (maximum 500).
- `assets` and `source` are package-relative paths that must exist. `runtime.entry` is required for non-declarative runtimes and must exist.
- `contributes` must declare at least one supported field for a declarative plugin.

### API v2 executable package

`apiVersion: 2` supports `runtime.kind: "isolated-ui"` and `"background"`. Packages install disabled and require user access review before execution. The public shape is:

```json
{
  "id": "com.example.weather",
  "name": "Weather",
  "version": "1.0.0",
  "apiVersion": 2,
  "runtime": { "kind": "isolated-ui", "entry": "ui/index.html" },
  "permissions": ["settings.read", "network.get"],
  "network": { "allow": ["https://api.open-meteo.com"] },
  "settings": [
    {
      "key": "location",
      "label": "Location",
      "type": "enum",
      "default": "seattle",
      "options": [{ "value": "seattle", "label": "Seattle" }]
    }
  ],
  "assets": ["ui/page.js", "ui/page.css"]
}
```

- `runtime.entry` is a package-relative HTML file. An executable package cannot declare declarative contributions. API v2 executable packages cannot declare remote panels; API v3 may declare command panels.
- Permissions are exact names: `settings.read`, `network.get`, `playback.read`, and `playback.control`. Duplicates, unknown names, and grants on `apiVersion: 1` are rejected.
- `network.get` requires `network.allow` with 1–8 HTTPS origins. Origins cannot contain a path, port, credentials, IP literal, loopback, or a private-network style host name. The package still cannot fetch directly; every request is bounded and host-mediated.
- Settings are host-rendered, host-validated, and stored in the existing plugin settings table. A package can read only its own resolved values.

### API v3 Home cards and commands

An `isolated-ui` package can declare up to eight commands and eight static Home cards. Set `apiVersion` to `3` and set `minHostVersion` to the oldest Luma version that supports the package. A readable package that requires a newer host installs disabled and shows a compatibility reason.

```json
{
  "commands": ["openForecast"],
  "cards": [{ "id": "forecast", "title": "Weather", "command": "openForecast" }]
}
```

Place these fields in `contributes`. IDs are unique within each list, start with a lowercase ASCII letter, and contain up to 32 ASCII letters, digits, hyphens, or underscores. A card title has 1–48 characters after trimming. Each card references a command from the same package. Luma renders the card only while the package is enabled under a current on-demand grant. Selecting it starts the isolated surface and delivers the command after `surface.ready`. See the [SDK command contract](PLUGIN-SDK.md#commands).

The [Weather example](../examples/plugins/com.example.weather) uses API v3, two settings, and a single network origin. A Svelte 5 + Tailwind equivalent is available in [com.example.weather-svelte](../examples/plugins/com.example.weather-svelte).

### API v3 phone commands

An API v3 `isolated-ui` package may also declare up to three commands for its paired-phone panel:

```json
{
  "title": "Weather controls",
  "commands": [
    { "command": "openForecast", "label": "Refresh forecast" }
  ]
}
```

Place this in `remote`. Every command must be declared in `contributes.commands` by the same package, and labels are host-rendered text of 1–32 characters. API v3 panels cannot declare `remote.actions`; playback controls use a command that calls the playback bridge. The paired remote shows the panel only while the package is enabled with a current on-demand grant. Choosing a control rechecks pairing, the current panel, the declared command, the installed version, and the grant. The host then starts a hidden runtime, waits for readiness, and delivers the command without opening or focusing a desktop surface. The phone receives `pending`, `success`, or `failure` in the action response; `success` means the command was delivered, not that the package finished its work. No package HTML or JavaScript runs on the phone. A hidden session ends with the normal on-demand lifecycle: `surface.close`, Disable, update, removal, hiding the launcher, or quitting Luma.

### Contribution fields

A contribution is either a static value or `{ "setting": "key" }`. Bindings are declarative; there are no expressions or eval.

| Field | Value | Setting binding |
| --- | --- | --- |
| `theme` | built-in theme id | enum whose options are all built-in themes |
| `appearance` | validated palette and design tokens | static only |
| `density` | `comfortable` or `compact` | enum with supported options |
| `appTileShape` | `wide` or `square` | enum with supported options |
| `textSize` | `standard`, `large`, or `larger` | enum with supported options |
| `reducedMotion` | boolean | boolean |
| `homeColumns` | 1 or 2 | integer number limited to 1–2 |
| `home.order` | all seven components once | static only |
| `home.rows.<component>.visible` | boolean | boolean |
| `home.rows.<component>.span` | 1 or 2 | integer number limited to 1–2 |
| `widgets.spotifyEnabled` | boolean | boolean |
| `widgets.spotifyPlacement` | `bar`, `top-left`, `top-right`, `bottom-right`, or `bottom-left` | enum with supported options |
| `order` | all twelve app ids once, `Custom` last | static only |

An `appearance` contribution supplies a complete or partial palette over the host's 20 semantic color tokens plus bounded design tokens: `font` (`system`, `apple`, `rounded`), `nav` (`top`, `sidebar`), `surface` (`solid`, `frosted`), `radius` (`standard`, `generous`), `background` (`ambient`, `plain`), `heroTitle` (`normal`, `spaced`), and `tiles` (`tinted`, `neutral`). Tokens are validated colors; arbitrary CSS and scripts are not accepted. The host applies the same tokens and shell styling used by built-in themes, so disabled packs restore the previous appearance without restarting.

A plugin owns every field it contributes. Two enabled plugins cannot own the same field. Enabling the second one fails with an explicit conflict naming the field and current owner. Disabled plugins may be installed with overlapping contributions; the conflict only blocks enabling.

If a user changes a field owned by an enabled plugin, Luma keeps their input, stores it in the underlying configuration, and detaches that field from the plugin. The pre-Keep review names the fields that will detach, and the plugin detail shows the detached controls with a **Preview and restore controls** action. Disabling a plugin clears its detached state so a later enable applies its full declared contribution.

## Lifecycle

1. **Import or MCP stage** validates the manifest and files, computes a SHA-256 content hash, installs the package under Luma-owned storage, and records it as disabled.
2. **Preview and enable** is a user action in Settings → Add-ons → Plugins. Luma stages the effective appearance and Home candidate and shows the normal review overlay. The user presses **Keep and enable**.
3. **Disable** removes the plugin's contributions, stops its runtime, and revokes executable grants. The base appearance, Home, saved settings, and other plugins remain.
4. **Update** requires a new version. Re-importing the same id and version with identical content is idempotent; changed content under the same version is rejected with `immutable_version`. A new version replaces the package, preserves declared setting values, and stays disabled until the user reviews it again. MCP staging refuses to replace an enabled package; the user must disable it in Settings first. Native Import may update it and disables it atomically for review.
5. **Remove** deletes the package code and grants, stops its contributions/runtime, and writes a history checkpoint. Settings and local plugin data are kept for reinstalling by default; the confirmation offers an explicit delete option. Earlier history entries that referenced the package restore with a visible "not installed" or version-changed explanation and do not pretend the plugin is enabled.

Installing or updating an enabled plugin disables it atomically; a failed install or update leaves the previous package, enabled state, settings, and rendered configuration intact.

## Settings

The manifest author writes the schema and defaults. Luma validates values on write and on load:

- Invalid stored values fall back to the declared default and are marked invalid in the plugin detail. Choosing a value and saving repairs the record.
- Settings can be edited through the host-rendered plugin detail even while the plugin is disabled. They persist across disable/enable and compatible updates.
- Settings staged through MCP are a preview candidate and require user Keep. The same values are used by the UI and MCP paths.
- Removing the plugin keeps its values unless the user selects the delete-data option. Reinstalling does not restore runtime grants. Export does not carry user values.
- Declarative settings bind only through declared contributions. Executable packages can read their own resolved settings through the approved `settings.read` bridge operation.
- Executable package settings use the same store and snapshot as declarative settings. They therefore appear in plugin listings and saved-version history; there is no per-setting private-value marker yet. The weather example stores a city choice, not coordinates or account data.

## Navigation and focus

- **Appearance & Home** is Luma's own Settings category with **Theme**, **Home layout**, and **Widgets**. It is not a plugin and cannot be removed.
- **Add-ons → Plugins** manages installed plugins.
- The old `Appearance & Home`, `Theme`, `Layout`, `Home layout`, and `Widgets` entry points open the matching Appearance & Home section.
- The Appearance & Home sections render the existing theme, layout, widget, preview, restore, and reset controls exactly once.
- **Add-ons → AI access** manages MCP connections.
- Back from a plugin review returns to Plugins. Cancel on a remove confirmation focuses Cancel. Import, action, busy, empty, error, disabled, and unsupported-runtime states are visible.
- A plugin failure does not remove the Appearance & Home sections, Settings, recovery, or the MCP connection.

## MCP

Resources:

- `luma://plugins/schema` — exact manifest, binding, runtime, and limit contract.
- `luma://plugins/example` — complete example manifest.
- `luma://customization/capabilities` — capability document including plugin support and limits.
- `luma://customization/guidance` — agent workflows for appearance changes and plugin development.
- `luma://plugins/templates/{declarative,weather,phone-command}` — complete, editable package submissions.
- `luma://plugins/templates/svelte` — source project with a local SDK. Build it in a coding workspace and submit the files from `dist`.

Tools:

| Tool | Behavior |
| --- | --- |
| `luma_plugins_list` | Installed plugins, enabled state, controls, setting schema/values, unsupported-runtime reason, effective config. |
| `luma_plugin_validate` | Validate manifest plus optional text/base64 files. No write. |
| `luma_plugin_stage` | Install the package disabled. Never enables, keeps, activates, executes, removes, or exports. |
| `luma_plugin_settings_stage` | Validate declared setting values and stage them as the single pending review. The user must press Keep. |
| `luma_plugin_dev_start` | Open a disposable instance with an executable draft disabled for user review. |
| `luma_plugin_dev_reload` | Validate and replace the active draft. Review changes to requested access before running again. |
| `luma_plugin_dev_invoke` | Deliver a declared command to the approved draft in its development instance. |
| `luma_plugin_dev_status` | Read the draft's access and runtime state, command delivery, and diagnostics. |
| `luma_plugin_dev_stop` | Close the disposable instance. |

Development uses its own app data and grants. You approve access inside the development window. Staging installs the final package disabled in the everyday launcher for separate approval. Staging settings requires a visible launcher for review. See [the agent development workflow](PLUGIN-DEVELOPMENT.md#develop-with-an-mcp-agent).

## Limits and archive checks

| Limit | Value |
| --- | --- |
| ZIP archive | 8 MiB |
| MCP submission payload | 2 MiB decoded |
| Entries | 512 |
| Files | 256 |
| Single file | 4 MiB |
| Total uncompressed | 16 MiB |
| Compression ratio | 200:1 |
| Manifest | 64 KiB |
| Settings per plugin | 24 |
| Options per enum setting | 20 |
| Declared assets/source entries | 64 each |
| Executable permissions | 8 |
| Network origins | 8 |
| Package path | 120 bytes |

Before activation, Luma rejects absolute paths, `..`, backslashes, drive/UNC paths, Windows-reserved names and characters, case-insensitive duplicate paths, file/directory collisions, symbolic links, executable binaries (`.exe`, `.dll`, `.msi`, `.bat`, `.cmd`, `.ps1`, `.scr`, `.com`, `.cpl`), missing declared files, oversized entries, too many entries, and suspicious compression ratios. Import never runs an unzip script or package install script.

## Executable lifecycle

In Settings > Add-ons > Plugins, choose **Review access**, review the package identity, exact permissions and network origins, and choose its trigger. **Allow and enable** persists an explicit grant. MCP development tools use the grant from the disposable instance. **Disable and revoke** stops the runtime and removes its grant while keeping settings.

- API v2 `isolated-ui` uses `launcher-open`. API v3 Home packages use `on-demand`. **Open** in Settings shows either desktop surface after `surface.ready`.
- `background` supports `launcher-open`, `startup`, and `always`. Launcher-open services stop when Luma hides to tray. Startup/always services can start without a launcher and continue in the tray until disabled or Luma exits.
- The bridge accepts `identity`, `settings.read`, `settings.subscribe`, `net.get`, `playback.read`, `playback.subscribe`, `playback.toggle`, `playback.next`, `playback.previous`, `surface.ready`, `surface.close`, and `commands.pending`. Host actions are bounded and checked against the installed package and its stored grant.
- Subscribed settings and playback notifications are pushed from host changes, not polled. Every push is a full snapshot under the matching read grant; duplicate snapshots are suppressed, and delivery stops when the session is revoked or stopped. See the [SDK notification contract](PLUGIN-SDK.md#settings-and-playback-notifications).
- Network requests use approved HTTPS origins, public-address resolution and address pinning, no redirects, bounded query values, and a 2 MiB JSON response cap under a 10-second timeout.
- The runtime has 16 registered isolation slots. Active sessions have distinct slot origins and fresh token paths; incognito WebView storage is ephemeral. Durable settings remain in SQLite.
- A ready timeout and bounded restart policy stop repeated failures: at most three restart attempts in ten minutes, followed by auto-disable with a visible reason.
- Update preserves declared settings but leaves the package disabled for review. Changed permission/network declarations clear the old grant. Incompatible packages remain installed and disabled rather than blocking startup.

The runtime uses a separate WebView2 window per active package, including a hidden window for a background service. It is not a separate native process sandbox. The live harness has verified bridge/CSP denials and lifecycle cleanup and measured startup/idle cost. Interactive focus, actual renderer crash events and an agreed resource budget remain open; see [the runtime record](CUSTOMIZATION-RUNTIME-SPIKE.md).

The tray preserves Open/Exit and adds one host-owned action per enabled executable package: Open for UI packages or Disable for background services.

## Web remote panels

A declarative API v1 package may declare `remote: { "title": "Music controls", "actions": ["playback.previous", "playback.toggle", "playback.next"] }`. Empty actions show playback only. Titles are limited to 48 characters and actions must be unique supported names. Panels appear in the paired web remote under **More > Plugin panels** only while their package is enabled. Every action rechecks the current declaration; no extension HTML or JavaScript is sent to the remote. Display Box has no plugin panel projection in its current protocol.

API v3 command panels use the same host-rendered foundation. The panel declares host labels and command references instead of playback actions. The web remote receives only the panel identity, title, label, and command identifier; it never receives package code, settings, or bridge credentials. v1 panels and playback-only panels keep their existing wire format.

The importable [music remote example](../examples/plugins/com.example.music-remote/luma-extension.json) proves the declarative panel shape. The [weather example](../examples/plugins/com.example.weather/luma-extension.json) uses the generic executable bridge and its own location/unit settings. Zip a package folder's contents, then import the ZIP through Settings > Add-ons > Plugins. For example:

```powershell
luma-plugin pack examples\plugins\com.example.weather --out $env:TEMP
```

## Shipped limits and verification

The native implementation is in `src-tauri/src/customization/plugin.rs`, `plugin/archive.rs`, `history.rs`, `mod.rs`, and `mcp.rs`. The Settings UI is in `PluginSettings.svelte`, `PluginOwnershipNotice.svelte`, `SettingsPanel.svelte`, and the `plugins.ts`/`settingsNavigation.ts` services.

Meaningful automated coverage includes:

- Manifest and setting default/type validation, unsupported runtimes, and disabled MCP staging without activation tools.
- Version 2 executable manifest permissions, network-origin rejection, the weather example install, runtime package asset reads, and generic bridge parsing/authorization.
- Version 3 command panels, current-grant projection, panel membership rechecks, hidden phone dispatch, and subscribed notification deduplication.
- ZIP roundtrip, export privacy, immutability, traversal, Windows paths, duplicate case, symlinks, missing declared files, and compression bombs.
- Preview/Keep enable, disable cleanup, unrelated later manual changes, field detachment, conflicting ownership, restart persistence, reset consistency, history restore of enabled/settings/detached state, removal, and failed update rollback.
- MCP and UI setting staging/persistence through the same native values.
- Settings navigation mapping for old appearance entry points and the bottom MCP category.

Manual TV/phone checks still required: real TV focus order, Back from the plugin review, removal confirmation focus, and a real Tauri import/export dialog on a clean second PC.
