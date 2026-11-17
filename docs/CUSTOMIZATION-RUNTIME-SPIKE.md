# Stage 6 executable UI runtime: spike contract and acceptance record

Status (2026-11-16): the Stage 6/7 package runtime, explicit grants, background supervisor, desktop controls, and declarative remote panels are implemented. Review and live acceptance are in progress. Historical development-only checkpoints below describe their recorded versions; they do not describe the current activation path. No unrecorded live or performance gate is claimed passed.

Assessed against the working tree at commit `e862e17` with concurrent sports/remote edits present. Re-read the cited symbols before implementing; line numbers will drift.

## 1. How to read this record

| Label | Meaning |
| --- | --- |
| **Static** | Read from the current working tree. The symbol exists; the behavior was not exercised. |
| **Tauri constraint** | Stated by the official Tauri 2.12.5 documentation or API reference. |
| **Contract** | A requirement proposed for the spike. It is not accepted implementation detail until the spike passes. |
| **Live check** | A real Tauri/TV/phone/tray check. Every row is **Not run** unless it says otherwise. |
| **Not established** | A claim this record deliberately does not make. |

The checkout pins `tauri 2.12.5` in `src-tauri/Cargo.lock`; the official references below were read at that version. Static source evidence is cited by file and symbol, not only by line.

Two evidence boundaries apply throughout:

- No Luma process was launched, no build, test, lint, or benchmark was run, and no extension code was executed for this record.
- A capability file, CSP string, or handler that exists in source does not prove the runtime denies a call. Only the live checks do.

The canonical plan names a weather widget and a music screen as representative acceptance cases. This record treats both as required: a display-only widget does not exercise existing playback, actions, or account boundaries, and a music screen alone does not exercise external data or network policy.

## 2. Stage 6 exit condition and current decision

The canonical plan requires the spike to prove native-command denial, scoped host actions, identity enforcement, crash isolation, CSP compatibility, focus entry/exit, Back, and cleanup, then measure visible/hidden cost and launcher-destroyed behavior. If evidence fails, declarative extensions remain and the constraint is documented instead of weakening the main window (`docs/CUSTOMIZATION-LAYERS.md`, Stage 6).

Current decision status:

| Item | Status |
| --- | --- |
| G2 / D6 runtime choice | **Open.** No candidate is selected. |
| Executable UI shipped | **No.** |
| Any executable package enableable | **No.** `runtime.kind` other than `declarative` installs disabled and cannot be enabled (`docs/PLUGINS.md`, "Future executable runtime"). |
| Live acceptance evidence | **None.** §10 lists every required check as Not run. |
| Representative cases ready to run | **Partly.** The installed weather example package exists and loads through a development-only package runtime (§20). The music screen is still a proposed use of existing playback, not a package surface. |
| Resource budget | **Not agreed.** The plan supplies a provisional idle-CPU envelope only; memory and interaction budgets require a measured baseline. |

Passing this record is necessary, not sufficient, for a general package loader. Package signing (G4), background services (G3/Stage 7), and executable remote panels (G5) are outside this record.

## 3. Static baseline

### 3.1 The declarative gate that already exists

| Static fact | Evidence | Consequence for Stage 6 |
| --- | --- | --- |
| The manifest accepts `declarative`, `isolated-ui`, `background`, and `main-page`, plus `runtime.entry` for non-declarative kinds. | `src-tauri/src/customization/plugin.rs`, `RuntimeKind`, `Runtime`, `validate_manifest`. | The package shape already has a place to hang a runtime, but no execution path reads `entry`. |
| Only `declarative` is enableable. A non-declarative package gets an execution reason, installs disabled, shows as executable, and cannot be enabled. Preview returns `plugin_unsupported`. | `plugin.rs`, `execution_reason`, `prepare_enable`, `apply_action`, `state`; test `unsupported_runtime_packages_install_disabled_and_cannot_enable`. | The current product behavior is the correct default until the gate passes. |
| Install and update write `enabled = 0`; update also preserves compatible setting values and clears detach state for review. | `plugin.rs`, `install`. | Executable packages can be staged today without running; a future runtime must keep this two-step consent. |
| The manifest is strict (`deny_unknown_fields`) and has no `permissions`, `network`, or surface declaration. | `plugin.rs`, `Manifest`, `Contributions`. | Any grant or allowlist contract requires an explicit manifest/API version change. It cannot be added silently to `apiVersion: 1`. |
| Archive validation rejects executable binary extensions (`.exe`, `.dll`, `.msi`, `.bat`, `.cmd`, `.ps1`, `.scr`, `.com`, `.cpl`) and Windows-reserved paths. | `plugin.rs`, `validate_relative_path`; `docs/PLUGINS.md`, "Limits and archive checks". | Executable UI code must be packaged text/assets; native sidecars and DLL loading remain outside this contract. |
| MCP advertises `declarative` as supported and the other kinds as installed-disabled. | `src-tauri/src/customization/mcp.rs`, capability document; `docs/PLUGINS.md`, MCP resources. | Agent-facing capability discovery already tells the truth; it must not be widened before the gate passes. |
| Settings UI labels a non-declarative package "Runtime not supported" and disables Enable. | `src/lib/components/PluginSettings.svelte`. | The review surface exists; an executable grant review would be a new step, not a reuse of Keep. |

### 3.2 Platform boundary in this checkout

| Static fact | Evidence | Consequence |
| --- | --- | --- |
| The only capability file grants `core:default`, updater permissions, and limited window permissions to `main`. | `src-tauri/capabilities/default.json`. | No extension surface currently has a capability. Do not add `windows: ["*"]`; it would merge `main` authority into every surface. |
| `app.security.capabilities` is not set, so every file under `capabilities/` is included automatically. | `src-tauri/tauri.conf.json`; Tauri capabilities reference. | A new extension capability file becomes active as soon as it exists. It must be written narrowly from the start. |
| CSP is application-wide and strict: `script-src 'self'`, `style-src 'self' 'unsafe-inline'`, `img-src ... https:`, `connect-src` limited to self, IPC, and Cinemeta, `font-src 'self'`, `object-src 'none'`, `frame-ancestors 'none'`. There is no `frame-src`, so frames fall back to `default-src 'self'`. | `src-tauri/tauri.conf.json`, `app.security.csp`. | A same-origin iframe is not an isolation boundary; a cross-origin or `data:`/`blob:` frame is blocked by the current policy. `img-src https:` still permits remote image loads, so a surface CSP must narrow image sources or the contract must treat images as an exfiltration channel. Any CSP change must be proven on the extension surface without relaxing `script-src` for `main`. |
| `tauri` declares the `tray-icon` feature and does not enable the `unstable` feature. | `src-tauri/Cargo.toml`; `Cargo.lock`. | Stable `WebviewWindow` is available. Child webviews inside one window (`Webview`/`add_child`) are not. |
| `build.rs` calls `tauri_build::build()` with no application command manifest. | `src-tauri/build.rs`. | Tauri's documented default applies: commands registered only through `invoke_handler` are callable by every window/webview. A no-capability label is not proof that Luma's custom commands are denied. |
| `invoke_handler` registers the customization, Spotify, app-launch, remote, activity, Stremio, Steam, YouTube, sports, updater, tray, and TV commands. | `src-tauri/src/lib.rs`. | Command denial is the largest pre-spike gap. It requires a build-time command manifest plus capability scopes before any extension webview exists. |
| Existing external surfaces use stable `WebviewWindowBuilder::new` with `WebviewUrl::External`, an `on_navigation` allowlist, initialization scripts, and host-owned windows. Stremio also denies new windows. | `src-tauri/src/commands/youtube.rs`, `stremio.rs`, `sports.rs`. | There is a working precedent for navigation policy and injected bridges, but these surfaces are not a sandbox and were not designed as third-party code hosts. |
| The YouTube bridge is a loopback HTTP server on a random port with a 32-byte token and a 16 KiB body limit. | `youtube.rs`, `start_bridge`. | Host-brokered I/O is an existing pattern. It does not grant Tauri IPC, but it is still a network surface that must be scoped per extension. |
| The web remote uses an `HttpOnly`, `SameSite=Strict` cookie and serves on port 9877; the customization HTTP server uses 9878 and MCP uses 9879, both on loopback. | `src-tauri/src/frame.rs`; `src-tauri/src/customization/server.rs`, `mcp.rs`; `docs/DEVELOPMENT.md`, "Phone remote"; `docs/CUSTOMIZATION-SERVER.md`. | Extension navigation and network policy must explicitly block all three services, including LAN addresses on 9877. Origin isolation alone is not sufficient evidence that a cookie cannot be sent or a command reached. |
| The launcher is destroyed on hide-to-tray: `hide_to_tray` unloads `main`, YouTube, Stremio, and sports; `dismiss`/`present` may retain YouTube and Stremio when fast app switching is enabled. | `src-tauri/src/background.rs`, `hide_to_tray`, `dismiss`, `present`; `docs/DEVELOPMENT.md`, "System tray and Windows startup". | A UI runtime must not assume a mounted Svelte component or a live launcher. Stage 6 should destroy extension surfaces with the launcher; background work is Stage 7. |

