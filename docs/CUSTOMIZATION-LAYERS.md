# Luma customization layers: canonical implementation plan

Implementation boundary (2026-11-15): the current batch builds MCP access, recovery, and declarative appearance/Home plugins with settings and portable packages. Plugin lifecycle work is in progress and requires integration review; its presence in source is not completed acceptance. This batch does not implement custom executable widgets/screens, external-data providers, background extension services, or new behavior inside existing integrations. Importing code files does not make them executable.

The broader extension system remains the intended product scope, not an optional replacement with presets. Stages 6 and 7 below establish the executable UI runtime, background supervisor, and remote integration. Their public contracts must support custom widgets/screens, permitted external service access with host-managed credentials where needed, and reuse of existing Luma data/actions/events without duplicate playback or account owners. A weather widget consuming an external service and a music screen using existing playback are representative acceptance cases. Do not call the broader plugin system complete until representative extensions run through install, configure, enable, disable, restart/tray, failure, and recovery journeys. Activity-provider integration remains separately sequenced in Stage 8.

Status: planning document. No application code is implemented by this change. This file owns the cross-layer plan; [CUSTOMIZATION-MCP-PLAN.md](CUSTOMIZATION-MCP-PLAN.md) owns agent transport, tool schema, and final setup-prompt wording, and [CUSTOMIZATION-DISPLAY-BOX.md](CUSTOMIZATION-DISPLAY-BOX.md) owns the Display Box client and frame protocol detail.

Assessed: 2026-11-15 against the shared checkout. Concurrent phone-remote and Sports work was present and is not depended on here. Re-read the cited files before implementing; symbols matter more than line numbers.

## 1. How to read this plan

| Label | Meaning |
| --- | --- |
| **Shipped** | Exists in this checkout and is cited to code or an existing implementation note. |
| **Accepted intent** | The product direction the user has agreed to. It authorizes design work, not implementation detail beyond what is written. |
| **Proposal** | A recommended design in this document. It must be accepted before implementation. |
| **Open decision** | A choice that materially changes UX, security, packaging, or scope. It is listed again in §12. |
| **Not accepted** | A tempting interpretation that is explicitly excluded unless the user reopens it. |

The three layers are appearance, composition, and executable extensions. Settings, apps, screens, the desktop shell, the web remote, and the Display Box are **surfaces** that all three layers may contribute to, not additional storage models. Keeping that distinction prevents a fourth configuration owner from appearing beside the native customization store.

Accepted intent includes broad extension capability: custom settings, apps, screens, widgets, appearance, background services, and remote UI. "Broad" means many declared surfaces with host-enforced contracts. It does not mean unconstrained host access.

## 2. Goal and explicit non-goals

The user outcome is:

> Ask an agent, or use Luma directly, to reshape how Luma looks and what it contains. Review the result, keep it, and recover it later. Extensions can add real functionality without making the installed launcher fragile or unsafe.

Accepted constraints:

- Agent-originated changes require a user **Keep** action in Luma. There is no silent auto-apply.
- Luma keeps the **last 10 saved versions**, not one previous configuration.
- **Reset appearance** preserves accounts, apps, app paths, history, and extension state.
- Background work and the web remote continue while Luma sits in the tray **where practical**. The tray remains Luma-owned.
- The default shell, Settings, recovery controls, and update path remain available regardless of customization state.
- A broken, incompatible, or uninstalled extension cannot prevent Luma from opening a usable launcher.

Explicitly **not accepted** without a separate product decision:

- **Full native shell source replacement.** Extensions do not generate, patch, compile, or replace Rust, Tauri configuration, capabilities, `index.html`, or the main Svelte shell. This plan provides no such mechanism.
- **Unrestricted extension runtime.** No arbitrary native command invocation, DLL loading, shell execution, inherited Tauri permissions, or unbrokered credential access.
- A marketplace, automatic third-party package updates, or an embedded model/chat client. These remain outside this plan.
- Arbitrary CSS/HTML/JavaScript inside the main WebView. Third-party code shares the main window's trust; a wrapper function is not an isolation boundary.

```text
User / Agent / Pack / Extension
             │
             ▼
   Host validation + review (Keep)
             │
             ▼
   Native customization service
      ├─ appearance document
      ├─ composition document
      ├─ saved-version history (10)
      └─ extension registry, grants, settings
             │
     ┌───────┼──────────┬─────────────┐
     ▼       ▼          ▼             ▼
  Desktop   Web remote  Display Box  Background
  shell     surface     surface      supervisor
```

## 3. What ships today

These are facts about the current checkout, not the target design.

### 3.1 Native configuration ownership

| Area | Evidence | Consequence for this plan |
| --- | --- | --- |
| Authoritative store | `src-tauri/src/customization/mod.rs` opens `customization.sqlite3` in Tauri's application config directory and owns document, revision, preview, and restore. | Extend this owner; do not add another configuration writer. |
| Schema | `src-tauri/src/customization/document.rs` defines `version: 3`, `revision`, and a strict `deny_unknown_fields` config. | A new layer requires an explicit version migration, not permissive fields. |
| Revision safety | Commits check `expected_revision`, validate the whole candidate, and write one SQLite transaction. A failed write leaves the committed document intact. | Reuse revision checks for appearance, composition, extension settings, and agent writes. |
| Recovery | Current code keeps exactly one `previous` document. `customization_restore` restores all supported fields; `customization_restore_layout` restores only Home rows and columns. | The 10-version requirement changes this storage and the restore surface. |
| Preview | One native in-memory candidate; `customization_preview` requires a visible, non-minimized main launcher. Keep is revision-checked; cancel, a committed write, launcher destruction, or process exit discards it. | Preview remains a candidate, never a second source of truth. It covers data surfaces, not executable code installation. |
| Frontend sync | `src/lib/services/customization.ts` subscribes before initialization, serializes saves, suppresses stale events, reconciles conflicts, and disposes listeners. | New surfaces must use this service rather than writing localStorage or events directly. |
| Local HTTP | `src-tauri/src/customization/server.rs` is opt-in through `LUMA_CUSTOMIZATION_TOKEN`, binds `127.0.0.1:9878`, requires a bearer token, rejects Origin, and limits bodies to 32 KiB. It currently grants direct commit and Keep to the token holder. | This is a developer/full-trust surface, not yet the user-facing MCP permission model. See §6.6 and [the MCP plan](CUSTOMIZATION-MCP-PLAN.md). |
| Remote mirror | `src-tauri/src/remote/server.rs` mirrors app order, appearance, sound, and settings; `publish()` also emits `customization-changed` to the main window. | Remote changes must keep flowing through the same native owner. |

### 3.2 Appearance

| Area | Evidence | Consequence |
| --- | --- | --- |
| Theme contract | `src/lib/services/theme.ts` defines six built-in themes and 20 semantic CSS tokens (`--bg` through `--rating`). | A custom theme is a complete validated palette, not a loose accent value. |
| Applied tokens | The selected palette is written to the document root. The same palette and theme list are sent to the Rust remote. | Custom themes must be usable by the desktop and remote with the same token set. |
| Limits | `src-tauri/src/remote/server.rs` caps appearance payloads at 24 tokens and 8 themes, and validates token names and hex colors. | A large custom-theme library needs an explicit payload decision; silently truncating is unacceptable. |
| Other appearance controls | `density`, `appTileShape`, `textSize`, `reducedMotion`, `interfaceSounds`, and `soundLevel` are owned fields in the native document. | Decide whether sound belongs to "appearance" for reset. See D2. |
| Surface override | The web remote can keep its own theme override (`remote/src/lib/appearance.ts` and remote tests) while layout, sound, and app order follow Luma. | Surface-scoped appearance is already a product concept; extend it deliberately. |

### 3.3 Composition

| Area | Evidence | Consequence |
| --- | --- | --- |
| Home registry | `src/lib/components/HomeRenderer.svelte` maps seven core kinds (`hero`, `continue`, `apps`, `popular`, `featured`, `recent`, `recentApps`) to typed Svelte snippets. Data and actions come from `src/App.svelte`. | The renderer is a kind registry, not a package loader. General composition can start from it. |
| Home document | Rows have instance `id`, `component`, `visible`, and `span`; `homeColumns` is 1 or 2. All seven kinds must occur exactly once. | Instances are already distinct from kinds, which is the right foundation. Singletons and no nesting are current limits. |
| Layout helpers | `src/lib/services/home.ts` holds row metadata and move/toggle helpers; `HomePreview.svelte` draws a miniature from live data. | Preview is a representation, not proof of rendering. Final review must use the real renderer. |
| Focus | `src/lib/services/navigation.ts` uses visible geometry and special cases for settings/modals. `preserveHomeFocus` restores by instance ID and item key. | Any new container or isolated surface needs an explicit focus entry, return, and fallback. |
| Runtime data owners | Activity, Spotify, Stremio, Steam, catalog, and app launching are owned outside `HomeRenderer`; it receives snapshots and callbacks. | Composition must reference existing services, not instantiate duplicate players or subscriptions. |

### 3.4 Widgets, apps, settings, and screens

