# Luma declarative plugins

Status: shipped 2026-11-15. This document supersedes the declarative-only parts of [the layer plan](CUSTOMIZATION-LAYERS.md), [the distribution plan](CUSTOMIZATION-DISTRIBUTION.md), and the plugin sections of [the MCP plan](CUSTOMIZATION-MCP-PLAN.md). Executable plugin runtimes remain future work.

## What ships

Luma installs, validates, enables, previews, disables, exports, and removes declarative plugin packages. A plugin can contribute appearance and Home layout, and can declare bounded settings that control those contributions.

A plugin never runs scripts, HTML, native code, or install hooks. Packages may contain source files for a future runtime, but a package that declares a non-declarative runtime installs disabled and cannot be enabled in this version.

Luma's own Theme, Home layout, and widgets are built into the app under **Settings → Appearance & Home**. They are not a plugin: they cannot be removed, and they stay usable when another plugin is disabled or broken. The old `Theme`, `Layout`, `Home layout`, and `Widgets` entry points resolve to those sections.

**Settings → Plugins** imports, previews, enables, disables, exports, and removes packages. It holds only installed plugins. **Settings → MCP customization** remains the bottom category and owns AI access, saved versions, and reset.

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

Required: `id`, `name`, `version`, `apiVersion`, `contributes`, and a supported `runtime`. `description`, `author`, `settings`, `assets`, `source`, and `runtime.kind` default to empty/declarative. Unknown fields are rejected.

- `id` is a lowercase vendor-qualified id such as `com.example.pack`, 3–96 characters. `core` is reserved. Plugin ids are case-sensitive in storage and lowercase-only in validation.
- `version` is numeric `major.minor.patch` without leading zeros.
- `apiVersion` must be `1`.
- `settings` declares at most 24 host-rendered values. Keys match `[a-z][A-Za-z0-9_-]{0,63}`. Types are `boolean`, `enum`, `number`, and `string`:
  - `boolean` default must be true or false.
  - `enum` requires 1–20 unique string options and a default in those options.
  - `number` accepts finite numbers, optional `min`/`max`, and optional `integer: true`.
  - `string` accepts optional `minLength`/`maxLength` (maximum 500).
- `assets` and `source` are package-relative paths that must exist. `runtime.entry` is required for non-declarative runtimes and must exist.
- `contributes` must declare at least one supported field for a declarative plugin.

### Contribution fields

A contribution is either a static value or `{ "setting": "key" }`. Bindings are declarative; there are no expressions or eval.

| Field | Value | Setting binding |
| --- | --- | --- |
| `theme` | built-in theme id | enum whose options are all built-in themes |
| `density` | `comfortable` or `compact` | enum with supported options |
| `appTileShape` | `wide` or `square` | enum with supported options |
| `textSize` | `standard`, `large`, or `larger` | enum with supported options |
| `reducedMotion` | boolean | boolean |
| `homeColumns` | 1 or 2 | integer number limited to 1–2 |
| `home.order` | all seven components once | static only |
| `home.rows.<component>.visible` | boolean | boolean |
| `home.rows.<component>.span` | 1 or 2 | integer number limited to 1–2 |
| `widgets.spotifyEnabled` | boolean | boolean |
| `widgets.spotifyPlacement` | `bar`, `top-right`, `bottom-right`, or `bottom-left` | enum with supported options |
| `order` | all twelve app ids once, `Custom` last | static only |

A plugin owns every field it contributes. Two enabled plugins cannot own the same field. Enabling the second one fails with an explicit conflict naming the field and current owner. Disabled plugins may be installed with overlapping contributions; the conflict only blocks enabling.

If a user changes a field owned by an enabled plugin, Luma keeps their input, stores it in the underlying configuration, and detaches that field from the plugin. The pre-Keep review names the fields that will detach, and the plugin detail shows the detached controls with a **Preview and restore controls** action. Disabling a plugin clears its detached state so a later enable applies its full declared contribution.

## Lifecycle

1. **Import or MCP stage** validates the manifest and files, computes a SHA-256 content hash, installs the package under Luma-owned storage, and records it as disabled.
2. **Preview and enable** is a user action in Settings → Plugins. Luma stages the effective appearance and Home candidate and shows the normal review overlay. The user presses **Keep and enable**.
3. **Disable** removes only that plugin's contributions. The base appearance, Home, saved settings, and other plugins remain.
4. **Update** requires a new version. Re-importing the same id and version with identical content is idempotent; changed content under the same version is rejected with `immutable_version`. A new version replaces the package, preserves declared setting values, and stays disabled until the user previews it. MCP staging refuses to replace an enabled package; the user must disable it in Settings first. Native Import may update it and disables it atomically for review.
5. **Remove** deletes the package and its saved setting values, stops all contributions, and writes a history checkpoint. Earlier history entries that referenced the package restore with a visible "not installed" or version-changed explanation and do not pretend the plugin is enabled.