### 3.3 Focus, Back, and navigation baseline

| Static fact | Evidence | Consequence |
| --- | --- | --- |
| The shell switches screens with a `tab` string (`Home`, `Search`, `Watch`, `Sports`, `Library`, `Music`, `Apps`); Settings, details, player, and dialogs are state flags. | `src/App.svelte`; `src/lib/components/TopNav.svelte`. | There is no screen registry or route table. An extension screen needs a host-owned destination and a host-owned exit path, not a new `tab` string written by the package. |
| Back is host-prioritized: review, exit, pairing, Settings, details, player overlay, sports detail, music detail, then Home. | `src/App.svelte`, `back`. | An extension surface must fit this order and must not create a second, competing Back stack. |
| The player remembers the trigger element, focuses `.now-playing-close` on open, and restores the trigger on close. | `src/App.svelte`, `openPlayer`, `closePlayer`. | This is the minimum focus-return contract for an extension surface. |
| Home focus recovery remembers instance ID, focus key, control index, and row index; it falls back within the same row, then the nearest row, then `#home-button`. | `src/lib/services/navigation.ts`, `preserveHomeFocus`. | A widget must expose stable instance and control identity if it owns focus. |
| Directional navigation is geometry-based within `dialog[open]` or `main`, with special cases for settings, rails, and selects. | `navigation.ts`, `createNavigation`. | A separate native webview is outside this DOM. Entry, directional forwarding, Back, and return must be host-coordinated. |
| The phone remote routes commands first to sports, a launched app, Stremio, or YouTube, then to `main`. | `src-tauri/src/remote/server.rs`, `navigate`; `src-tauri/src/frame.rs`. | A new extension surface must be added to this router before the phone remote can operate it. Otherwise directional input goes to `main` while the extension is visible. |
| Existing embedded surfaces receive commands through host `eval` hooks such as `window.__lumaNavigate` and `window.__lumaSportsNavigate`; Home closes the surface. | `youtube.rs`, `navigate`/`forward`; `sports.rs`, `forward`. | A host-to-surface navigation channel is an established pattern. It is not a security boundary by itself; the surface must validate its own inputs. |

### 3.4 Existing playback ownership

| Static fact | Evidence | Consequence |
| --- | --- | --- |
| Playback belongs to the native Spotify `Player`; the frontend store subscribes to playback, artwork, and connection events. | `src-tauri/src/integrations/spotify/player.rs`; `src/lib/state/spotify.svelte.ts`, `initialize`. | A music extension screen must consume this owner, not create a second player or auth session. |
| Elapsed-time ticking runs only while playing **and** the app is visible; `dispose` clears the timer and subscriptions. | `spotify.svelte.ts`, `updateTimer`, `dispose`. | This is the model for an extension: event-driven snapshots plus a visibility-gated display tick, not background polling. |
| Track actions are parsed and bounded: `toggle`, `play`, `pause`, `next`, `previous`, `seek`, `shuffle`, `repeat`. | `src-tauri/src/integrations/spotify/models.rs`, `parse_command`; `src-tauri/src/integrations/spotify/mod.rs`, `spotify_control`. | A scoped playback host action can expose a small action enum without exposing account, device, or credential operations. |
| Music detail is a local stack: `MusicHome` remembers the previous element and restores it when a collection closes. | `src/lib/components/spotify/MusicHome.svelte`, `browse`, `closeDetail`. | A music extension screen must support the same local-layer Back behavior before the host closes the screen. |
| `NowPlaying` is a host player overlay; Back closes its own overlay before the host closes the player. | `src/App.svelte`, `back`/`closePlayer`; `src/lib/components/spotify/NowPlaying.svelte`. | A music extension screen must not assume Back closes the core player, and must not replace the host player overlay. |
| Music entry from the shell focuses a music action or control, not the top of the page. | `src/App.svelte`, `openMusic`; `navigation.ts`, `focusMusic`. | Entry focus is a product requirement, not an implementation detail. |

### 3.5 The weather case does not exist yet

| Static fact | Evidence | Consequence |
| --- | --- | --- |
| The only widget definition is Spotify with four placements. | `src/lib/services/widgets.ts`; `src/lib/components/WidgetHost.svelte`. | A weather widget is a representative acceptance case, not shipped functionality. |
| The widget ID type is the literal `'spotify'`, and settings store a record keyed by it. | `src/lib/types/index.ts`, `WidgetId`, `Settings['widgets']`. | A weather widget requires a host type and settings-schema change, not only a package contribution. |
| No weather provider, weather settings, or weather network origin exists. | Repository search; CSP `connect-src`. | The case must be built for the spike; it cannot be validated by toggling an existing feature. |
| Widgets are statically imported Svelte components; there is no package component loader. | `widgets.ts`; `WidgetHost.svelte`. | Executable widget loading is new runtime work, not a manifest change. |

## 4. Tauri constraints that shape the contract

These are constraints from the official Tauri documentation, not Luma behavior claims.