| Area | Evidence | Consequence |
| --- | --- | --- |
| Widgets | `src/lib/services/widgets.ts` statically imports `MiniPlayer`; `WidgetHost.svelte` renders enabled definitions. One Spotify widget supports four placements. | This is the right metadata shape for built-ins, but it is compile-time only. |
| App registry | `src-tauri/src/commands/apps.rs` defines a static `SERVICES` table (id, executable locations, optional URL), detection, path validation, and launch behavior. | Apps are data inside the binary, but adding one is still a Rust edit. Dynamic apps need a host-validated registry. |
| App order | `src/lib/types/index.ts` and `src-tauri/src/customization/document.rs` require all 12 app IDs exactly once, with `Custom` last. | Extension apps require an explicit order/migration rule, not a permissive string array. |
| App paths | `apps.json` stores user-selected `.exe` paths. Luma validates local absolute paths and launches without a shell. | An extension must never supply or change an executable path. The user picker remains host-owned. |
| App identities | Frontend `AppId`, `appProviders`, `ProviderIcon`, local assets, and the Rust catalog all encode the same twelve apps. | A dynamic app must be added in one host registry and projected to all of these surfaces, or drift is guaranteed. |
| Activity providers | `src-tauri/src/activity/providers/mod.rs` dispatches `plex`, `jellyfin`, and `trakt` with a `match`; providers are compiled modules. | A custom provider is executable extension work, not a data pack. |
| Settings | `SettingsPanel.svelte` has six categories and section-based navigation. `src/lib/services/settings.ts` defines seven remote-exposed choices plus `homeColumns`; the remote renders by `section`/`kind`. | Extension settings need a schema and a placement rule; the remote contract currently supports only choice/toggle values. |
| Screens | `src/App.svelte` switches on a `tab` string; `TopNav.svelte` lists Home, Watch, Sports, Library, Music, Apps. Settings, details, player, and dialogs are state flags. | There is no screen registry, route table, or contribution point today. |

### 3.5 Boundaries, lifecycle, and packaging

| Area | Evidence | Consequence |
| --- | --- | --- |
| WebView permissions | `src-tauri/capabilities/default.json` grants updater and limited window permissions to `main` only. | Tauri capabilities are per webview/window. A child extension surface needs its own capability file and must not inherit `main`. |
| CSP | `tauri.conf.json` uses `script-src 'self'`, `style-src 'self' 'unsafe-inline'`, strict `connect-src`, `object-src 'none'`, and `frame-ancestors 'none'`. | Loading extension code requires a new origin/runtime decision. Relaxing `script-src` on the main window is not an option. |
| Tray and background | `src-tauri/src/background.rs` creates the tray, can start with `--background` and no launcher, destroys launcher/YouTube/Stremio webviews on hide, keeps the remote server alive, and supports fast app switching. | Background services are feasible, but must not depend on a mounted Svelte component. Tray UI stays host-owned. |
| Single instance | `src-tauri/src/lib.rs` uses `tauri_plugin_single_instance` and forwards a second launch to `background::show`. | Extension services must be owned by that one process or a supervised child, never by a second Luma instance. |
| Remote server | `src-tauri/src/frame.rs` serves the web remote and Display Box on one LAN port (9877), with separate persistent credentials. `/apps` is derived from the native app catalog and app order; app commands resolve through `apps::app_command`. | Extension apps/screens must be projected into this catalog and command map, or the remotes will disagree with the TV. |
| Remote build | `package.json` builds the TV app and then the standalone `remote/` Svelte app; `scripts/inline-remote.mjs` inlines scripts and styles into `src-tauri/src/phone.html`. | Extension UI for the remote needs either host-rendered components or an isolated runtime; there is no runtime loader in the inlined page. |
| Packaging | Tauri bundles a Windows installer; release builds use a signed updater. `bundle.resources` is minimal. Updates replace the app, not user configuration. | Extension packages need their own install location, integrity, compatibility, and removal rules. |
| No extension loader | There is no plugin loader, manifest parser, grant store, or third-party code path. | Everything in §6 through §9 is new work, not an exposed existing feature. |

## 4. The three layers and their shared model

### 4.1 Layer A: appearance

**Scope (proposal):** theme identity, palette tokens, typography scale, density, tile shape, text size, motion, and per-surface overrides. Sound sets and level are visually adjacent but are **not** appearance by default; D2 confirms this.

Rules:

- A theme is a named, complete palette over the existing 20-token contract. No new token is introduced by third-party data without a host contract change.
- Values are validated for token name, color syntax, opacity range, and count before persistence and before remote publication.
- An active theme that disappears because an extension was disabled/uninstalled must not leave an unstyled launcher. The host falls back to the last built-in theme and reports the repair in Settings; the document is not silently rewritten.
- Per-surface appearance is declared, not improvised. Desktop is authoritative; the web remote may keep a local override; the Display Box consumes what its protocol supports. See [the Display Box plan](CUSTOMIZATION-DISPLAY-BOX.md).
- An extension-specific accent may be scoped to that extension's view, but it does not replace the global theme contract. This follows [the distribution plan](CUSTOMIZATION-DISTRIBUTION.md).
- Appearance contributes no scripts. A font contribution is an asset-packaging decision, not a CSS injection feature (D9).

### 4.2 Layer B: composition

**Scope (proposal):** reusable layout documents that place registered components on Home and on added screens, with bounded props, data bindings, and actions.

Rules:

- Composition is data. It never contains JavaScript expressions, arbitrary Rust commands, shell text, file paths, secrets, or raw CSS.
- The unit is a versioned document with stable instance IDs. Instance identity is separate from component type, as Home already does.
- The first supported layout is ordered rows in one or two columns, matching current Home behavior. Nesting, grids, overlays, and pixel positioning are later capabilities only when a concrete journey requires them and TV focus remains predictable.
- A component type declares supported surfaces, props, bindings, actions, sizing, instance limits, focus behavior, and required host capabilities. Components are referenced by qualified ID.
- Layout validation is whole-document and strict at commit. Loading tolerates a known-missing extension type in a quarantined/degraded state so a broken extension cannot erase the user's layout or block startup. See §6.3.
- One renderer must serve review and live use. `HomePreview` may remain a lightweight editor thumbnail, but Keep must render through the real component path or the review is not meaningful.
- Session owners stay outside instances. A Spotify widget instance reads the existing player state; it does not create a second player.

Target document shape (proposal; illustrative, not the shipped v3 schema):

```json
{
  "version": 4,
  "revision": 42,
  "config": {
    "appearance": {
      "theme": { "source": "builtin", "id": "sapphire" },
      "density": "comfortable",
      "appTileShape": "wide",
      "textSize": "standard",
      "reducedMotion": false,
      "surfaces": {
        "remote": { "themeId": "graphite" }
      }
    },
    "sound": {
      "interfaceSounds": "real",
      "soundLevel": "standard"
    },
    "composition": {
      "screens": [
        {
          "id": "home",
          "profile": "home",
          "columns": 2,
          "rows": [
            { "id": "hero-main", "component": "core.hero", "visible": true, "span": 2 }
          ]
        }
      ]
    },
    "widgets": [
      { "id": "spotify-main", "component": "core.spotify", "enabled": true, "placement": "bar" }
    ],
    "apps": {
      "order": [
        "YouTube", "Spotify", "Stremio", "Steam", "Netflix", "Disney+",
        "Prime Video", "Apple TV+", "Hulu", "Max", "Peacock", "Custom"
      ]
    }
  }
}
```

Migration from v3 prefixes core component IDs, maps `home`/`homeColumns` to the reserved `home` screen, maps the widget record to a list, groups the existing sound fields under `sound`, and preserves revision, order, visibility, spans, theme, and one-step recovery. The shipped v3 API shape remains valid until that migration lands; do not expose v4 piecemeal.

### 4.3 Layer C: executable extensions

**Scope (proposal):** packaged code that contributes at least one of: an executable UI component/screen, a background service/provider, a declarative contribution plus code, or custom settings requiring host operations.

Rules:

- Executable extensions are installed packages with identity, version, API range, declared permissions, and entry points. They are not snippets pasted into Luma.
- Code is isolated from the main WebView. The host owns identity, grants, credential release, network policy, navigation routing, and lifecycle.
- The host never trusts the extension's claim about its own ID, permission, or user grant. Identity is resolved from the installed package record and grant store.
- Disable/uninstall stops work, releases resources, and keeps a repair path for configuration that referenced the extension.
- Executable UI and background code may need different runtimes. Do not assume one sandbox fits both.

### 4.4 How the layers interoperate

| Interaction | Contract |
| --- | --- |
| Appearance + composition | A composition references an appearance theme or surface override by qualified ID. Invalid/missing themes fall back visibly; layouts do not crash. |
| Composition + extension | A package must be installed and activated, not merely staged, before its component types or screens can be committed. A pack may ship both data and code, but code activation and visual Keep are two separate consents. |
| Extension + settings | An extension declares settings; the host stores and validates them. Settings are not smuggled through component props or raw JSON blobs. |
| Extension + apps | An extension contributes safe app metadata; executable paths remain user-selected and host-stored. The native app catalog projects to desktop, web remote, and Display Box from one source. |
| Extension + screens | Reserved core screen IDs cannot be replaced. Extension screens use qualified IDs and the same component registry as Home. |
| Extension + remote | Remote contributions are declared per surface. The host renders them or isolates them; extension code never receives the pairing cookie or same-origin command authority. |
| Preview/Keep + everything | Preview stages appearance/composition data. Installing or updating executable code is an explicit install/grant step outside preview. Keep applies only the exact reviewed candidate and base revision. |