Installing or updating an enabled plugin disables it atomically; a failed install or update leaves the previous package, enabled state, settings, and rendered configuration intact.

## Settings

The manifest author writes the schema and defaults. Luma validates values on write and on load:

- Invalid stored values fall back to the declared default and are marked invalid in the plugin detail. Choosing a value and saving repairs the record.
- Settings can be edited through the host-rendered plugin detail even while the plugin is disabled. They persist across disable/enable and compatible updates.
- Settings staged through MCP are a preview candidate and require user Keep. The same values are used by the UI and MCP paths.
- Removing the plugin deletes its values. Export does not carry user values.
- Values bind only through declared contributions. A setting that is not bound is still stored and surfaced, but it has no hidden execution side effect.

## Navigation and focus

- **Appearance & Home** is Luma's own Settings category with **Theme**, **Home layout**, and **Widgets**. It is not a plugin and cannot be removed.
- **Plugins** is a separate Settings category holding **Installed plugins**: import, enable, export, and remove. It sits directly above MCP customization.
- The old `Appearance & Home`, `Theme`, `Layout`, `Home layout`, and `Widgets` entry points open the matching Appearance & Home section.
- The Appearance & Home sections render the existing theme, layout, widget, preview, restore, and reset controls exactly once.
- The bottom **MCP customization** category keeps AI access, saved versions, and reset.
- Back from a plugin review returns to Plugins. Cancel on a remove confirmation focuses Cancel. Import, action, busy, empty, error, disabled, and unsupported-runtime states are visible.
- A plugin failure does not remove the Appearance & Home sections, Settings, recovery, or the MCP connection.

## MCP

Resources:

- `luma://plugins/schema` — exact manifest, binding, runtime, and limit contract.
- `luma://plugins/example` — complete example manifest.
- `luma://customization/capabilities` — capability document including plugin support and limits.
- `luma://customization/guidance` — read → validate → stage → user Keep workflow.

Tools:

| Tool | Behavior |
| --- | --- |
| `luma_plugins_list` | Installed plugins, enabled state, controls, setting schema/values, unsupported-runtime reason, effective config. |
| `luma_plugin_validate` | Validate manifest plus optional text/base64 files. No write. |
| `luma_plugin_stage` | Install the package disabled. Never enables, keeps, activates, executes, removes, or exports. |
| `luma_plugin_settings_stage` | Validate declared setting values and stage them as the single pending review. The user must press Keep. |

No MCP tool can enable, keep, disable, remove, export, commit, restore, reset, launch, or execute a plugin. Staging settings requires a visible launcher for review, matching candidate preview.

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
| Package path | 120 bytes |

Before activation, Luma rejects absolute paths, `..`, backslashes, drive/UNC paths, Windows-reserved names and characters, case-insensitive duplicate paths, file/directory collisions, symbolic links, executable binaries (`.exe`, `.dll`, `.msi`, `.bat`, `.cmd`, `.ps1`, `.scr`, `.com`, `.cpl`), missing declared files, oversized entries, too many entries, and suspicious compression ratios. Import never runs an unzip script or package install script.

## Future executable runtime

Not shipped. The manifest already accepts `runtime.kind` values `isolated-ui`, `background`, and `main-page`, plus an `entry`. Packages with those runtimes install disabled, are labeled unsupported, are never executed, and cannot be enabled. A future runtime must define isolation, permissions, focus entry/return, CSP compatibility, background lifecycle, crash containment, and resource budgets before those packages can be enabled. Declarative packages remain the supported contract.

## Shipped limits and verification

The native implementation is in `src-tauri/src/customization/plugin.rs`, `plugin/archive.rs`, `history.rs`, `mod.rs`, and `mcp.rs`. The Settings UI is in `PluginSettings.svelte`, `PluginOwnershipNotice.svelte`, `SettingsPanel.svelte`, and the `plugins.ts`/`settingsNavigation.ts` services.

Meaningful automated coverage includes:

- Manifest and setting default/type validation, unsupported runtimes, and disabled MCP staging without activation tools.
- ZIP roundtrip, export privacy, immutability, traversal, Windows paths, duplicate case, symlinks, missing declared files, and compression bombs.
- Preview/Keep enable, disable cleanup, unrelated later manual changes, field detachment, conflicting ownership, restart persistence, reset consistency, history restore of enabled/settings/detached state, removal, and failed update rollback.
- MCP and UI setting staging/persistence through the same native values.
- Settings navigation mapping for old appearance entry points and the bottom MCP category.

Manual TV/phone checks still required: real TV focus order, Back from the plugin review, removal confirmation focus, and a real Tauri import/export dialog on a clean second PC.