| Constraint | Official source | Contract implication |
| --- | --- | --- |
| Capabilities target windows/webviews by **label**. A window match applies to all its webviews; `webviews` gives finer control. Multiple matching capabilities merge. A webview matching no capability has no ACL-gated IPC access. | [Capabilities](https://v2.tauri.app/security/capabilities/); [Capability reference](https://v2.tauri.app/reference/config/#capability) | Give extension surfaces unique labels and a dedicated capability. Never use `windows: ["*"]`. Remember that labels accept only alphanumerics plus `-`, `/`, `:`, `_`; plugin IDs contain dots, so the host must map a generated label to the package ID. |
| Commands registered only through `invoke_handler` are allowed to all windows/webviews by default. `AppManifest::commands` makes application commands subject to permissions/capabilities. | [Capabilities](https://v2.tauri.app/security/capabilities/) | **Pre-spike blocker.** Register every Luma command, grant a main-only permission set, and grant the extension label only the relay command. Then test denial. A capability file alone does not deny Luma's custom commands today. |
| Remote content does not get API access by default. A capability must opt in with `remote.urls`. | [Capabilities](https://v2.tauri.app/security/capabilities/) | Never add `remote.urls` to an extension capability. The extension entry must be app-served, not a remote origin with local authority. |
| CSP is configured application-wide. Tauri hashes/nonces bundled scripts and styles. The documented per-response hook (`on_web_resource_request`) can rewrite a `tauri` protocol response, but it is not a first-class per-window CSP setting. | [CSP](https://v2.tauri.app/security/csp/); [WebviewWindowBuilder](https://docs.rs/tauri/latest/tauri/webview/struct.WebviewWindowBuilder.html) | Per-surface CSP must be measured, not assumed. If a custom protocol serves the extension entry, prove which CSP header actually arrives. Never relax `script-src` for `main`. |
| `WebviewUrl` supports `App` (app path, same app origin), `CustomProtocol`, and `External` (HTTP/HTTPS only). | [`WebviewUrl`](https://docs.rs/tauri/latest/tauri/enum.WebviewUrl.html) | `App` is not an origin boundary. A distinct custom protocol or an external loopback origin is required for isolation; each needs its own handler and validation. |
| A window hosting one webview (`WebviewWindow`) is stable. Child webviews inside one window require the `unstable` Cargo feature; without it creation is rejected. | [Webview JS API](https://v2.tauri.app/reference/javascript/api/namespacewebview/); `Cargo.toml` | The plan's preferred "child WebView" is not available in this checkout as an embedded surface. The stable alternative is a separate native window, which changes focus, Back, and phone-remote behavior. |
| `on_navigation` can cancel navigation; `on_new_window` decides whether `window.open` creates a window. | [WebviewWindowBuilder](https://docs.rs/tauri/latest/tauri/webview/struct.WebviewWindowBuilder.html) | Both handlers are mandatory. Allow only the extension's own origin and explicit host-opened URLs; deny everything else, including loopback services, `file:`, `data:`, `blob:`, and unknown hosts. |
| Initialization scripts run on top-level navigations; on Windows they are also added to subframes. | [WebviewWindowBuilder](https://docs.rs/tauri/latest/tauri/webview/struct.WebviewWindowBuilder.html) | Guard the host bridge by `window.location` and frame identity. Do not expose the bridge to untrusted child frames. |
| Window creation on Windows can deadlock in a synchronous command or event handler; use async commands or a separate thread. | [WebviewWindowBuilder](https://docs.rs/tauri/latest/tauri/webview/struct.WebviewWindowBuilder.html), "Known issues" | Surface creation must not block the launcher. Use an async path, a timeout, and a host fallback that keeps focus usable. |
| Background throttling policies exist for webviews not in a window. | [Configuration](https://v2.tauri.app/reference/config/#backgroundthrottlingpolicy) | Do not use a hidden extension webview as a timer or background service. Stop extension work when hidden; background triggers are Stage 7. |
| The isolation pattern intercepts IPC through a sandboxed iframe, encrypts messages, and has Windows limitations (including no ES modules in the isolation application). | [Isolation pattern](https://v2.tauri.app/concept/inter-process-communication/isolation/) | It can harden IPC validation, but it does not isolate third-party UI or replace origin isolation. It is not selected by this record. |
| The runtime authority denies ACL-gated calls before the command runs, but capabilities do not protect against malicious Rust, incorrect command scope checks, intentional Rust bypasses, or WebView 0-days. | [Runtime authority](https://v2.tauri.app/security/runtime-authority/); [Capabilities](https://v2.tauri.app/security/capabilities/) | Treat capability scoping as one layer. The contract also needs host authorization by package identity, bounded data, navigation policy, and a documented residual risk. |

## 5. Proposed spike contract

Everything in this section is a proposal for the spike, not shipped behavior. Candidate mechanisms are listed so the spike can reject them with evidence; nothing is selected.

### 5.1 Candidate runtime shapes to compare

| Candidate | Status in this checkout | Why it is a candidate | What must be proven before it can be selected |
| --- | --- | --- | --- |
| **A. Stable `WebviewWindow` with a dedicated extension label and capability** | Available. | Uses stable Tauri APIs; separate native window gives process/focus containment; navigation and new-window handlers are available. | It can be positioned and owned as a TV surface without looking like a second app; focus entry/return; phone remote routing; Back remains host-authoritative; main stays usable if it crashes. |
| **B. Child webview inside the main window (multiwebview)** | Not available: `unstable` feature is off. | Would embed a webview in the existing window and keep one native focus tree. | Requires enabling an unstable API; layout/focus/clipping behavior; capability targeting by `webviews`; crash behavior. Do not enable the feature merely to make the spike easier. |
| **C. Sandboxed iframe inside the main WebView** | Blocked by current CSP for cross-origin/`data:`/`blob:` frames; a same-origin iframe shares the main origin and trust. | No new native window; could reuse host layout. | Proven origin isolation, not just `sandbox`; a CSP `frame-src` change; no access to main DOM, storage, or IPC; Back/focus behavior inside the frame. If it is same-origin, it fails the isolation requirement. |
| **D. `main-page` scripts** | Must remain unsupported. | None. | Nothing. This is explicitly excluded by the product boundary; third-party code must not share the main WebView's trust. |

Selection rule: a candidate is selected only when every applicable live row in §10 passes. If none passes, keep declarative extensions and record the blocking constraint. Do not weaken `main` to make a candidate fit.

### 5.2 Identity and grants

- The host owns a mapping from a generated surface label to one installed package ID and version. The label must be generated from a validated package ID plus a random or hashed suffix, because Tauri labels cannot contain the dots used by plugin IDs.
- The extension never supplies its own package ID, permissions, or capability. The host resolves identity from the installed record and the webview label. Manifest `permissions` in this document are package grants reviewed by the user, not Tauri ACL identifiers; a package cannot request `core:*` or a capability name.
- One relay command is exposed to extension surfaces, for example `extension_rpc`. It accepts `{ op, args }`; it does not accept a command name, package ID, URL, filesystem path, or credential.
- The host injects a minimal `window.luma` request/subscribe client through an origin-guarded initialization script. The package does not rely on `window.__TAURI__`, `withGlobalTauri`, or a bundled Tauri API.
- The host authorizes each `op` against the package's version-scoped grant. A payload that claims another package or an ungranted op is rejected with `denied`.
- Install remains disabled. Enabling executable code is a separate consent that lists package identity, version, origin, runtime kind, permissions, network origins, and the representative surface. Visual Keep is not code activation.
- Permission expansion on update requires a new visible grant. Downgrade/rollback does not silently restore broader grants.

### 5.3 Manifest additions (versioned, not v1)

The current `Manifest` rejects unknown fields, so the following shape requires a new `apiVersion` (or an explicitly versioned runtime block) and migration. Do not add fields to `apiVersion: 1` and hope old clients ignore them.

```json
{
  "id": "com.example.weather",
  "version": "1.0.0",
  "apiVersion": 2,
  "runtime": { "kind": "isolated-ui", "entry": "ui/index.html" },
  "permissions": ["ui.widget", "settings.own", "network.allowlist"],
  "network": { "allow": ["https://api.weather.example"] },
  "settings": [
    { "key": "location", "type": "string", "default": "Seattle", "maxLength": 80 },
    { "key": "units", "type": "enum", "default": "c", "options": [
      { "value": "c", "label": "Celsius" },
      { "value": "f", "label": "Fahrenheit" }
    ] },
    { "key": "refreshMinutes", "type": "number", "default": 30, "min": 15, "max": 180, "integer": true }
  ]
}
```

Rules:

- `runtime.kind: "isolated-ui"` is the only executable UI kind in scope. `background` is Stage 7; `main-page` remains rejected.
- Permissions are exact host names, not wildcards. Unknown names reject the package. The names above are proposed; define them with the manifest/API version and keep them separate from MCP grant scopes.
- `network.allow` is an HTTPS origin allowlist for **host-mediated** requests, not a licence for the extension to fetch directly. No redirects to origins outside the list; no loopback, link-local, or private-network targets.
- The entry and all assets are package-relative, validated, and served by the host. No remote scripts, no CDN, no `eval`, no inline script requirement.
- Settings are host-rendered and host-stored. A secret, if a provider needs one, goes through the host credential broker and never appears in the manifest, document, history, export, or extension payload (`docs/CUSTOMIZATION-LAYERS.md`, D12).

### 5.4 Capability file contract

Proposed shape, not an implemented file:

```json
{
  "identifier": "extension-surface",
  "description": "Minimal relay access for executable plugin UI surfaces.",
  "windows": ["ext-*"],
  "permissions": ["allow-extension-rpc"],
  "local": true
}
```

- `allow-extension-rpc` is an application permission generated from the build-time command manifest. It allows exactly `extension_rpc`.
- The entry is served from a distinct host-owned origin (a dedicated custom protocol or an equivalent proven origin). `WebviewUrl::App` is not acceptable for executable UI because it shares the main app origin.
- If the relay uses Tauri IPC, the surface CSP must retain the IPC connect source used by this Tauri version (the app currently uses `ipc:` and `http://ipc.localhost`) and must not add data origins. If a custom protocol supplies its own CSP, verify that the relay still works; a policy that blocks IPC also blocks the intended host actions.
- No `remote` block. No `core:*`, no plugin permissions, no updater, no dialog, no opener, no shell, no filesystem.
- All other application commands must be denied by omission **after** they are registered in `AppManifest::commands` and granted only to `main` (and any existing host surface that genuinely needs them). Today the build has no such manifest, so this is a prerequisite code change for the spike, not something this record claims exists.
- If updates need to be pushed from host to surface, prefer a `tauri::ipc::Channel` created by the extension and passed to the relay subscribe call. If Channel requires an event permission, record that and choose an alternative host push; do not grant a general event bus just to obtain push. Listening to arbitrary Luma events is denied.

### 5.5 Explicitly denied capabilities and authority

This is the deny list the spike must enforce and test. "Denied" means the extension surface must fail the call, not merely lack a UI affordance.

| Authority | Required state | Enforcement layer | Live proof |
| --- | --- | --- | --- |
| All `core:*` permissions | Denied | Capability omission; build-time command manifest for app commands | Invoke core commands from the surface; expect rejection |
| `core:webview:allow-create-webview`, `allow-create-webview-window`, `allow-reparent` | Denied | Capability omission; `on_new_window` | Try to create/reparent a webview or window |
| `core:window:*` (focus, fullscreen, close, always-on-top, set-size) | Denied | Capability omission; host owns the window | Try to move, resize, focus-steal, or fullscreen the host |
| `core:event:*` | Denied | Capability omission; relay Channel or host push instead | Try to `listen()` to `customization-changed`, playback, or remote events |
| `core:app:*`, `core:path:*`, `core:resources:*`, `core:menu:*`, `core:tray:*` | Denied | Capability omission | Try to read paths/resources or alter menus/tray |
| Updater, dialog, opener, fs, shell, process, http, websocket | Denied | Capability omission | Try to update, open a file/URL, read a path, spawn a process |
| Autostart, single-instance, global-shortcut, notification, clipboard, store, deep-link | Denied | Capability omission | Try to register a shortcut, read the clipboard, or persist arbitrary state |
| Devtools | Denied in release; debug-only if needed | No permission; build profile | Confirm release surface has no devtools affordance |
| `remote.urls` | Absent | Capability has no remote block | Serve the entry from a remote origin and confirm no API access |
| `asset:` protocol scope | Empty/not granted | `app.security.assetProtocol`; no asset URL in the surface | Try to load `asset://` paths outside the package |
| `dangerousDisableAssetCspModification` | Not enabled | Configuration | Inspect the actual CSP header |
| Direct network to any origin | Denied | CSP `connect-src`; host fetch only | Try `fetch` to the weather origin and to loopback |
| Remote images, beacons, WebSocket, `sendBeacon` | Denied on the extension surface | Surface CSP `img-src`/`connect-src`; host-served assets; no remote URLs in markup | Load a remote image with query data, use `sendBeacon`, and open a WebSocket; inspect requests |
| Web remote `:9877` (loopback and LAN), customization `:9878`, MCP `:9879` | Denied | `on_navigation`; network allowlist; CSP | Navigate and fetch; inspect cookies and responses |
| Pairing cookie, Spotify/Stremio tokens, app paths, history | Denied | No command returns them; separate origin/storage; relay allowlist | Attempt to read cookies, storage, and relay fields |
| Arbitrary native command or DLL/shell execution | Denied | No shell/process permission; no native code loading; package validation | Attempt invoke and package an executable; expect rejection |
| `main` DOM, `localStorage`, `IndexedDB`, `window.opener` | Denied | Separate origin/window; origin isolation; no same-origin frame | Try to reach parent/opener or main storage |
| Remote scripts, `eval`, `data:`/`blob:` script loads | Denied | CSP `script-src 'self'`; packaged entry only | Try a remote `<script>`, dynamic import, and `eval` |
| Background execution, timers while hidden, startup work | Denied for Stage 6 | Surface destroyed on hide; no background trigger; Stage 7 owns services | Hide to tray and inspect processes/timers/network |

Residual risk to state in the decision: a compromised WebView2 renderer or a WebView 0-day is outside Tauri's capability boundary. The runtime must not be described as a perfect sandbox.

### 5.6 Relay API

Request:

```json
{ "op": "weather.read", "args": { "force": false } }
```

Response:

```json
{ "ok": true, "data": { "temperature": 12, "units": "c", "observedAt": "..." } }
```

Rules:

- One request in flight per surface; bounded request size (for example 16 KiB) and bounded response size (for example 256 KiB).
- Host timeout (for example 5 s for weather, 2 s for playback) returns a typed error; the surface must render it, not spin.
- Ops are exact strings. No `command`, `url`, `path`, `token`, `query`, or `headers` passthrough.
- Errors use stable codes: `denied`, `invalid`, `unavailable`, `timeout`, `stale`, `busy`, `revoked`, `crashed`.
- The host rate-limits control ops (for example one playback control per 250 ms) and coalesces duplicate refreshes.
- The relay never returns core credentials, pairing data, executable paths, history, or another package's settings.

### 5.7 Surface states and host fallback

Every executable surface must define and render:

| State | Host responsibility | Surface responsibility |
| --- | --- | --- |
| Loading | Host shell/chrome remains; Back works | Skeleton or progress, no layout jump that moves focus |
| Empty | Explain what is missing | No fake data; no dead control |
| Stale | Show age and a Retry action | Keep last known data visibly marked stale |
| Error | Host-owned fallback and Retry; focus recovery | Show the typed error; preserve user input |
| Disabled/revoked | Close the surface; explain in Settings | No lingering process, timer, or request |
| Crashed/unresponsive | Reclaim focus after a bounded timeout; show fallback | No assumption that the renderer recovers itself |
| Unsupported runtime | Do not create a surface | Not applicable; package stays disabled |

The host, not the extension, owns the close/Back affordance and the recovery placeholder. An extension cannot make Back dead or hide the only exit.

## 6. Representative case A: weather widget

### 6.1 User journey

1. Import a package. It appears disabled with identity, version, runtime, permissions, and `https://api.weather.example` listed.
2. Review and enable. The grant is recorded; the widget is not created yet.
3. Add the widget to a supported Home placement. The host creates the surface lazily when Home is visible.
4. The surface shows loading, then current conditions. Missing location, offline, timeout, invalid payload, and stale cache each have a distinct state.
5. Back/Home leaves the widget without destroying Home focus. Hiding to tray destroys the surface and stops its refresh timer.
6. Disable or uninstall removes the surface, stops requests, and leaves a recoverable Settings entry/placeholder if Home still references it.

### 6.2 Contract

- **Data path:** the surface calls `weather.read`; the host validates the package's allowlisted origin, performs the HTTPS request with a timeout and response cap, validates the payload schema, and returns only normalized fields. The surface never receives the API key (if one is added) and never performs the fetch itself.
- **Settings:** `location` is a bounded string; `units` is an enum; `refreshMinutes` is an integer with a minimum (proposal: 15). The host validates on write and load. Invalid stored values fall back to defaults with a visible warning.
- **Refresh:** read on first visible mount if the cache is missing or older than the declared interval. While visible, refresh no faster than the declared interval. No timer while hidden, disabled, or in the tray. No background polling. One in-flight request maximum; duplicate mounts share the host cache.
- **Placement:** the host decides which placements accept an executable widget. If the current Home profile only supports the Spotify widget, the spike may use a dedicated screen instead, but it must not pretend the Home widget path is complete.
- **Focus:** display-only by default, so it is skipped by directional navigation. If it has an action, exactly one entry control; entry from the widget instance; Back returns to that instance and control; no nested focus trap.
- **Failure:** host fallback shows why the widget is unavailable and a Retry that is reachable by D-pad. A crash leaves a host placeholder, not a blank Home row.
- **Privacy:** no location or weather history in the customization document, history checkpoints, export, or logs. Cache is bounded and cleared on uninstall or explicit removal.

### 6.3 What the spike must prove for this case

- The widget can load external data without the extension surface having general network or IPC authority.
- The host can deny an unlisted origin, a redirect to an unlisted origin, a loopback target, an oversized response, and a malformed payload.
- The widget has clear loading, empty, stale, error, disabled, and crashed states on a TV at 1080p and 4K.
- Disable/uninstall/hide stops requests and destroys the surface; re-enable does not duplicate a timer or a second host subscription.

## 7. Representative case B: music screen using existing playback

### 7.1 User journey

1. Enable a package that contributes a music screen. The grant lists playback read/control, not account or device authority.
2. Open the screen from the host-owned screen hub or an allowed pinned destination.
3. The host supplies the existing playback snapshot. The screen shows track, artist, artwork, progress, and a small control set.
4. Play/pause/next/previous act through the host relay, which calls the existing Spotify player owner. The surface never authenticates, transfers devices, or opens a second player.
5. Back closes a local detail/list layer first, then the screen, then restores focus to the trigger. Home closes the screen and returns Home.
6. Disable/uninstall closes the screen and removes its subscription. The core Music screen and player are unaffected.

### 7.2 Contract

```json
{
  "id": "com.example.nowplaying",
  "version": "1.0.0",
  "apiVersion": 2,
  "runtime": { "kind": "isolated-ui", "entry": "ui/index.html" },
  "permissions": ["ui.screen", "playback.read", "playback.control"]
}
```

- **Single owner:** Rust `Player` and the frontend Spotify store remain the only playback/auth owners. The extension receives a read-only snapshot plus a bounded control operation. It does not call `spotify_connect`, `spotify_set_client_id`, `spotify_start`, `spotify_transfer`, or any account command.
- **Data:** title, artist, album, artwork reference, duration, position, playing/paused, and connection state only. No tokens, device credentials, queue history, or private library data beyond what the screen displays.
- **Actions:** `play`, `pause`, `toggle`, `next`, `previous`, and bounded `seek`; optionally `shuffle`/`repeat` only if the representative design requires them. If the screen also browses, `playUri` accepts only a validated `spotify:` URI; arbitrary URLs and account changes are rejected.
- **Updates:** host push or a visibility-gated display tick. No polling loop in the extension. When paused or hidden, the progress tick stops. When the screen closes, the subscription ends.
- **States:** not authenticated (host-owned "Connect Spotify" action opens existing account settings, not extension OAuth), starting, no active device, playing, paused, error, disabled, crashed. The screen never fabricates playback.
- **Focus:** entry at the first control or the screen container; left/right moves through the control row; no focus trap; Back returns to the exact trigger; the existing player overlay's Back priority is preserved.
- **No duplicate session:** one player, one auth session, one device selection owner. A second extension screen showing playback does not create a second Spotify client.

### 7.3 What the spike must prove for this case

- The extension can render live playback and issue controls without any Spotify credential, account, or device command.
- Closing/disable/uninstall ends the subscription and leaves the core Music screen and player working.
- Back closes the local layer first, then the screen, and never closes the core player unexpectedly.
- The phone remote and keyboard produce the same focus/activation/Back behavior.

## 8. Focus and Back protocol

This protocol is normative for the spike. The mechanism may differ by candidate; the behavior may not.

### 8.1 Terms

- **Trigger:** the control that opened the surface (widget action, screen tile, top-nav item, phone command).
- **Surface:** the extension-rendered UI.
- **Host shell:** Luma-owned chrome, recovery, and focus authority.
- **Local layer:** a detail, menu, or list inside the surface that should close before the surface exits.

### 8.2 Entry

1. The host validates the trigger and the package grant before creating anything.
2. The host creates or reveals the surface with a host-owned loading state; focus stays on the trigger until the surface reports ready or the timeout expires.
3. On ready, the host focuses the declared entry control. If none is declared, the host focuses the first visible focusable; if none exists, the surface container with an accessible label.
4. The focus ring is visible at TV distance. No control is hover-only. Reduced motion is respected.
5. If creation fails, focus remains on the trigger and the host reports the failure. Focus is never left on `body`.

### 8.3 Back

Back is host-authoritative, even when the surface handles a local layer first.

1. The host routes Back to the active surface.
2. The surface closes one local layer and reports `handled: true`, or reports `handled: false` if it has no local layer.
3. On `false`, timeout, or crash, the host closes the surface, stops its relay/subscription, and restores focus to the trigger.
4. If the trigger no longer exists, the host restores the nearest Home instance/control, then `#home-button`. It does not jump to the top of the page.
5. Home always closes the surface and returns Home, regardless of the surface's local state.
6. A surface may not intercept Back permanently, hide the exit, or navigate another Luma screen by itself.

The existing shell priority remains: review/exit/pairing/Settings/details/player overlay/sports detail/music detail before Home (`src/App.svelte`, `back`). An extension surface is inserted at the point where its trigger opened it; it does not reorder the core layers.

### 8.4 Native-window caveat

If the selected candidate is a separate `WebviewWindow`, it is a different native focus tree. The spike must prove:

- The window is owned/positioned as a TV surface, not a second taskbar app; `parent`/`skip_taskbar`/decorations behavior is acceptable on Windows.
- Focus transfer to the surface and back is deterministic; the main window does not remain interactable underneath if the surface is meant to be modal.
- Back reaches the host even when the surface has focus. A surface that can swallow Back fails the gate.
- The phone remote router sends commands to the extension surface before `main`; Home closes it and restores the launcher.
- Hiding to tray destroys the surface; no hidden native window remains.

If these cannot be met, the separate-window candidate is rejected even if its isolation is good. The same rule applies to an iframe that cannot return focus predictably.

### 8.5 Crash and unresponsive behavior

- If the surface crashes or stops responding, the host reclaims focus after a bounded timeout (proposal: 2 s) and shows a host fallback with Retry/Close.
- One manual Retry is allowed. Automatic reload loops are not. If automatic recovery is attempted, bound it (proposal: at most one attempt per open) and record the result.
- The tray, Home, Settings, and recovery remain usable throughout.
- Back and Home must work while the fallback is shown.

## 9. Lifecycle and resource gates

### 9.1 Lifecycle states

```text
installed(disabled) -> enabled(granted) -> surface-requested -> creating -> ready -> visible
                                                                  |          |
                                                                  v          v
                                                                error      hidden
                                                                  |          |
                                                                  +-> destroyed <- disable/uninstall/exit
```

| Transition | Host obligations |
| --- | --- |
| Install | Validate manifest/files/identity; store package; keep disabled; no code execution. |
| Enable | Record version-scoped grants after explicit review; do not create a surface yet. |
| Surface request | Check grant, target, placement, and one-instance limits; create asynchronously. |
| Creating | Show host loading; enforce timeout; keep focus on the trigger; on failure show a recoverable error. |
| Ready | Verify entry focus; start only the declared subscription/timer; no hidden work. |
| Visible | Serve relay ops within rate/size caps; report errors; preserve host Back/Home. |
| Hidden | Stop timers/subscriptions; destroy the surface if the launcher is hidden; do not run code in the tray. |
| Disable/revoke | Close surface; stop relay; clear pending work; destroy webview; keep settings/data per policy; repair references. |
| Uninstall | Disable first; remove package files; remove or retain data per user choice; never leave a process or hidden window. |
| App update | Revalidate `apiVersion`; incompatible packages stay disabled with a reason; configuration is preserved. |
| Exit | Destroy every extension surface before process exit; do not rely on OS kill for cleanup. |
| Crash | Host fallback; bounded retry; no unbounded loop; core UI and tray survive. |

No extension surface is created at startup, for a disabled package, or for a package whose grant was revoked. No extension work survives hide-to-tray in Stage 6.

### 9.2 Resource gates

The plan's provisional envelope is a gate, not a shipped guarantee: no additional startup delay visible to the user, and idle CPU under 1% of one core averaged over a minute (`docs/CUSTOMIZATION-LAYERS.md`, §7.4). Memory must be agreed from a measured baseline before general installation. The interaction and surface-open budgets below are proposals, not accepted values.

| Gate | Proposed measurement | Pass condition |
| --- | --- | --- |
| Startup | Median startup-to-input over 5 runs with 0 vs 3 installed-but-disabled packages; full process tree | No visible regression; any delta is within the baseline's run-to-run spread and is recorded. |
| Idle, Home visible | 60 s at 1 s samples; weather widget visible, music screen closed; full process tree | Extension-related CPU delta under 1% of one core averaged over the minute; no unexpected network requests. |
| Hidden/tray | Same measurement after hide-to-tray | No extension webview process, timer, or request; CPU returns to baseline within measurement noise. |
| Surface open | Time from trigger to first interactive paint; 10 runs | Budget agreed before the test; loading state appears promptly and input is not blocked. |
| Weather data | Time to first data and to typed error/timeout | Bounded by the host timeout; Retry works; no indefinite spinner. |
| Interaction | Input-to-feedback for play/pause and focus moves | Budget agreed before the test; no dropped input while a relay call is in flight. |
| Mount/unmount | 10 open/close cycles; process count and private bytes | Process count returns to baseline; no monotonic growth; residual memory within the agreed budget. |
| Cache growth | Repeated weather refreshes and artwork changes | Cache is bounded; no unbounded image or response retention. |

Measurement rules:

- Measure the full process tree, including WebView2 child processes, not only `Luma.exe`.
- Record machine, build, commit, display resolution, widget/screen state, sample count, and method.
- Report raw numbers. Do not describe a delta as an improvement or equivalence without them.
- A budget not agreed before the run is not a pass criterion after the run.

## 10. Acceptance record

### 10.1 Static evidence recorded

These rows are facts read from source or official docs. They are not runtime proof.

| ID | Static check | Evidence | Status |
| --- | --- | --- | --- |
| S1 | Only declarative plugins are enableable; executable kinds install disabled and cannot be enabled. | `plugin.rs`, `execution_reason`, `prepare_enable`; test `unsupported_runtime_packages_install_disabled_and_cannot_enable`; `PLUGINS.md`. | Static only |
| S2 | The manifest has no permissions/network surface and rejects unknown fields. | `plugin.rs`, `Manifest`. | Static only |
| S3 | The only capability file targets `main`. | `src-tauri/capabilities/default.json`. | Static only |
| S4 | `build.rs` has no `AppManifest::commands`; custom commands are default-allowed by Tauri. | `src-tauri/build.rs`; Tauri capabilities docs. | Static only |
| S5 | CSP is strict and application-wide; no `frame-src`; no weather origin. | `tauri.conf.json`. | Static only |
| S6 | The `unstable` Cargo feature is not enabled. | `Cargo.toml`; `Cargo.lock`. | Static only |
| S7 | External webviews use navigation allowlists and host-injected bridges. | `youtube.rs`, `stremio.rs`, `sports.rs`. | Static only |
| S8 | Back/focus recovery has a host priority and Home fallback. | `App.svelte`, `back`; `navigation.ts`. | Static only |
| S9 | Playback is owned by the native player and frontend event subscription; ticking is visibility-gated. | Spotify Rust/frontend modules. | Static only |
| S10 | No weather widget/provider exists; widgets are static Svelte imports. | `widgets.ts`; `WidgetHost.svelte`. | Static only |
| S11 | The phone remote router has explicit surfaces but no extension surface. | `remote/server.rs`, `navigate`. | Static only |

### 10.2 Live checks not run

Every row is **Not run**. No pass/fail result exists.

| ID | Check | Method | Pass condition |
| --- | --- | --- | --- |
| L1 | Surface creation and first paint | Open the representative surface from the trigger | Surface appears without blocking the launcher; host loading state is replaced by ready content. |
| L2 | Main shell remains usable | With the surface open and after a forced crash, use Home/Settings/Back | Core Home, Settings, recovery, and tray remain reachable. |
| L3 | Native command denial | Invoke every application command category from the surface, including `window.__TAURI_INTERNALS__.invoke`, `customization_read`, `launch_app`, `spotify_connect`, and a window-creation command | All are denied; no side effect. |
| L4 | Webview/window creation denial | Try `core:webview` create/reparent and `window.open` | Denied; no new window/webview. |
| L5 | Navigation confinement | Navigate to 9877 (loopback and LAN), loopback 9878/9879, `file:`, `data:`, `blob:`, an unlisted host, and the surface's own origin | Only the own origin is allowed; every other navigation is cancelled. |
| L6 | New-window handling | `window.open` to an allowed and a disallowed URL | Denied or host-opened as designed; no untracked window. |
| L7 | Direct network denial | `fetch`, an image beacon, `sendBeacon`, and a WebSocket to the weather origin, loopback, a private address, and a redirect target | All are blocked; only host-mediated weather works. |
| L8 | Remote script denial | Add a remote `<script>`, a dynamic import, and `eval` in a test build | Blocked by CSP/validation; main CSP unchanged. |
| L9 | Secret/cookie denial | Read `document.cookie`, local/session storage, IndexedDB, and relay fields | No pairing cookie, token, path, or history is readable. |
| L10 | Identity spoofing | Modify surface JS to send another package ID, version, or permission | Relay rejects; host identity remains label-derived. |
| L11 | Ungranted operation | Call a valid op the package did not declare, and an unknown op | Denied with a typed error; no fallback command. |
| L12 | Weather states | Force loading, success, empty, stale, 429/5xx, timeout, malformed payload, and offline | Each state is distinct and recoverable; Retry works. |
| L13 | Weather lifecycle | Hide to tray, disable, uninstall, and re-enable while the widget is active | No request, timer, or process remains; re-enable is fresh and singular. |
| L14 | Playback display and control | Open the music screen; play/pause/next/previous/seek | Matches the core player; no duplicate state. |
| L15 | No duplicate auth/player | Search for a second Spotify session/player; attempt account/device ops | No second session; those ops are denied. |
| L16 | Focus entry/return | Enter the widget/screen by D-pad from Home and a hub; leave by Back/Home | Focus lands on the declared control and returns to the exact trigger or documented fallback. |
| L17 | Back layering | Open a local detail/menu, press Back, then Back again | Local layer closes first; surface closes second; focus restores. |
| L18 | Phone remote parity | Use arrow/activate/Back/Home from the phone while the surface is active | Host routes to the surface before `main`; Home closes it. |
| L19 | Crash isolation | Terminate the surface renderer process and separately hang its JS | Main survives; host fallback and focus recovery appear; tray works. |
| L20 | Disable cleanup | Disable while visible and while hidden | Surface, relay, timers, and process end; core UI remains. |
| L21 | Uninstall cleanup | Uninstall while referenced by Home/screen | Surface ends; references repair or show a placeholder; no orphan process. |
| L22 | Tray lifecycle | Hide to tray, reopen, exit | No extension surface survives hidden; reopen creates none until requested; exit cleans up. |
| L23 | Restart persistence | Restart Luma with the package enabled/disabled | Enabled state, grants, settings, and surface availability match the record; no code starts early. |
| L24 | Startup/resource measurement | §9.2 startup and idle measurements | Provisional envelope met on the target machine; raw numbers recorded. |
| L25 | Mount/unmount growth | 10 open/close cycles | No monotonic process/memory growth; residual within the agreed budget. |
| L26 | 1080p/4K readability | TV check at both resolutions | Focus visible, controls reachable, no obscured recovery, text readable at distance. |
| L27 | CSP header inspection | Capture the actual CSP on the surface and main | Surface policy is at least as strict; `main` `script-src` is unchanged; no `unsafe-eval`. |
| L28 | Full install journey | Import, review, enable, configure, disable, re-enable, remove | Every step has a visible state; failures preserve the previous package and configuration. |

A future decision record must fill the evidence column for L1–L28. Until then, the only defensible statement is: **the executable UI gate is unproven**.

## 11. Open decisions and blockers

| Item | Why it blocks | Recommended next step |
| --- | --- | --- |
| G2 / D6 runtime choice | No candidate has evidence for focus, Back, isolation, and resource cost. | Run the smallest candidate comparison; do not enable `unstable` or relax CSP just to keep a candidate alive. |
| Application command gating | Today custom commands are default-allowed; L3 cannot pass. | Add a build-time command manifest and a main-only permission set before the first extension webview exists. This is code work outside this document. |
| Per-surface CSP | The app CSP is global; custom-protocol response behavior is untested. | Capture the actual header on the candidate surface; reject if it requires weakening `main`. |
| Push mechanism | `core:event` is denied; Channel permission behavior is unverified. | Test Channel; otherwise use a host push with an origin-guarded bridge. Do not grant a general event bus. |
| Embedded vs separate window | Child webviews need `unstable`; a separate window changes focus/remote behavior. | Compare A and B only if B can be enabled deliberately and safely; otherwise prove A's focus/remote contract. |
| Label mapping | Tauri labels cannot contain plugin-ID dots. | Generate and persist a host-owned label mapping; never accept a label from the package. |
| Manifest/API version | The current strict v1 manifest cannot carry grants or network policy. | Define the v2 shape and migration; keep v1 declarative-only. |
| Memory/interaction budget | The plan gives only a provisional CPU envelope. | Measure baseline, agree budgets, then test; do not invent a shipped guarantee. |
| Credential broker | A weather provider may need a key; core tokens must never be exposed. | Decide whether the representative case is keyless; if not, design host-brokered credentials (D12). |
| Music scope | Browse/play-URI expands the action surface. | Start with now-playing plus bounded controls; add browse only if the journey requires it. |
| Package trust | Signing and publisher identity are G4. | Out of scope; do not imply that isolation replaces trust. |
| Background services | Stage 7 owns supervisors, triggers, and tray work. | Keep Stage 6 surfaces launcher-bound; do not smuggle background work into the UI runtime. |

## 12. What this record does not establish

- That an executable UI runtime is shipped, selected, or enableable.
- That a weather widget or an extension music screen exists.
- That native commands, network, cookies, or remote scripts are denied in practice.
- That focus entry/return, Back, or phone-remote routing work on a TV.
- That crash isolation, cleanup, or resource budgets pass.
- That CSP is compatible with a per-surface policy.
- That the main window, tray, Settings, or recovery survive an extension failure.
- That any measurement, build, test, or live check was run.

## 13. How this record can be closed

A future decision entry must include:

- **Runtime:** selected candidate, Tauri version and feature flags, origin/CSP mechanism, capability labels, relay command, and window model.
- **Evidence:** L1–L28 with the commands/methods used, raw measurements, machine, build, and display resolution.
- **Authority:** the exact granted permissions, denied permissions, network origins, navigation allowlist, and credential path.
- **Focus/Back:** entry, local-layer handling, return, phone-remote routing, crash recovery, and 1080p/4K checks.
- **Resources:** startup, visible idle, hidden/tray, surface-open, interaction, and mount/unmount numbers compared with budgets agreed before the run.
- **Residual risk:** WebView compromise, unsigned package trust, debug-only affordances, and any untested platform behavior.
- **Outcome:** accepted for a general loader, or rejected with the declarative constraint documented. Rejection is a valid result.

Until that entry exists, this record stays **pre-spike** and the declarative runtime remains the only supported contract.

## 14. Sources

Repository (read-only for this record):

- `docs/CUSTOMIZATION-LAYERS.md` — Stage 6 scope/exit criteria and §7.4 resource envelope.
- `docs/PLUGINS.md` — shipped declarative contract and the future-runtime boundary.
- `docs/CUSTOMIZATION-ROADMAP.md` — Phase 6 decision/spike framing.
- `src-tauri/src/customization/plugin.rs` — manifest, runtime kinds, execution gate, install/update, tests.
- `src-tauri/src/customization/mcp.rs` — capability discovery and tool boundary.
- `src-tauri/capabilities/default.json`, `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml`, `src-tauri/build.rs`.
- `src-tauri/src/lib.rs`, `background.rs`, `remote/server.rs`, `frame.rs`, `commands/youtube.rs`, `commands/stremio.rs`, `commands/sports.rs`.
- `src/App.svelte`, `src/lib/services/navigation.ts`, `src/lib/services/widgets.ts`, `src/lib/components/WidgetHost.svelte`, `src/lib/components/PluginSettings.svelte`, `src/lib/services/settingsNavigation.ts`.
- `src/lib/state/spotify.svelte.ts`, `src/lib/components/spotify/MusicHome.svelte`, `src-tauri/src/integrations/spotify/{mod,models,player}.rs`.

Official Tauri 2 documentation (primary):

- [Capabilities](https://v2.tauri.app/security/capabilities/)
- [Runtime authority](https://v2.tauri.app/security/runtime-authority/)
- [Content Security Policy](https://v2.tauri.app/security/csp/)
- [Isolation pattern](https://v2.tauri.app/concept/inter-process-communication/isolation/)
- [Configuration reference](https://v2.tauri.app/reference/config/) (capability, remote, CSP, background throttling)
- [WebviewWindowBuilder](https://docs.rs/tauri/latest/tauri/webview/struct.WebviewWindowBuilder.html) (`on_navigation`, `on_new_window`, initialization scripts, Windows creation caveat)
- [WebviewUrl](https://docs.rs/tauri/latest/tauri/enum.WebviewUrl.html)
- [Webview JavaScript API](https://v2.tauri.app/reference/javascript/api/namespacewebview/) (child webviews require the `unstable` Cargo feature)


## 15. Development harness checkpoint (2026-11-15)

`src-tauri/src/extension_spike.rs` implements candidate A as an opt-in boundary probe. Run a debug build with `$env:LUMA_EXTENSION_SPIKE="1"; npm run tauri dev`. It opens a separate incognito window on `http://luma-extension-spike.localhost`, serves three embedded assets under a restrictive response CSP, denies navigation off that origin and new windows, and supplies identity/ping/close actions. Release builds cannot open or serve the probe. Installed packages are never loaded or enabled by this harness.

Superseded by §20: the embedded assets and fixed identity were replaced by an installed-package surface. This paragraph records the earlier checkpoint.

This checkpoint uses a native invoke-handler guard rather than the proposed application command manifest. For the fixed spike label, the guard requires the debug opt-in and the expected current page origin, rejects every other application command explicitly, and deserializes only the three host-owned actions. The capability grants zero core/plugin permissions. Other existing windows retain their existing dispatch behavior. This is a spike-specific boundary, not a general executable-package grant system; AppManifest command registration remains the proposed production contract. Live IPC reachability and denial must still be proved.

The page tests command attempts and an uncaught JavaScript exception. That exception is not a renderer-process crash test. Closing returns native focus to main; destroying main dismisses the probe. DOM focus restoration, phone remote routing, hidden-window lifecycle, crash containment, and resource measurements remain unverified. The weather widget and playback integration are not implemented. All live acceptance rows above remain Not run; Stage 6 is not complete.

Parent verification: focused frontend plugin/customization tests passed (11). After parent review, `cargo test --manifest-path src-tauri/Cargo.toml --lib extension_spike` passed all four spike tests, and `node --check src-tauri/src/extension_spike/page.js` passed. These static/unit checks do not complete the live gates.


## 16. Read-only playback checkpoint (2026-11-15)

The debug harness now accepts a strict argument-free `playback` action and returns exactly `{connected, title, artist, playing}`. It projects the existing native player's remote music mirror rather than invoking Spotify or Windows media again. Title and artist are trimmed and capped at 200 characters each. No credentials, account names, URLs, artwork, device identifiers, or paths are returned. `connected` reports stored Spotify authentication presence, not provider reachability; the mirror can reflect the last published state rather than current provider state.

Parent source review traced this mirror through `Player` publishing and `remote::server::sync_music`. The focused native suite passed seven tests, including exact response fields, empty state, bounded Unicode text, and rejection of additional playback arguments. The page reads playback once on open and on explicit Refresh, with loading, empty, disconnected, and recoverable error states. Refresh failures retain the previous snapshot. Arrow keys move among enabled controls; Escape/BrowserBack request host closure. Parent review fixed refresh completion stealing focus after navigation. No live playback, denial, focus, phone, tray, or performance gate is completed by these tests.


Playback page regression coverage: `page.test.mjs` uses a dependency-free VM harness for successful/empty/disconnected snapshots, failed refresh with retained data, initial-error recovery, request serialization, focus restoration without stealing focus, enabled-button navigation, Escape, and BrowserBack. Parent reviewed and ran all 12 cases successfully. Simulated DOM behavior does not prove WebView2 blur, native focus, IPC, or window closure. Native phone/tray routing work remains in progress.


## 17. Host navigation and lifecycle checkpoint (2026-11-15)

The native phone navigation path gives a visible, non-minimized spike priority: Back closes it without touching main's Back stack; Home dismisses it before normal Home routing; arrows and Activate use fixed host-owned scripts with no interpolated input. Remote wake recognizes the spike. Hide-to-tray, remote stop, and main destruction dismiss it.

Parent review closed the reported surface-overlap gap: the debug probe now starts once after successful launcher presentation, rather than concurrently during setup. Later normal presentations dismiss it, covering tray Open, Exit review, and existing app presentation paths. This remains an environment-variable development entry, without an installed-plugin trigger or DOM focus-restoration target. A visible spike consumes phone navigation even when it lacks native focus. Native keyboard Home and Back while main has focus are still outside the spike's page path.

Tests cover the bounded relay and Back/Home routing decisions. Actual WebView key delivery, phone behavior, native focus/closure, competing presentations, tray cleanup, and resource measurements still require a live run. No Stage 6 live gate is marked passed by this checkpoint.

Parent verification after lifecycle integration: all 175 native library tests and all 133 `npm test` cases passed. Diff whitespace checks passed for the affected spike/routing files; unrelated generated `phone.html` whitespace remains outside this slice. No live acceptance or performance measurements were performed.


## 18. Bounded playback controls checkpoint (2026-11-15)

The development relay now accepts exactly three additional argument-free actions: `playback-toggle`, `playback-next`, and `playback-previous`. They map to the existing native Spotify player owner with no value/device/URI argument. Extra fields and adjacent actions such as seek, transfer, volume, connect, or arbitrary commands are rejected. The same opt-in, fixed-window, and current-origin checks apply before dispatch.

Controls execute asynchronously, with one in flight across the spike and an immediate busy rejection for overlap. Success means `{accepted:true}`, not that a refreshed snapshot has arrived. The existing player owns subsequent publication; the extension does not authenticate, create a player, select devices, or poll. Closing a surface does not undo a media command already dispatched.

Parent source review checked action parsing, fixed command mapping, and nonblocking resolver dispatch; all 10 focused native spike tests passed. UI integration and live Spotify/remote checks remain pending. Production package grants, activation, renderer isolation, and the other Stage 6 live gates are unchanged.

Playback UI integration: Previous, Play/Pause, and Next use those exact argument-free actions with pending/accepted/error feedback. Controls and refresh serialize without optimistic playback changes or polling; the user explicitly refreshes the cached snapshot. Parent review added a handler-level refresh guard during controls and a regression test. All 23 page tests, JavaScript syntax checking, and all 177 native library tests passed. Real player effects, WebView focus, and phone activation remain unverified.


## 19. Weather representative case integration (2026-11-15)

The development page has a fixed city/unit selector and an explicit Load/Refresh action. Opening the page or changing selection makes no weather request. Last results retain their original city/units after failed or slow requests. The guarded native relay accepts only strict `{action:"weather-read",location,units}` after the same debug opt-in, window-label, and current-origin checks; extra authority fields are rejected. The host client remains under implementation review. This does not add a core Home widget, settings persistence, package activation, or production grants.

Parent reviewed UI behavior, added a returned-unit consistency check, and ran all 37 page tests successfully. Native compilation was attempted but blocked by concurrent unrelated `viewing.rs` Windows imports and missing `session::press_window`; no passing native check is claimed for this checkpoint. Live provider-through-Luma, attribution-link handling, focus, phone select navigation, lifecycle cancellation, and performance evidence remain open.


Weather host review checkpoint: the fixed-origin client is wired to the guarded development relay with strict selection parsing, a 5-second request timeout, no redirects, a 64-KiB response cap, one in-flight request, bounded display projection, and sanitized errors. Parent replaced HTTP-client initialization panic with a recoverable unavailable error. The page keeps all direct provider access outside its CSP. An attribution URL is displayed but a host-opened clickable attribution link remains an integration gap. Requests already sent can continue for up to the timeout after surface close; cancellation remains a lifecycle gate. Retried native compilation is still blocked by two concurrent `viewing.rs` Windows import errors, so integrated native checks have not passed. These changes do not complete Stage 6 or enable executable packages.

Product clarification: weather is the example executable plugin for proving public package APIs and lifecycle, not a built-in feature. The current hardcoded development probe does not meet that milestone. Its eventual UI, settings, assets, and identity must come from an installed package using the same bounded public bridge available to third-party plugins. Do not add core weather widget/settings/schema fields to complete the example.


## 20. Package-driven development runtime checkpoint (2026-11-16)

The embedded fixed weather page, fixed package identity, and `weather-read` host branch are gone. The development harness now loads one installed `isolated-ui` package: its HTML entry, assets, declared settings values, name, id, version, permissions, and network allowlist all come from the installed package record at `<app-data>/plugins/<id>/<version>`. The weather surface is [examples/plugins/com.example.weather](../examples/plugins/com.example.weather); the host contains no weather provider endpoint, city coordinates, WMO mapping, or weather reading schema.

### 20.1 Public package contract

| Area | Implemented contract |
| --- | --- |
| Manifest | `apiVersion: 2`, `runtime.kind: "isolated-ui"`, package-relative HTML `runtime.entry`, exact `permissions`, optional `network.allow`, settings, assets. Version 1 remains declarative-only. |
| Permissions | `settings.read`, `network.get`, `playback.read`, `playback.control`. Unknown, duplicate, or unapproved-on-v1 names are rejected. |
| Network | `network.get` requires 1–8 HTTPS origins. Origins reject paths, ports, credentials, IP literals, loopback, and private-network-style names. The host constructs the URL from an allowed origin, relative path, and bounded query pairs. No URL, header, cookie, token, or redirect passthrough. |
| Bridge | `extension_bridge` accepts a strict `{ op, args }` envelope. Exact ops: `identity`, `settings.read`, `net.get`, `playback.read`, `playback.toggle`, `playback.next`, `playback.previous`, `surface.ready`, `surface.close`. Unknown ops, extra fields, and arguments on argument-free ops are rejected. |
| Identity | Returned from the installed record: id, name, version, API version, runtime, surface, and host origin. |
| Settings | Host-rendered and host-stored using the existing plugin settings table. The package reads resolved values only, including defaults and invalid-value fallback. |
| Assets | Served only from the installed package directory after path validation and canonical containment. Response CSP keeps scripts/styles/images/fonts self-only and IPC the only connect source. |
| Lifecycle | The package window opens in its loading state; the host focuses it and marks it active only after `surface.ready`. A 5-second watchdog closes an unready surface and restores launcher focus with a customization notice. Close, phone Back, remote stop, hide-to-tray, main destruction, package disable, remove, and update end the surface. |
| Authority | Package id and version never come from the page. Calling a bridge op the package did not declare returns `denied`. If the active package disappears, bridge calls return `revoked`. |

### 20.2 Development gate

The harness remains debug-only and requires `LUMA_EXTENSION_SPIKE=1`. It discovers exactly one installed `isolated-ui` package, or uses `LUMA_EXTENSION_PLUGIN=<id>` when several are installed. It opens the package after the launcher is presented. While the gate is active, the installed package card also shows **Open development surface**, providing a repeatable entry without restarting Luma. Production builds cannot open or serve the harness. Executable packages still install disabled, remain labeled unsupported, and cannot be enabled through Settings or MCP. The manifest's permissions are declarations, not a recorded user grant approval; there is no grant-review UI yet.

The example package import procedure and the bridge contract are documented in [PLUGINS.md](PLUGINS.md), "Future executable runtime".

### 20.3 Verification run

- `cargo test --manifest-path src-tauri/Cargo.toml --lib extension_spike`: 14 passed. Covers origin confinement, content types, exact bridge ops, permission mapping, argument rejection, network origin/path/query bounds, playback projection, and navigation keys.
- `cargo test --manifest-path src-tauri/Cargo.toml --lib customization::plugin`: 23 passed. Includes the weather example install, v2 grant validation, disabled activation, asset reads, traversal rejection, and existing plugin lifecycle tests.
- `cargo test --manifest-path src-tauri/Cargo.toml --lib customization::mcp`: 13 passed.
- `node --check examples/plugins/com.example.weather/ui/page.js` and `node --test examples/plugins/com.example.weather.test.mjs`: 12 passed. Covers package identity/settings/ready order, structured network request, empty/loading/ready/stale/error states, single in-flight request, focus movement, and close/Back.

These are unit and VM checks. No Luma process, TV, phone, tray, WebView2, CSP header capture, crash, or resource measurement was performed.

### 20.4 Remaining live and integration gates

- No live proof that the package origin receives the intended CSP, that Tauri IPC is reachable only through the bridge, or that native commands are denied.
- No live focus entry/return, phone remote parity, tray cleanup, package crash, WebView2 renderer termination, or unresponsive-surface recovery check.
- No production grant-review step: users see declared permissions in package details but do not approve them, and the Settings enable path remains blocked.
- The watchdog covers a package that never reports ready. It does not distinguish a slow machine from a crashed renderer and does not offer an in-surface Retry.
- Executable settings are part of the existing plugin snapshot, so saved-version history and MCP plugin listings include them. Per-setting privacy is not implemented; the example stores a city id and units, not coordinates or credentials.
- Direct network denial, redirect refusal at the provider, malformed/oversized response handling through the real bridge, and the actual Open-Meteo request from Luma's `reqwest` client are still unverified.
- Network allowlists reject private-looking host names and IP literals, but the host does not resolve the name and reject a public name that resolves to a private address. DNS-based SSRF remains a residual risk for a future production grant.
- The package attribution is a visible text credit with a fixed URL, not a host-opened link. CC BY link behavior remains an integration gap.
- Package signing, publisher identity, background execution, and a general multi-package loader remain outside this checkpoint.

## 21. Stage 7 implementation and live acceptance checkpoint (2026-11-16 local)

The reusable runtime, background supervisor, explicit permission review, tray actions, declarative remote panels, and uninstall retention choice are implemented. This supersedes the development-only activation descriptions in earlier checkpoints. Stage 7 implementation is complete; full release acceptance still has the interactive and resource gates below.

### Current runtime and lifecycle

- API v2 isolated UI and background packages require a persisted user grant. Import and MCP staging remain disabled-only. Native enable validates revision, permissions, network origins and trigger in one transaction. Disable revokes access; saved-version restore does not restore executable grants.
- Each active runtime uses one of 16 registered protocol slots (`luma-extension.s0` through `luma-extension.s15`) with its own host, a fresh session token/path, label and generation. Protocol and invoke guards check both slot and token. Slots can be reused after teardown; a fresh token URL is not a promise of a globally new browser origin on every restart.
- The initially proposed per-token subdomain was rejected by Tauri before the invoke guard in a real run. The fixed registered protocol slots are the tested implementation. Package WebViews run incognito; WebView cookies/localStorage/IndexedDB are not durable plugin storage. Durable package settings remain in SQLite.
- Launcher-open runtimes stop when the launcher hides. Startup/always background services survive launcher destruction and stop on disable, removal or exit. Explicit UI close tears down its runtime; reopening creates a fresh session.
- Startup readiness has a five-second timeout. WebView2 process-failure/unresponsive events feed bounded restart handling. Three restarts within ten minutes are allowed, then the package is disabled with a persisted reason and its grant cleared.
- The tray keeps Open/Exit and adds at most one host-owned action per enabled executable plugin: Open for UI packages, Disable for background services. The web remote renders declared playback panels itself; no extension code or pairing cookie is passed to a plugin. Display Box has no panel projection in its existing protocol.
- Removal keeps settings/profile data by default. Selecting delete-data removes settings and awaits runtime teardown before profile deletion. A locked profile produces an explicit cleanup error. Package export excludes user settings/profile data.

### Review fixes and regression evidence

Live bounded-failure testing found `disable_on_crash` holding the Store connection mutex across `store.snapshot()`, which tried to lock the same mutex. The transaction now drops its connection guard before snapshot/publish. A complete live failure cycle subsequently reached auto-disable and further Store reads, covering the reported deadlock. Related plugin lock/publish scopes were inspected.

Other reviewed fixes cover disabled-package loading through a retained grant, remote Back priority for inline consent/removal, post-close session cancellation, full DNS/request/body deadlines, public-address pinning without a proxy, failed-start visibility, renderer failure monitoring, and freeing protocol slots after failed windows are gone.

### Verification actually run by root

- `cargo test --manifest-path src-tauri/Cargo.toml --lib`: **221 passed**.
- `cargo check --manifest-path src-tauri/Cargo.toml --release --lib`: passed; the acceptance harness is excluded from release builds.
- `npm test`: **149 passed**.
- `npm run check`: **0 errors, 0 warnings**.
- `npm run build`: desktop and remote builds passed; generated `phone.html` rebuilt.
- Svelte MCP autofixer on modified desktop controls: no issues. Existing App/SettingsPanel suggestions outside the changed behavior were not refactored.
- `cargo build --manifest-path src-tauri/Cargo.toml --example plugin_runtime` and `src-tauri/target/debug/examples/plugin_runtime.exe`: **live acceptance passed**, exit code 0.
- The root rerun used a new temporary Store and `com.luma.acceptance.p16876`, never the user's database or credentials. Both generated data directories were absent after teardown. The harness cleans only paths owned by its run.

The live executable staged three real packages and approved grants through native code, without a main window. It verified background readiness and ongoing execution, UI bridge identity/settings/readiness, rejection of core and ungranted playback commands, CSP network denial with a security-policy violation and zero accepted TCP connections, old-session navigation denial, launcher-hide behavior, disable/revoke, retention on removal, three restarts followed by auto-disable, explicit data deletion, and terminal shutdown cleanup.

### Measured cost

Windows 11 Home 10.0.26300, debug build. Baseline and idle samples each lasted approximately 5.4 seconds; startup sampling lasted 1.1 seconds. CPU percentages are CPU-seconds divided by elapsed wall time (one logical core), not normalized across the machine. The background fixture has a one-second heartbeat to demonstrate ongoing execution.

| Phase | Host CPU | Host private | WebView2 CPU | WebView2 private | WebView2 processes |
| --- | ---: | ---: | ---: | ---: | ---: |
| No package runtime | 0.00% | 4.0 MiB | 0.00% | 0 MiB | 0 |
| Background startup | 1.38% | 6.4 MiB | 85.30% | 164.1 MiB | 6 |
| Background idle | 0.00% | 6.4 MiB | 2.02% | 158.5 MiB | 6 |

Background ready was **301 ms**; UI ready was **281 ms**, both below the five-second readiness deadline. These short samples establish cost, not long-duration stability or acceptable performance. No CPU/private-memory budget has been agreed, so no resource gate is claimed passed. A hidden WebView2 renderer is the available implementation primitive, not a separate native process sandbox or a lightweight worker guarantee.

### Remaining release acceptance

- Real TV/phone directional navigation, activation, Back/Home/Escape and focus return; 1080p/4K, fullscreen and multi-monitor behavior.
- Real tray menu interactions and the full launcher boot/destroy/reopen path, rather than direct harness lifecycle calls.
- Renderer-process termination/unresponsive events, native close-button flow, OS suspend/resume, and release-build runtime behavior. The live failure fixture exercises readiness failures; ProcessFailed is wired and has classification tests but was not induced live.
- Long-duration/repeated-cycle memory growth, all 16 concurrent slots, allowed host-mediated HTTPS traffic, real weather/provider behavior, and paired-phone endpoint behavior.
- Agree a startup/idle CPU and memory budget using the measured cost before marking the Stage 7 resource gate complete. Full Stage 6/7 acceptance remains open until these required checks are resolved.