## 5. Contribution contracts

### 5.1 Package and manifest

**Proposal.** A package is a directory (or a single archive containing that directory) with a manifest, assets, and optional code entries. Suggested name: `luma-extension.json`.

```json
{
  "id": "com.example.nowplaying",
  "name": "Now Playing Extra",
  "version": "1.2.0",
  "apiVersion": "1.0",
  "description": "A compact now-playing screen with a custom provider.",
  "author": "Example",
  "contributes": {
    "themes": ["themes/midnight.json"],
    "components": ["components/now-playing.json"],
    "screens": ["screens/now-playing.json"],
    "apps": ["apps/example-tv.json"],
    "settings": ["settings.json"],
    "remotePanels": ["remote/panel.json"],
    "background": ["background/service.json"]
  },
  "permissions": [
    "appearance.contribute",
    "composition.contribute",
    "settings.own",
    "screens.own",
    "apps.contribute",
    "background.run",
    "network.allowlist"
  ],
  "network": {
    "allow": ["https://api.example.com"]
  },
  "assets": ["assets/icon.png"],
  "integrity": "sha256-..."
}
```

Manifest rules:

- `id` is a stable reverse-DNS or vendor-qualified identifier. It cannot collide with a core ID or another installed package.
- `version` is semver. `apiVersion` is a host-published range, not the Luma app version.
- Contribution paths are relative, normalized, inside the package, and size/type limited. No absolute paths, `..`, symlinks, or network URLs in the manifest.
- `permissions` are exact host capability names. Unknown permissions reject the package. A later version cannot gain a permission without a new visible grant.
- Package permissions are not MCP grant scopes. The MCP plan's `read`, `appearance`, `composition`, `settings`, `apps`, `screens`, `extensions.install`, and `extensions.configure` describe what an agent candidate may touch. A package that contributes appearance does not thereby gain agent read/write scope, and an agent scope never starts extension code.
- `integrity` covers the package content. Local development may allow an unsigned package with a persistent warning. Signature requirements for sharing are decided by D10 and [the distribution plan](CUSTOMIZATION-DISTRIBUTION.md).
- Install is a transaction: validate the manifest, unpack to a versioned directory, verify integrity, and write a `staged` record. Activation is a separate user action that registers contributions, checks grants, and enables the allowed surfaces. A failed step leaves the previous version and grants intact.
- One extension may declare several contribution kinds, but each contribution validates independently so one bad theme does not hide a working screen.

### 5.2 Qualified IDs

**Proposal.** Every contributed object uses a qualified ID: `core.<name>` for Luma-owned objects, and `<extensionId>.<name>` for package-owned objects. Examples: `core.hero`, `com.example.nowplaying.compact`, `com.example.nowplaying.screen`.

- IDs are ASCII, 1–96 characters, dot-separated segments, no empty segments, no whitespace.
- Core IDs are reserved and cannot be registered by a package.
- A screen, component, widget, app, theme, setting, and remote panel each has its own namespace; the same suffix in two namespaces does not collide.
- Instance IDs remain separate and are unique only within their document. Existing Home instance IDs stay valid after migration.
- Removing a package leaves qualified references unresolved rather than silently rebinding them to another package with a similar name.

### 5.3 Component contribution

```json
{
  "component": "com.example.nowplaying.compact",
  "displayName": "Compact now playing",
  "apiVersion": "1.0",
  "surfaces": ["screen"],
  "sizing": { "spans": [1, 2], "defaultSpan": 2 },
  "props": {
    "showArtist": { "type": "boolean", "default": true },
    "title": { "type": "string", "maxLength": 60, "default": "Now playing" }
  },
  "bindings": [
    { "key": "track", "source": "spotify.nowPlaying", "required": true }
  ],
  "actions": [
    { "key": "open", "action": "spotify.openPlayer" }
  ],
  "instances": { "min": 0, "max": 1 },
  "focus": { "entry": "first-control", "return": "remember-instance" },
  "requires": ["core.spotify"],
  "runtime": "isolated-ui"
}
```

- Props are a bounded schema: boolean, enum, bounded integer, bounded string, or a host-defined named format. No expressions, no CSS, no HTML.
- Bindings reference a host-published data source with its own validation and refresh policy. A component cannot open an arbitrary URL or subscribe to a private event.
- Actions reference a host-published operation. A component cannot invoke a Tauri command by name.
- `runtime` is `declarative` (host-rendered) or `isolated-ui` (code). A missing runtime is rejected. Background services declare their runtime separately.
- Metadata declares loading, empty, error, disabled, and unavailable states, and whether the component can own focus. The host renders the fallback shell for those states so a crashing extension does not leave a blank home.
- Instance limits are enforced across all screens, not per screen, when a component shares a session resource.

### 5.4 Screen contribution

**Proposal.** A screen contribution adds a destination; it never replaces a core destination.

```json
{
  "screen": "com.example.nowplaying.screen",
  "title": "Now Playing",
  "icon": "assets/now-playing.svg",
  "description": "A focused view of the current track.",
  "surfaces": ["desktop"],
  "runtime": "declarative",
  "layout": {
    "columns": 1,
    "rows": [
      { "id": "now", "component": "com.example.nowplaying.compact", "span": 1 }
    ]
  },
  "nav": { "placement": "customization-screens", "pinned": false }
}
```

- Core screen IDs are reserved: `home`, `watch`, `sports`, `library`, `music`, `apps`, `settings`. Core screens remain owned by Luma and are always reachable through recovery.
- Extension screens are reachable through Settings → Customization → Screens by default. Pinning into the top navigation is an explicit user action with a small cap, because the top bar must remain readable at 1080p and 4K. D4 decides the cap and default.
- An executable screen declares a runtime entry point and surface; its data bindings still use the host contract. It may not create its own authentication or background loop outside the supervisor.
- A screen that is disabled/uninstalled leaves a recoverable Settings entry; it does not remove the user's other screens or core navigation.
- The web remote and Display Box may expose a screen summary or a remote-specific panel. They do not run desktop screen code. See §5.7.

### 5.5 App contribution

**Proposal.** An app contribution adds safe launch metadata. It does not add a native command.

```json
{
  "app": "com.example.iptv",
  "label": "Example TV",
  "icon": "assets/example-tv.png",
  "surfaces": ["desktop", "web-remote", "display-box"],
  "launch": {
    "kind": "url",
    "url": "https://tv.example.com/",
    "profile": "isolated"
  },
  "deepLink": null,
  "activityProvider": null
}
```

Launch kinds:

| Kind | Behavior | Status |
| --- | --- | --- |
| `url` | Host opens an allowlisted `https` URL in its existing isolated Edge profile path, or an embedded host window if the URL policy allows it. | Proposal for first app contribution. |
| `screen` | Opens a declared extension screen instead of an external service. | Proposal. |
| `embedded` | Host-owned WebView window with an allowed origin list. | Open; existing Stremio/YouTube windows are host-specific. |
| `executable` | The **user** chooses a local `.exe`; the host stores it under the extension app's qualified key. The extension never supplies a path or arguments. | Proposal, gated by D8. |
| `native-command` | Extension invokes an arbitrary native action. | **Rejected.** |

Integration requirements:

- Add one authoritative host app registry that merges the core catalog and installed extension apps. Project it to the dock, `AppId` handling, `appProviders`, the web remote `/apps`, and the Display Box app list.
- Preserve the existing rule that all core apps appear exactly once and `Custom` stays last. Extension apps occupy slots before `Custom`.
- The remote app command must become a generic, validated `app:<qualifiedId>` or an equivalent catalog-token lookup. Do not add one hardcoded command per extension.
- App icons are host-served from validated package assets; never interpolate an extension path into HTML.
- Disabling an app removes it from the dock and remotes but preserves its stored order position and user-chosen executable path for re-enable. Uninstalling removes the app and keeps a repair record until the package is gone.

### 5.6 Settings contribution

**Proposal.** Settings are declared, host-rendered fields with host-owned storage.

```json
{
  "settings": [
    {
      "key": "apiBase",
      "label": "Server URL",
      "type": "string",
      "format": "https-url",
      "default": "",
      "maxLength": 200,
      "surfaces": ["desktop", "remote"],
      "secret": false
    },
    {
      "key": "refreshMinutes",
      "label": "Refresh interval",
      "type": "integer",
      "min": 5,
      "max": 240,
      "default": 30,
      "surfaces": ["desktop"]
    }
  ]
}
```

Rules:

- Supported first types: boolean/toggle, enum/choice, bounded integer, bounded string with a named host format. Free-form JSON, regex supplied by the extension, and HTML are rejected.
- Values are validated on write and on load. An invalid stored value falls back to the declared default with a visible warning; it does not break startup or erase the raw record without a recovery path.
- Storage is host-owned and keyed by extension ID + setting key. Values are included in export only when explicitly portable and non-secret. Secrets go to Windows Credential Manager through the host credential broker and never enter the document, history, remote payload, or export.
- Default placement is inside **Settings → Customization → Extensions → <extension name>** so contributed settings do not expand the top-level navigation. A package may declare `settings.section` to place fields in an existing category only after D3 accepts that behavior; the host controls section order and provenance labels.
- The current remote contract supports choice/toggle values. Extension settings appear on the remote only for declared-compatible types; unsupported types remain desktop-only with a remote explanation. This must be explicit in the contract, not a silently missing control.
- Extension settings have their own expected revision for optimistic concurrency. A desktop and remote edit to the same setting must conflict visibly, not overwrite.
- Uninstall offers "Keep settings for reinstall" (default) or "Delete extension data"; secrets are revoked immediately when the extension is removed, regardless.

### 5.7 Desktop and web remote surfaces

**Proposal.** Contributions declare surfaces. The host renders each surface with its own trust level.

| Surface | First supported contributions | Isolation rule |
| --- | --- | --- |
| Desktop shell | Theme, component, widget, screen, app, settings, background service, tray command (later). | Code runs only in the chosen isolated runtime. Main WebView never loads extension scripts. |
| Home | Core components plus registered components allowed by the Home profile. | Uses the existing data/action owners; no per-instance subscriptions. |
| Customization hub | Installed packages, screens, versions, grants, errors, capabilities. | Host-rendered. |
| Web remote | Declarative panels and settings first; executable panels only after D7. | Extension code never receives the pairing cookie or same-origin API authority. |
| Display Box | Appearance, app catalog/order, state-derived read-only panels that its protocol supports. | Frame client and protocol are owned by [the Display Box plan](CUSTOMIZATION-DISPLAY-BOX.md); no extension code on the frame. |
| Tray | At most one command entry per enabled extension, host-rendered; no replacement of Open/Exit. | Host owns the tray icon, menu, and lifecycle. D14. |

Web remote rules (accepted intent for broad capability, proposal for mechanism):

- The served remote page gets authenticated state from the host. It must not fetch third-party scripts; CSP and inline-build rules stay.
- A declarative panel is a host-rendered form/list bound to a named remote command or setting. It can control only actions the host exposes to that extension.
- Executable remote panels require a separate, isolated origin or a sandboxed frame without same-origin access. The host relays messages after validating extension identity and the declared action. This is not the same as dropping extension HTML into `phone.html`.
- The remote must explain when a screen/setting is desktop-only rather than showing a dead control.
- Pairing, revoke, and "Forget all remotes" behavior is unchanged by extension contribution. Installing an extension must not renew or broaden remote credentials.
- Current remote CSP and the inlined single-file page deliberately do not execute extension scripts. Executable remote panels therefore require a separate isolation design, not a CSP relaxation. [The Display Box plan](CUSTOMIZATION-DISPLAY-BOX.md) recommends keeping extension code off both remote surfaces in the first implementation; D7 agrees.
- Display Box compatibility is a surface constraint, not a license to weaken the frame protocol. Its proposed surface vocabulary is owned by [the Display Box plan](CUSTOMIZATION-DISPLAY-BOX.md); this plan does not add frame node types.

## 6. One owner, one review flow

### 6.1 Persistence model

**Proposal.** Keep one SQLite database as the customization authority, adding tables rather than creating a second store:

| Table | Purpose |
| --- | --- |
| `customization` | Current committed document and compatibility `previous` pointer until history migration completes. |
| `versions` | Up to 10 checkpoints: revision, document JSON, origin, timestamp, human summary, non-secret extension-settings snapshot, and referenced artifact IDs/versions/hashes. The MCP plan uses this table name and owns the row detail. |
| `extensions` | Installed package record: id, version, API range, manifest hash, enabled, installed path, compatibility state. |
| `extension_grants` | Package runtime grants per permission, version-scoped, plus grant timestamp; a permission expansion requires a new row/consent. |
| `agent_grants` | Hashed agent credentials, label, scopes, created/last-use, and revocation state. The MCP plan owns the scope model. |
| `extension_settings` | Non-secret values keyed by extension ID and setting key, with revision metadata. |
| `extension_data` | Small host-owned non-secret state (order position, last error summary, disable reason). Large extension-owned data belongs in a per-extension data directory. |

Secrets stay in Windows Credential Manager. Package code and large assets stay in an `extensions/<id>/<version>` directory under the app config/data directory. Do not put arbitrary package paths in the document.

Why one database: install/disable/update, document repair, grants, and history need one transaction boundary. A failed install or a failed Keep must not leave a half-applied extension. Read-only asset files can be written before the transaction and cleaned up on rollback.

### 6.2 Revision, history, and the last 10 versions

**Proposal.** Every successful commit produces a history checkpoint, with one coalescing rule for autosave bursts:

- A checkpoint stores the complete committed document, `revision`, `createdAt`, `origin` (`manual`, `remote`, `display-box`, `agent`, `extension`, `migration`, `reset`, `restore`), and a short host-generated summary (for example "Theme and Home columns changed").
- A checkpoint also stores the non-secret extension settings and artifact references (`id`, `version`, `hash`) that the document depended on. Artifact bytes are retained while they appear in the 10-version window and fit the agreed size cap. A missing artifact still restores configuration, but the extension stays disabled with an explicit "artifact missing" state. Never substitute another version's bytes.
- Restoring a checkpoint writes its non-secret extension settings back with the document. It never activates a package, restores a credential, or replays external side effects. Secrets are excluded from history and must be re-entered or reconnected.
- An extension install, update, activation, or removal that changes customization or referenced artifacts also writes a checkpoint in the same transaction as the extension record change.
- Explicit review actions — Keep, restore, reset, pack apply, agent apply — always create a distinct checkpoint and are never coalesced.
- Ordinary autosaves from the same origin and client may coalesce into the newest checkpoint for a short window (proposal: 60 seconds) so that toggling a slider ten times does not consume the entire history. The coalesced checkpoint holds the latest state. D1 confirms the window and whether remote/desktop share a burst key.
- Retain the 10 most recent checkpoints. The current document is the newest checkpoint and is shown as "Current"; the total is 10, not current + 10.
- Restoring a checkpoint creates a **new revision** whose config equals that checkpoint. It never rewinds the revision counter and never deletes the current state from history.
- Migration seeds the first checkpoint from the migrated document so the user can return to their pre-upgrade state after the first post-upgrade edit. If the previous one-step document exists, import it as the second checkpoint rather than dropping it.
- A failed write or failed Keep inserts no checkpoint. A discarded preview inserts no checkpoint.
- History never stores credentials, tokens, pairing secrets, executable paths, playback history, or extension secrets. It may store app order and non-secret layout/appearance/extension selection.
- The HTTP/MCP surface exposes history metadata only. Restore and reset are user actions in Luma and are not MCP tools.

`customization_restore` becomes "restore the immediately previous checkpoint" for compatibility; `customization_restore_layout` applies only the composition namespace from that checkpoint. A new `customization_history_read` returns metadata; `customization_history_restore` takes a checkpoint ID or revision plus an expected current revision.

### 6.3 Strict validation, degraded loading, and repair

Two levels are needed because an installed extension can disappear after a document was valid.

| Level | Used for | Behavior |
| --- | --- | --- |
| Strict | Commit, Keep, pack import, agent candidate, manifest install, settings write. | Reject unknown component/theme/app, duplicate instance, invalid props/bindings/actions, wrong surface, over-limit size/depth, missing resolution, permissions not granted. Return structured, actionable errors. |
| Degraded load | Startup, launcher re-creation, app update, extension disable/uninstall. | Load the document without overwriting it. Render a host-owned "Unavailable extension" placeholder at the affected instance/screen. Keep core Home, Settings, and recovery reachable. Block commits that would preserve the unresolved reference until repaired. Report the exact package/component needed. |

Repair flow (proposal):

1. Open Settings → Customization → Problems.
2. Choose **Re-enable package**, **Replace component** (compatible registered type), **Remove instance**, or **Remove screen**.
3. Repair becomes a normal draft. Preview and Keep are required for a multi-node repair; a single missing instance may be removed after explicit confirmation.
4. If the user uninstalls a package that is still referenced, warn and offer to repair first. Do not silently delete composition.

### 6.4 Preview and Keep

- Shipped preview is native, revision-checked, single-candidate, and launcher-dependent. The target service separates **staging** from **rendering**: a candidate may be staged while the launcher is absent, but its review panel renders when the user opens Luma. Keep still requires the desktop review UI.
- The reviewed candidate includes appearance, composition, screens, widgets, and app order changes in one document.
- The review surface must show: what changed, which package/agent produced it, any unavailable capabilities, and the base revision.
- Keep applies the exact candidate ID and base revision. It is a Tauri command restricted to the `main` review UI; there is no HTTP route or MCP tool that reaches it. A committed write from another source invalidates the preview; stale IDs cannot keep or cancel a newer candidate.
- Agent-originated Keep is a **user action in Luma**. An agent token may stage/read a candidate and request repair; it must not be able to press Keep by itself. See §6.6.
- Installing executable code is not previewable in the same sense. It needs an explicit activation consent listing permissions, package identity, version, and origin. A visual sample may be shown only after activation, and the grant is not implied by Keep.

### 6.5 Reset appearance

**Proposal.** Add `customization_reset_appearance(expected_revision)` with a narrowly defined scope:

- Implementation scope: reset theme, density, app tile shape, text size, reduced motion, Home rows/order/visibility/columns/spans, and widget visibility/placement to built-in defaults. The confirmation names layout and widgets explicitly. Future appearance namespaces require a separate migration.
- Preserve: app order, app paths, activity/history, accounts, pairing credentials, extension installs/grants/settings, extension data, and sound settings. History can restore the pre-reset look.
- Sound set/level are preserved unless D2 says otherwise.
- Only host-owned appearance resets. The web remote's local follow/override is stored in that browser's localStorage; it is cleared by choosing **Follow TV** on that device, not by a desktop reset. The Display Box's own light/dark palette and standalone settings are outside this reset.
- The reset is a normal commit with a new revision, a `reset` origin checkpoint, and a confirmation that names what will change. A second click is not required if the first dialog clearly says "Appearance only; accounts and apps stay."
- If reset clears an active custom palette, the fallback is the built-in default, not an undefined theme.
- The UI offers **Reset appearance**, existing **Reset size and motion**, **Reset sound**, **Restore previous layout**, and history restore as distinct actions. They must not be merged into one vague "Reset Luma".

### 6.6 AI access (MCP) requirements

The detailed wire contract belongs to [CUSTOMIZATION-MCP-PLAN.md](CUSTOMIZATION-MCP-PLAN.md). The cross-layer requirements are:

- **Capability discovery** reports installed appearances, components, screens, apps, settings, background services, their versions/limits, and what the agent may read/draft. It must not report secrets or private history.
- **Read** returns the current document and revision. A separate metadata read lists available versions. Neither exposes credentials, playback history, pairing tokens, or executable paths.
- **Validate** returns structured errors for the complete candidate at an expected revision.
- **Stage** creates a service-side pending candidate bound to the base revision. It does not require the launcher to be mounted; the review panel renders when Luma is opened. [The MCP plan](CUSTOMIZATION-MCP-PLAN.md) owns the candidate record and digest detail.
- **Keep is user-only.** The agent can read its own candidate and status; it cannot commit. A separate full-trust developer token may still use the shipped direct API, but that is not the agent permission model (D15).
- **History** is readable as metadata. Restore and reset are user actions in Luma, not MCP tools; an agent may ask the user to perform one, but cannot call it.
- **Install/update/disable** of executable packages is never an MCP capability in this plan. The most an agent may do is stage a declarative pack or an inert extension artifact for user review. Activation is not an MCP action.
- **Revocation** takes effect on the next request: rotate/revoke invalidates outstanding tokens, and a pending candidate created by that grant is discarded with a visible explanation. Keep authority never transfers to another grant.
- **Disconnect** leaves Luma fully usable. No normal rendering or navigation depends on the agent process.
- **Tray** does not change agent behavior: the service can answer read/validate and accept a staged candidate while the launcher is destroyed. Rendering the review and Keep require the user to open Luma.

### 6.7 AI access settings surface

**Accepted intent:** Settings offers a generic, copyable setup prompt with no IDE-specific instructions. **Proposal** for the controls:

Settings → Customization → AI access:

| Control | Behavior |
| --- | --- |
| Enable local agent access | Starts/stops the loopback service. Off by default. |
| Address | Read-only `127.0.0.1:9878`, or the dedicated MCP port chosen by the transport spike. |
| Grants | One row per client: label, scopes, created time, last use, **Revoke**. The credential is shown once at creation and never rendered again. |
| Permissions | Scoped read/stage grants only; recommended default is read + appearance + composition. Keep is performed in Luma. A separate developer/full-trust mode is labeled as such. |
| Copy setup prompt | Copies a host-agnostic prompt with **no credential and no endpoint**. This text is safe to paste into an agent chat. |
| Copy connection details | Copies the endpoint/credential or bridge command. Luma warns that this text is a credential and must not be pasted into a chat, issue, or repository. |
| Test connection | Distinguishes Luma not running, access disabled, invalid grant, and version mismatch. |
| Connection help | Short troubleshooting: start Luma, enable access, open Luma to review a staged change, rotate after sharing, port conflict. |

Generic prompt shape (final wording owned by the MCP plan):

```text
Connect to the Luma MCP server that the user has already added to your MCP client.

Luma is a Windows TV launcher. You can inspect its customization capabilities,
read the current configuration, and stage changes for the user to review.

Rules:
1. If the Luma tools are not available, stop and tell the user to open
   Luma > Settings > Customization > AI access and add the server shown there.
   Do not guess connection details.
2. Start with the capability and configuration reads. Do not invent components,
   properties, themes, screens, or extension permissions.
3. Prefer the smallest change that satisfies the request. Ask the user when the
   request is ambiguous, and say so before changing app order or screens.
4. Validate a complete candidate, then stage it. Tell the user the change is
   waiting for review and that they must press Keep in Luma.
5. Never claim a change is applied, saved, or live until a later read shows a
   new revision.
6. Report validation errors exactly. Do not retry an unchanged candidate.
7. Do not ask for, print, or store tokens, passwords, or account credentials.
   Luma never returns them through these tools.
```

No mention of a specific editor, CLI, model vendor, or config-file location. The same prompt must work with any host that can call HTTP or launch its own MCP bridge. The credential lives only in the separate connection-details copy, which follows the MCP plan's grant model.

The docs deliberately do not show a real token or endpoint in the prompt. Copying a credential is a separate, warned action because prompts are routinely pasted into chats and logs.

## 7. Background lifecycle

**Accepted intent:** extensions may run background services, and Luma should stay useful in the tray where practical. **Proposal** for the supervisor:

### 7.1 Levels and triggers

| Trigger | Runs when | Requires |
| --- | --- | --- |
| `launcher-open` | The main launcher webview exists. | Explicit `background.run`; stopped on hide/destroy. |
| `startup` | Luma starts, including `--background`, after the tray and core services are ready. | Explicit `background.run` + user grant for startup execution. |
| `on-demand` | A screen, app, or action requests the service. | Explicit `background.run`; stops after idle timeout unless another trigger applies. |
| `always` | While the extension is enabled, including tray-only and startup. | Strongest grant, visible warning, resource budget, and a user-visible tray/extension status. |

- `launcher-open` is the default for UI-adjacent services. `startup`/`always` must be opt-in per extension and shown in the grant list.
- Do not start user extension code before core customization, credentials, and tray state are ready. Start asynchronously so window and tray readiness do not wait on extension work.
- One instance per extension service per user session. A second start request attaches or reports "already running."
- No background polling when an existing host event, socket, or file watcher can be used. The host may deny a manifest that declares an unreasonable timer; timers are not a general keep-alive.

### 7.2 Lifecycle states

```text
installed → disabled → enabled → starting → running → idle/suspended
                                      │         │
                                      └─ error ─┴→ stopped
```

- **Install (stage):** validate and record the package; contributions are not registered and code is not started until activation.
- **Activate/enable:** register contributions, validate compatibility and grants, then start the matching trigger only. Code activation is a separate consent from UI contribution.
- **Disable/revoke:** stop the runtime, close listeners/handles, clear pending work, release the hidden window/process. Keep data and settings.
- **Uninstall:** disable first; then remove code; then offer data deletion; then repair/remove references. Never leave a running process after uninstall.
- **App update:** revalidate every installed package against the new `apiVersion`. Incompatible packages are disabled with a reason and a "Check for compatible version" action; their configuration is preserved.
- **Crash:** bounded restart (proposal: at most 3 attempts in 10 minutes), then disable and surface the last error. No unbounded retry loops.
- **Suspend:** on-demand/launcher-open services stop after a defined idle period when no visible surface depends on them. `always` services need an explicit user-visible reason and budget.
- **Exit:** stop every extension runtime before process exit; do not rely on OS kill for cleanup that could corrupt user data.

### 7.3 Tray behavior

- The tray icon and menu remain host-owned. Extensions may contribute at most one command submenu/item, host-rendered, after D14. They cannot replace Open Luma TV or Exit, hide the tray, or change startup.
- `startup`, `always`, and remote-panel services may continue while the launcher is destroyed, provided the user enabled that behavior. Rendering code for the launcher does not continue with a hidden window.
- A pending candidate may add a host-rendered "Review pending change" tray entry that opens the desktop review panel. The tray never shows the diff and never performs Keep.
- Tray-only operation must not require a Svelte component, a mounted store, or the main WebView's IPC. This is the same rule the native customization server already follows.
- Choosing Exit stops the local server, tray, and all extension code. The confirmation remains truthful about what will stop.
- A crashed extension must not remove or disable the core tray menu.

### 7.4 Resource and trust boundaries

- Each runtime has a host-enforced memory/CPU/network/disk budget. The supervisor measures and reports; it does not trust self-reporting.
- Network access is host-mediated or explicitly allowlisted per manifest. Current CSP `connect-src` restrictions are not bypassed by extension code. An extension cannot silently widen Luma's network policy.
- Credentials are released per operation or scoped to the extension's own isolated runtime; the host never hands over core Spotify/Stremio/remote pairing tokens.
- Extension storage is a dedicated directory with a size cap and no traversal. Uninstall offers retention or deletion.
- Errors are summarized for the user and capped in `extension_data`; no unbounded log growth, no telemetry upload.
- Provisional acceptance envelope for a future loader (must be re-measured, not claimed now): no additional startup delay visible to the user, idle CPU under 1% of one core averaged over a minute, and private working set agreed from a measured baseline before enabling general installation. Numeric caps are a gate, not a shipped guarantee.

## 8. Settings information architecture

The existing category/section pattern stays valid. **Proposal:**

| Location | Sections | Contents |
| --- | --- | --- |
| Appearance & Home | Theme, Layout, Widgets | Built-in/custom themes, density/text/tile/motion, Home composition, widget placement. |
| Display & Sound | Display, TV & audio, Sound | Unchanged; sound reset remains separate from appearance reset. |
| Apps & Services | Apps, Accounts | Core apps, extension app contributions, executable paths, account connections. |
| Remotes & Devices | Remotes, Keyboard | Pairing, Display Box, web remote; extension remote panels are configured from Customization, not here. |
| History & Privacy | Recording, Clear history | Activity history and privacy; does not clear customization history. |
| System | Startup & memory, Updates | Tray/startup/fast switch, updater. |
| **Customization** | Extensions, Screens, Versions, Reset appearance, AI access, Problems | Package list/detail, enable/disable/grant/uninstall, screen pinning, 10-version history, visual reset, MCP setup, degraded-load repair, developer mode. |

Notes:

- This follows the Settings placement proposed in [the MCP plan](CUSTOMIZATION-MCP-PLAN.md): one new top-level **Customization** category, rather than spreading Versions, Extensions, and AI access through six existing categories. A seventh top-level category is a real navigation cost; the alternative buries extension management and makes Problems hard to reach. D3 decides; the recommendation is one new category with a 10-foot width check.
- Extension-owned settings live in the extension detail page. A package may request a section in an existing category only if D3 accepts it; contributed sections are labeled with the package name and cannot reorder core sections.
- Versions are user-facing, not a raw JSON editor. Each row shows time, origin ("You", "Phone", "Agent", "Extension"), and a short summary. Restore is confirmed and reversible because it creates a new version.
- Reset appearance and history restore are separate actions with separate confirmation text. Neither touches accounts or app paths.
- The web remote reflects core appearance/layout settings it already supports, plus its device-local Follow TV/override choice. A desktop reset does not reach that local storage; the remote offers its own Follow TV action.
- The Display Box surface shows only what its protocol supports; its settings and local palette remain on the frame and its own settings, not this reset.

## 9. Packaging and build strategy

### 9.1 Shipped baseline

- `npm run build` builds the Svelte app, then the standalone remote, then inlines the remote into `phone.html`.
- Tauri packages a Windows installer; the updater is signed and replaces the application, preserving user data directories.
- The main window has a strict CSP and a narrow capability file. The remote is a separate inlined app served by Rust, with its own CSP and cookie auth.
- Bundled assets are compiled into the binary or listed in `bundle.resources`.

### 9.2 Target packaging by layer

| Layer | Artifact | Loading | Update |
| --- | --- | --- | --- |
| Appearance | JSON theme(s) + optional images/fonts | Host validates and serves assets | Pack version; no code |
| Composition | JSON layout/screen/widget/app declarations + assets | Host registry and renderer | Pack version; preview/Keep |
| Executable UI | Package code + manifest assets | Isolated runtime selected at gate G2 | Manual package update; grant re-check |
| Background service | Package code + declaration | Supervisor-selected runtime at gate G3 | Manual update; stop/restart |
| Web remote panel | Declarative data or isolated panel asset | Host-rendered or isolated frame at gate G5 | Same package version |

- Declarative packs install under a host-owned `packs/<id>/<version>` directory. Import validates manifest, compatibility, assets, and required core components before offering a preview.
- A declarative pack exports only portable data: themes, composition, app declarations, non-secret settings, and package identity/version. It excludes credentials, tokens, paths, history, code binaries, and machine identifiers. A code extension export instead follows [the distribution plan](CUSTOMIZATION-DISTRIBUTION.md) and includes its own package source/assets and manifest for import; it still excludes credentials, tokens, personal paths, history, and machine identifiers.
- Executable packages install under `extensions/<id>/<version>` with a manifest hash and a `current` pointer. Keep the previous version until the new one starts successfully or an explicit rollback window passes.
- No marketplace, remote index, automatic package update, or background download in the first release. A user chooses a local file/folder, reviews identity and permissions, and installs. Direct file sharing and the later catalog path follow [the distribution plan](CUSTOMIZATION-DISTRIBUTION.md); Luma never sends a package to another person automatically.
- Content limits apply before unpacking: total size, file count, path length, compression ratio, and allowed extensions. Reject executables inside declarative packs.
- App updates must never delete extension packages or user data. A compatibility failure disables and reports; it does not wipe.

### 9.3 Decision gates

Each gate needs a written result and evidence before the next stage depends on it.

| Gate | Question | Why it blocks | Recommendation |
| --- | --- | --- | --- |
| G1 | Can declarative packs meet the near-term goal without executable code? | Avoids an unnecessary runtime and supply-chain burden. | Ship G1 first; require a named feature that cannot be expressed declaratively before G2. |
| G2 | Which isolated UI runtime: child WebView with a custom origin, sandboxed iframe, or separate process/window? | Focus, CSP, permissions, crash containment, and TV Back behavior differ materially. | Timeboxed spike with one widget. Prefer a child WebView with its own capability file if focus entry/return is acceptable; iframe only if origin isolation is proven. No main-WebView scripts. |
| G3 | Which background runtime: hidden WebView worker, JS sidecar process, WASM/WASI host module, or native plugin? | Memory, startup, IPC, packaging, and trust differ; UI and providers may need different answers. | Spike with one provider-style service; prefer the smallest runtime that can be denied native access. Native plugins stay out of the first release. |
| G4 | How are package identity and integrity established? | Unsigned code plus broad grants is not a defensible sharing model. | Local import with content hashes and explicit review first; decide publisher signatures before a catalog, per [the distribution plan](CUSTOMIZATION-DISTRIBUTION.md). Reuse an established signature format rather than inventing one. |
| G5 | How is executable remote UI isolated? | Same-origin remote code would inherit cookie/command authority. | Declarative remote panels first. For executable panels, cross-origin/isolated frame without cookies; host-relayed actions only. |
| G6 | Can extension assets/fonts load without weakening the main CSP? | `script-src 'self'` and `font-src 'self'` are deliberate. | Host-served validated assets via a scoped protocol or small data URIs; never relax script policy for the shell. |
| G7 | Does the chosen composition model replace the Home renderer or sit beside it? | Two renderers drift and double focus bugs. | One registry/renderer with a Home validation profile; migrate Home in one versioned step (D5). |
| G8 | Is full native shell replacement in scope? | It changes the product from launcher customization to a fork platform. | **Not accepted.** If requested, treat it as a separate product/repository decision with its own build, signing, update, and support plan. |

## 10. Staged implementation

The stage sequence below is a proposal. Every stage ends with a checkpoint update to this document and the roadmap. Do not build later stages in advance.

### Stage 0 — Resolve gates and contracts

Deliverables:

- Accept or reject D1–D16; record the answer here and in the roadmap.
- Write the manifest, component metadata, settings schema, and v4 migration as small schemas with examples.
- Decide the history coalescing rule and reset scope before touching the store.
- Keep MCP and Display Box detail in their companion plans; agree only the cross-layer requirements in §6.6 and §5.7.

Exit criteria:

- No unresolved choice blocks Stage 1.
- The target document shape and migration are reviewable without application code.

### Stage 1 — Appearance control, 10-version history, reset appearance

Scope:

- Add `versions` with 10 checkpoints, origin/summary, migration from `previous`, and no-loss behavior for unsupported documents.
- Add read/restore endpoints and native commands; map existing `restore`/`restore_layout` semantics to the new history.
- Add `reset_appearance` scoped edit.
- Add Settings → Customization → Versions, plus a reset appearance confirmation and provenance labels. Expose reset near Theme as well if the entry point proves hard to find, but keep one action.
- Seed migration history; preserve the old one-step document as a checkpoint.
- No composition schema change and no extension code yet.

Exit criteria:

- 12 commits leave exactly 10 checkpoints, with the oldest pruned only after a successful commit.
- Restore from any checkpoint creates a new revision, matches the checkpoint config, and leaves the replaced state recoverable.
- Reset appearance changes only appearance fields; all other namespaces and account state are unchanged.
- A failed write, canceled preview, launcher destruction, and stale revision insert no checkpoint and do not change the current document.
- Existing manual, remote, tray, and HTTP flows still work; the remote mirrors the new appearance values.
- Live TV/phone checks cover Settings focus, Back, confirmation dialogs, and restore feedback.

### Stage 2 — Composition v1 and the shared renderer

Scope:

- Add the v4 document and v3→v4 migration.
- Extract a component registry/metadata layer; represent Home with a `home` profile and migrate the seven core kinds to qualified IDs.
- Implement one renderer path used by both live Home and Keep review; keep data/action owners where they are.
- Add custom screens composed only of core components; no third-party components yet.
- Add strict/degraded validation and the Problems repair entry.
- Keep one/two columns and span; no nesting, grid, overlay, or pixel positioning.

Exit criteria:

- Default and migrated Home match current behavior for order, leading hero, inset hero, hidden rows, app previews, empty/error states, and all-hidden recovery.
- Keyboard and phone navigation can enter, act, return, and reach Settings from every core screen; focus returns after rerender.
- A document referencing a temporarily missing extension type loads degraded with a visible placeholder, does not crash, and does not overwrite the document.
- Qualified-ID, duplicate-instance, invalid-prop, wrong-surface, and over-limit candidates are rejected with actionable errors.
- Preview/Keep renders through the same component path as live.

### Stage 3 — Declarative packs and custom screens

Scope:

- Import/export packs containing themes, composition, core-component screens, app declarations, and non-secret settings.
- Preview/Keep for pack application; compatibility and missing-component checks.
- Settings → Customization → Extensions: packs, detail, enable/disable, export, remove.
- Screen hub and optional pinning (per D4).
- Asset pipeline for pack images/icons, with limits.
- No executable code yet.

Exit criteria:

- A pack moves between two installations and applies through review without repository access.
- An incompatible pack leaves the current document and checkpoint history intact.
- Export contains no credentials, tokens, paths, pairing data, history, or machine identifiers.
- Disabling a pack removes its theme/screen/app contributions from desktop and remotes, and re-enabling restores positions and non-secret settings.
- A pack with a missing core component fails before Keep with the missing IDs listed.

### Stage 4 — Extension package lifecycle (declarative contributions)

Scope:

- Add the extension registry, manifest validation, grants, per-extension settings, app contributions, screen contributions, and remote app projection.
- Add the unified host app registry and generic remote/Display Box app command.
- Add Settings → Customization → Extensions/Problems detail and the extension settings page.
- Code entries may be present in the manifest but remain staged and inert until Stage 6/7; install must not run, mount, or start them.
- Add compatibility checks and update/disable/uninstall flows.

Exit criteria:

- Install, enable, disable, update, uninstall, and reinstall preserve or delete data according to the chosen policy.
- A contributed app appears in the dock, web remote `/apps`, and Display Box list in one consistent order; launching it uses only its declared safe launch kind.
- A contributed theme applies on desktop and remote; a contributed screen is reachable and focusable; a contributed setting validates and persists.
- Uninstalling a referenced component either blocks with repair options or completes the repair through a new checkpoint; no crash or silent deletion.
- Permission expansion on update is visible and requires a new grant.
- Core app behavior and `Custom`-last ordering remain unchanged.

### Stage 5 — MCP integration

Scope:

- Implement capability discovery, read, validate, preview, status, and history/reset candidacy for the agent, using the shared service.
- Add the AI access settings surface, grant list, and generic copyable prompt with a separate credential copy action.
- Enforce user-only Keep and token rotation/revocation.
- Coordinate transport/auth details with [CUSTOMIZATION-MCP-PLAN.md](CUSTOMIZATION-MCP-PLAN.md).

Exit criteria:

- An agent can discover installed capabilities and produce a valid rearrangement candidate without repository access.
- A user sees the candidate and must press Keep; no agent-only path commits configuration.
- Stale revisions, disabled launcher, invalid candidates, and revoked tokens return actionable, non-destructive errors.
- History is readable as metadata; restore and reset remain user actions, not MCP tools. The agent cannot read credentials, pairing tokens, executable paths, or private playback history.
- Luma remains fully usable after disconnect; a background-only session can answer read/validate but not fake a preview.

### Stage 6 — Executable UI gate and first runtime

Dependency: G1/G2 evidence and a feature that cannot be expressed declaratively.

Scope:

- Run the runtime spike with one representative widget/screen.
- Prove native-command denial, scoped host actions, identity enforcement, crash isolation, CSP compatibility, focus entry/exit, Back, and cleanup.
- Measure visible/hidden cost, startup delay, and behavior when the launcher is destroyed.
- Write the runtime decision and API contract before adding a general loader.

Exit criteria:

- The chosen runtime meets the agreed UX and resource envelope on the target machine.
- Extension code cannot call core native commands, read pairing secrets, or load remote scripts outside policy.
- A crashing extension leaves core Home, Settings, recovery, and the tray usable.
- If evidence fails, keep declarative extensions and document the constraint instead of weakening the main window.

### Stage 7 — Background services, tray, and remote UI

Dependency: G3/G5 evidence and successful Stage 6 (or an independently justified background-only runtime).

Scope:

- Implement the supervisor, triggers, grants, suspension, crash policy, and resource reporting.
- Add `startup`/`always` grants, tray command contribution if D14 accepts it, and launcher-destroyed behavior.
- Add web remote panels under the chosen isolation model; add Display Box projections only within its protocol.
- Add per-extension data retention/deletion and update compatibility.

Exit criteria:

- A granted background service starts in tray-only/background startup mode without a launcher, survives launcher destruction, and stops on disable/exit.
- A crash restarts within the bound at most the allowed number of times, then disables with a visible reason; the tray and remotes stay usable.
- Disabling or uninstalling stops timers, listeners, processes, and hidden windows; no orphaned work remains.
- The web remote can use the contributed panel without receiving the pairing cookie or arbitrary native authority; unsupported desktop-only controls are explained.
- Measured idle/startup cost meets the budget agreed at the gate on the target machine.
- An app update that incompatibly changes the extension API disables the package without deleting its data or blocking Luma startup.

### Stage 8 — Deferred capabilities

Only after evidence and a separate decision:

- Nesting/grid/overlay composition and pinned extension screens.
- Signed sharing/distribution beyond local installs.
- Background provider contribution into activity; provider credentials and host-mediated network policy.
- Full native shell replacement. **Not accepted in this plan.**

## 11. Acceptance criteria that must be meaningful

These are behaviors to verify, not claims that they pass. Automated tests cannot replace the live TV/phone/tray gates.

### 11.1 Persistence and recovery

- Commit through desktop, web remote, Display Box, and agent paths; every successful write produces one checkpoint or one coalesced checkpoint, never zero or two for the same revision.
- Kill the process during preview, Keep, migration, and history prune; reopening yields either the previous committed document or the new one, never a partial document.
- Restore the oldest retained checkpoint, then undo by restoring the newer one; both create new revisions and keep history bounded at 10.
- Open a database with an unsupported future document; Luma reports the version, does not overwrite it, and remains recoverable.
- Remove an extension between commit and next launch; the document loads degraded, not reset.

### 11.2 Appearance

- Apply a built-in and a contributed theme; all 20 tokens change on desktop, the remote receives a valid palette, and no unvalidated token/CSS reaches the page.
- Exceed the current 8-theme/24-token remote limit; the chosen policy is visible and does not silently truncate the desktop theme list.
- Reset appearance while a custom theme, two-column Home, custom apps, accounts, and a background service exist; the documented visual/layout/widget fields reset; accounts, apps, sounds, and background services remain unchanged.
- Activate a theme whose package was disabled; fallback is legible and Settings names the missing theme.

### 11.3 Composition and navigation

- Default, migrated, one-column, two-column, reordered, renamed-instance, and all-hidden Home states preserve focus and recovery.
- Keyboard D-pad and the phone remote can enter every screen, activate a control, go Back, return focus to the previous control, and reach Settings.
- A hidden or replaced node chooses a nearby usable target, not the top of the page or a dead control.
- Keep review and live use render the same component instances; a preview that only draws a diagram does not satisfy this.
- A 1080p and a 4K layout remain readable at TV distance with visible focus and no obscured controls.

### 11.4 Extensions, apps, and remotes

- Install a pack with a theme, screen, app, and setting; all four appear where expected, and disabling the pack removes all four without losing other configuration.
- Launch a contributed app from desktop, web remote, and Display Box; all three resolve the same identity and no extension-supplied path/argument reaches the OS.
- Disable a package referenced by a screen; repair is offered and no data is silently discarded.
- Revoke a grant; the extension stops performing that operation immediately, including while the launcher is destroyed.
- Attempt a native command, unlisted network host, same-origin remote API call, and cookie read from an extension surface; all are denied and reported.

### 11.5 Background and tray

- Start with `--background`, open no launcher, and let a granted `startup` service run; closing/hiding the launcher does not stop it.
- Hide to tray and reopen; background state and remote availability survive, while launcher rendering does not run hidden.
- Crash the service; bounded restart occurs, then disable with a visible reason; Open/Exit still work.
- Disable, uninstall, and exit; no process, timer, listener, or hidden webview remains.
- Measure startup-to-input, idle CPU, and private memory on the target machine before and after; record numbers and compare to the gate budget.

## 12. Open decisions

| ID | Decision | Recommendation | Impact if unresolved |
| --- | --- | --- | --- |
| D1 | History: every commit vs autosave coalescing; burst window; do remote/desktop share a burst key. | 10 total versions including the current one; every explicit Keep/restore/reset is distinct; coalesce ordinary autosaves by origin/client for 60 seconds. | History fills with slider changes or loses meaningful checkpoints. |
| D2 | Does "appearance" include sound set/level? | No. Reset appearance changes palette/typography/density/tile/motion plus Home layout and widget visibility/placement; reset sound stays separate. | Reset semantics surprise users and may mute/alter audio unexpectedly. |
| D3 | New top-level Customization/Extensions category vs sections under existing categories; may packages add settings sections elsewhere? | One new **Customization** category shared with the MCP plan; package settings default to the extension detail; optional core-approved section later. | Broad extension management becomes buried or the top nav grows without control. |
| D4 | Extension screens: hub-only vs pinnable top navigation; cap. | Hub by default; allow pinning at most 2, with explicit user action and width checks. | Nav readability and Back behavior on TV. |
| D5 | Unify Home into generic composition now vs add a parallel custom-screen engine. | One registry/renderer and a Home profile; migrate in one versioned step. | Two renderers drift; preview and live disagree; duplicate focus bugs. |
| D6 | Executable UI runtime (child WebView vs sandboxed iframe vs process/window). | Timeboxed spike; prefer a child WebView with its own capabilities if TV focus is acceptable. | Code isolation, focus, CSP, and packaging cannot be implemented safely. |
| D7 | Executable web-remote panels vs declarative-only first. | Declarative first; executable panels only with isolated origin and host-relayed actions. | Same-origin extension code could inherit remote command authority. |
| D8 | Extension app launch kinds; whether extensions may request executable launch at all. | URL/screen first. For executable, the user selects the `.exe`; the extension never supplies a path. | Path/argument injection and a false sense of sandboxing. |
| D9 | Custom theme capacity and remote appearance payload; fonts. | Send built-ins + active/pinned custom themes; raise the cap only with a measured payload limit. Treat fonts as a separate asset gate. | Remote theme picker truncates or payload validation rejects valid desktops. |
| D10 | Package trust/signing and local sharing. | Follow [the distribution plan](CUSTOMIZATION-DISTRIBUTION.md): local file import first, content hashes, explicit review, no marketplace; publisher signing is optional future catalog work, not a prerequisite for family file sharing. | Broad permissions on unverifiable code, or a sharing flow that claims more trust than it has. |
| D11 | Extension data retention on uninstall; update rollback window. | Keep settings/data by default with explicit delete; retain one previous version until a successful start. | Silent data loss or orphaned code. |
| D12 | Credential broker vs extension-owned secrets. | Host broker; extension receives scoped operations or its own isolated secret, never core tokens. | One extension can impersonate another service or read core accounts. |
| D13 | Background triggers and default budgets; `always` vs `startup`. | Default `launcher-open`; `startup`/`always` opt-in with visible warning and measured budgets. | Tray becomes a resource drain or extensions silently run forever. |
| D14 | Tray contributions. | Host-owned tray; at most one host-rendered command per extension; no replacement of Open/Exit. | Extensions can confuse recovery or hide core actions. |
| D15 | MCP transport/auth and whether the shipped full-trust HTTP token remains. | Separate scoped agent grants; user-only Keep; keep the direct token as an explicitly labeled developer mode or remove it. | Agent could auto-apply, violating accepted intent. |
| D16 | Full native shell replacement. | Not accepted in this plan. Reopen only as a separate product/repo decision. | Feature requests would otherwise assume a runtime that does not exist. |

## 13. Risks and mitigations

| Risk | Why it matters | Mitigation |
| --- | --- | --- |
| Focus across isolated surfaces | Child webviews/iframes break the single focus tree; Back may close the wrong layer. | Make focus entry/return part of the runtime gate; never ship a surface that cannot return to a known control. |
| Configuration churn | Autosave plus agents plus extensions can create frequent revisions and conflicts. | One native owner, expected revisions, coalesced history, serialized frontend writes, explicit conflict feedback. |
| Schema migration breadth | v3 → v4 touches every consumer: API, remote, Display Box, tests, localStorage mirror. | One atomic migration, migrated current + previous, compatibility endpoints, no mixed v3/v4 writers. |
| App registry drift | Rust catalog, `AppId`, icons, activity mapping, remote `/apps`, and Display Box all encode apps. | One host registry projected to all surfaces; a drift test for IDs and order. |
| Extension uninstall breaks layout | Qualified references can disappear. | Degraded load, repair flow, block/warn on uninstall, never silently drop nodes. |
| Remote trust | Same-origin remote code could call authenticated commands. | Declarative first; isolated panel origin; host-relayed actions; keep pairing cookie out of extension code. |
| Supply chain | Broad grants on unsigned code. | Explicit manifests, content integrity, user review, version-scoped grants, resource caps, and no automatic updates; publisher signing is future catalog work. |
| Performance regression | Background services and hidden webviews consume idle resources. | Measured baseline, gate budgets, triggers/suspension, no polling, stop on disable/exit. |
| Consent fatigue | Permissions plus Keep plus install prompts can overwhelm. | Bundle grants by capability with plain-language scope; Keep lists only changed areas; default least privilege. |
| Documentation drift | MCP, Display Box, and layer contracts evolve in parallel. | This file owns cross-layer rules; companion plans own their detail; checkpoint updates at every stage. |

## 14. Verification strategy

This planning change needs no application build. Implementation should use the smallest relevant checks at each stage:

- Native Rust tests for history retention/pruning, restore-as-new-revision, reset scope isolation, migration v3→v4, degraded load, install transactions, grant revocation, supervisor stop/restart, and compatibility disable.
- Frontend Node tests for `home.test.mjs` migration/order helpers, `homeRenderer.test.mjs` rendering and instance identity, `customization.test.mjs` hydration/conflict/teardown, `settings.test.mjs` contributed-setting validation, and `navigation.test.mjs` focus loss/return.
- Remote tests for the appearance payload, extension-app catalog/command mapping, remote panel entitlement, and refusal of same-origin extension authority.
- `npm run check`, focused `npm test`, and `cargo test --manifest-path src-tauri/Cargo.toml` for changed native modules. Run a production build when packaging or rendered assets change.
- Real Tauri checks: tray destroy/reopen, background startup (`--background`), preview/Keep/cancel, revision conflict, failed write, extension disable/uninstall, crash containment, and app-update compatibility.
- TV and phone checks at 1080p and 4K: readable focus, reachable Settings, predictable Back, screen entry/return, all-hidden Home recovery, and remote parity.
- Measure startup-to-input, idle CPU, private memory, mounted subscriptions, and repeated mount/unmount growth on the same target machine before and after a stage. Record actual numbers; do not claim equivalence without them.
- Verify export privacy by inspecting a produced pack for credentials, tokens, absolute paths, history, and pairing data.

## 15. Relationship to existing documents

- [CUSTOMIZATION.md](CUSTOMIZATION.md) — assessment of the current foundation and open product decisions. This plan supersedes its one-previous-revision and Home-only assumptions where the accepted intent is broader, but does not retroactively claim those features shipped.
- [CUSTOMIZATION-ROADMAP.md](CUSTOMIZATION-ROADMAP.md) — phase history and earlier gates. Stages here extend that pathway; update both at each checkpoint so they do not disagree.
- [CUSTOMIZATION-SERVER.md](CUSTOMIZATION-SERVER.md) — shipped local REST API and its current full-trust token. §6.6 and D15 define the separate agent permission model.
- [CUSTOMIZATION-MCP-PLAN.md](CUSTOMIZATION-MCP-PLAN.md) — authoritative transport, authentication, tool schema, and final generic setup-prompt wording. Cross-layer requirements are in §6.6–§6.7.
- [CUSTOMIZATION-DISPLAY-BOX.md](CUSTOMIZATION-DISPLAY-BOX.md) — authoritative frame client and protocol detail. Cross-layer expectations are in §5.7 and §4.1.
- [CUSTOMIZATION-DISTRIBUTION.md](CUSTOMIZATION-DISTRIBUTION.md) — package sharing, bundled examples, and theme-pack boundaries. Cross-layer packaging expectations are in §9 and D10.
- [DEVELOPMENT.md](DEVELOPMENT.md) — current architecture, tray, remote, and validation notes. Update it when a stage ships; this plan is not a substitute for shipped-behavior documentation.

The MCP and Display Box plans are authored in parallel with this file. Their filenames above are the canonical links; if either lands under a different name, update every reference in this document in the same change.

## 16. Checkpoint template

At the end of each stage, replace status prose with facts:

```text
Stage: N — name
Date:
Shipped:
  - evidence (file/module/test)
Not shipped / still proposed:
  - item and reason
Decisions closed:
  - D#: answer, date
Decisions still open:
  - D#: why it still blocks
Measurements:
  - machine, build, startup-to-input, idle CPU, memory, method
Verification run:
  - command and result (only what actually ran)
Remaining manual gates:
  - TV/phone/tray/packaging checks not performed
Documentation updated:
  - this file, roadmap, DEVELOPMENT, companion plans
```

A stage is not complete because the code exists. It is complete when its exit criteria and the applicable manual gates have evidence, and the shipped/proposed distinction in this file is current.
