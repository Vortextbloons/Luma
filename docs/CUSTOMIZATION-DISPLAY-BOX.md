# Display Box and web remote surfaces for customization

> **Update 2026-11-24:** Display Box's TV Remote now uses frame-authenticated `/display/*` routes for Luma settings, app order, playback timeline, plugin panels, search and album art, plus native settings, search, panels, app order, offline retry and appearance following. See the Display Box working tree's `docs/project-status.md` for verification limits. The assessment below records the earlier state and the still-open surface/plugin architecture questions.

Status: planning document, no application code written. Assessed 2026-11-15 against the shared Luma working tree and the Display Box working tree. Both trees had uncommitted changes from other agents at assessment; facts below describe those working trees, not committed `HEAD`, and line references reflect the tree at assessment. Display Box was inspected read-only: no device commands, no builds, no writes there.

This document covers one connected question: how the customization layers reach the two non-TV surfaces — the native Android frame (Display Box) and the browser web remote — without turning either into a general-purpose extension host. Layer names follow the canonical [customization layers](CUSTOMIZATION-LAYERS.md) map; where this document needs to say what a layer means for a display, it does, and otherwise defers to that map rather than redefining it.

Read [the local server notes](CUSTOMIZATION-SERVER.md), [the MCP and recovery plan](CUSTOMIZATION-MCP-PLAN.md), and [the runtime record](CUSTOMIZATION-RUNTIME-SPIKE.md) for the implemented foundation, wire rules, and review rules.

## Status vocabulary used here

| Label | Meaning |
| --- | --- |
| **Shipped** | Present in the working trees and supported by the cited source. Not the same as user-accepted on hardware. |
| **Accepted intent** | The user has agreed to this direction. It is authorized scope, not implemented behavior. |
| **Proposal** | A design recommendation in this document. Not agreed, not built. |
| **Unresolved** | A decision that must be made before implementation, with the dependency stated. |

Anything not labelled **Shipped** is not a capability claim.

## Scope decision that is still open

**Unresolved (user decision):** whether Display Box support is Luma-mode-only.

- Recommendation: **yes, Luma-mode-only** for every new surface. The frame's standalone mode keeps its existing local Music, Games, and settings UI untouched; Luma surfaces appear only while Luma mode is enabled and paired.
- Why: standalone mode has its own Spotify/Soloist ownership, its own pages, and no PC connection. A surface document is host-authored; rendering it in standalone would either claim a disconnected PC is present or create a second, unreviewed configuration path on the frame.
- Consequence if the user later wants surfaces in standalone: caching a signed or explicitly user-saved surface, an offline action model, and a clear "this came from your PC" state are all required. None of that is proposed here.
- The Display Box `launcher/README.md` already describes standalone as an explicit action that retains pairing and sessions (see sources). This document does not change that contract.

**Accepted intent (user):** broad extension capabilities — appearance, composition, executable code extensions with background services, custom settings, apps, screens, desktop and web remote UI. This document accepts that breadth and then states which parts the frame can honestly render.

## Shipped facts

### Luma side

- One LAN server on `0.0.0.0:9877` serves the Display Box frame protocol, the web remote page, and phone routes. Routes and limits are defined in `src-tauri/src/frame.rs:774-800`; every route is under `DefaultBodyLimit::max(4096)`.
- Frame authentication is `Authorization: Bearer <frame-token>`; frame requests must not include `Origin` (`frame.rs:256`, `frame.rs:139-165`). Phone routes additionally require same-origin `POST`s.
- `GET /state` and `GET /phone/state` return `{ pcName, state }` where `state` is the full `AppState`: `view`, `fullscreen`, `sound`, `window`, `windowFullscreen`, optional `appearance` (selected tokens plus up to 8 theme palettes), `appOrder`, `settings` (8 entries with `key`, `group`, `section`, `kind`, `label`, `description`, `options`, `value`), and — in the current working tree — an optional `music` object (`src-tauri/src/remote/server.rs:63-147,192-206`).
- `POST /command` accepts a fixed command string; the allowlist is navigation commands, `stop`, `click`, `fullscreen`, `toggle`/`previous`/`next`, `musictoggle`/`musicprevious`/`musicnext`, volume, `youtube`, and app names from the compiled catalog (`frame.rs:304-334` as of the working tree). There is no command that carries a path, shell text, or arbitrary script.
- `GET /apps` returns the compiled app catalog in the shared dock order, with PNG icon paths; `GET /apps/{command}/icon` serves those PNGs (`frame.rs:461-552`).
- `GET /phone/ws` is an authenticated socket for the web remote. Incoming frames are capped at 4 KiB; appearance, app-order, and music changes are pushed unsolicited from `watch` channels (`frame.rs:644-726`).
- The working tree also contains uncommitted remote work: an optional compact `music` object on `AppState` (title, artist, position, capabilities, artwork flag; no artwork URL), music commands, and `GET /phone/music/art` serving current artwork bytes with `no-store`. A native test asserts the music object serializes under 512 bytes (`src-tauri/src/remote/server.rs`, `frame.rs`). This is in-progress work by another agent, not a committed feature; it is cited here only as evidence of the host-served, bounded-image pattern.
- Appearance is a fixed 20-token palette contract with validated `#rrggbb`/`#rrggbbaa` values, at most 24 tokens and 12 themes (`remote/server.rs:6-30`; `src/lib/services/theme.ts`).
- The web remote is a standalone Svelte/TypeScript/Tailwind app in `remote/`, built by Vite and inlined into the generated `src-tauri/src/phone.html` (`docs/DEVELOPMENT.md`, "Local remote server → Web remote"). The generated page was 171,203 bytes at inspection.
- The web remote renders `RemoteScreen` (Back/Home/more row, D-pad, transport deck), `AppsScreen`, `SettingsDialog`, and pairing/boot/offline states. Settings cards are derived from the `settings` payload's `group`/`section`/`kind`, so ordinary new settings appear without a page change (`remote/src/App.svelte`, `remote/src/components/SettingsDialog.svelte:112-149`).
- The web remote applies the TV palette through CSS custom properties, with a local "follow TV" override (`remote/src/lib/appearance.ts`).
- Customization persistence is version 3, native-owned SQLite, expected-revision commits, **one** previous configuration for restore, preview/Keep for Home layouts, and an opt-in localhost REST API on `127.0.0.1:9878` (`docs/CUSTOMIZATION-SERVER.md`; `src-tauri/src/customization/mod.rs:157`).
- There is no surface/capability endpoint, no extension manifest, no plugin loader, and no MCP adapter. Those are stated as future work in the roadmap.

### Display Box side

- Native Java and Android Views, no WebView in the launcher (`launcher/README.md:5`).
- `LumaClient` is a hand-written HTTP/1.1 client over raw sockets on port 9877: keep-alive for `/command`, `/text`, `/pointer`, one-shot for other GET/POSTs, 2500 ms connect, 3500 ms read, JSON responses capped at 65,536 bytes, icon responses capped at 512 KiB, and a 4,096-byte header-line reader. The working tree adds an `artwork(path)` variant with a 3 MiB cap for host-served album art (`launcher/src/com/isaac/framehome/LumaClient.java:59-170`).
- Pairing stores host, token, and PC name in Android Keystore-backed preferences; mode selection is a boolean preference; a failed command closes the connection and surfaces a message without replay (`LumaClient.java:27-62`).
- The native Luma UI is fixed composition: TV Remote deck with D-pad, OK, Back/Home, PC volume/mute, Spotify transport, fullscreen and Stop actions, plus an Apps sub-tab (`MainActivity.java`, `buildTvRemote`/`buildPcApps`). App tiles are host-driven; icons are fetched as PNG and downsampled to roughly 116 dp (`MainActivity.java`, `loadLumaApps`/`iconBitmap`).
- The working tree also renders a native Music card from the host's optional `music` object and fetches album art with `luma.artwork("/phone/music/art")` (`MainActivity.java`, music card builder; `LumaClient.java:65-68`). That is a host-driven, bounded-image precedent, not a general surface protocol. Note the auth split: the frame sends its bearer token, while `/phone/music/art` currently requires the phone cookie (`frame.rs`, `phone_music_art`). Display routes must not reuse phone authentication; see capability negotiation.
- `/state` is polled about every 3 seconds while the activity is running and in Luma mode, with an extra read before playback commands (`MainActivity.java`, `pollLuma`/`lumaPlayback`). The frame consumes `window`/`windowFullscreen` and, in the working tree, `music`; it discards `appearance` and `settings`.
- The frame's palette is its own light/dark selection plus the Apple-style skin; it does not consume Luma's token names (`MainActivity.java`, `onCreate` preference load).
- Luma mode disables Soloist startup and disconnects local playback without deleting the Spotify or Soloist session; standalone is an explicit user action that retains pairing (`docs/project-status.md:14`, `launcher/README.md:17`, `MainActivity.java`, `lumaSetup`/`activateLuma`).
- The manifest declares `FOREGROUND_SERVICE`, `RECEIVE_BOOT_COMPLETED`, and two `mediaPlayback` services (Soloist, experimental receiver). `usesCleartextTraffic` is `false`; today's LAN traffic works because `LumaClient` uses raw sockets rather than Android's HTTP stack (`launcher/AndroidManifest.xml:1-32`).
- The Display Box tree had uncommitted edits to `LumaClient.java`, `MainActivity.java`, `Glyph.java`, `SoloistService.java`, `build.ps1`, `README.md`, `project-status.md`, and new drawables at inspection. Facts above describe that working tree, not committed `HEAD`.

## What the three layers mean on a display

| Layer | Desktop TV | Web remote | Display Box frame |
| --- | --- | --- | --- |
| Appearance | Semantic tokens, density, text size, motion | Applies the same 20 tokens as CSS variables; local follow/override | **Proposal:** a small validated role set (scheme, accent, 3-4 surface roles), mapped to native colors; no arbitrary CSS, fonts, or token names |
| Composition | Home rows, columns, spans, screens | Renders richer layout and editors; full surface fidelity | **Proposal:** a bounded panel of nodes on a remote surface; not a mirror of Home, no nesting beyond a fixed depth, no pixel positioning |
| Executable extensions | Isolated host runtime, not yet chosen | May present extension settings and previews; no extension script execution under current CSP | **Proposal:** never executes extension code; receives only a host-authored surface description and sends action IDs |

The frame is not a second Luma renderer. It renders a deliberately smaller, host-authored description of what a person can do from across the room. The web remote is where a richer surface can live, because it is already a browser UI with the app's real design tokens. This follows the accepted product principle that a surface may omit a capability it cannot render safely, but must not invent a parallel write path.

## Native bounded renderer (proposal)

### Shape

A display surface is JSON produced by Luma, fetched by the frame, and rendered by a small set of native view factories. Draft name `luma.surface/1`; this is a design draft, not a wire contract.

```json
{
  "surface": 1,
  "id": "media.panel",
  "revision": 42,
  "title": "Now playing",
  "stale": false,
  "root": {
    "kind": "column",
    "children": [
      { "kind": "text", "role": "title", "text": "Track name", "maxLines": 1 },
      { "kind": "progress", "value": 0.42, "label": "1:12 / 2:51" },
      { "kind": "row", "children": [
        { "kind": "button", "label": "Previous", "action": "media.previous" },
        { "kind": "button", "label": "Play", "action": "media.toggle", "emphasis": true },
        { "kind": "button", "label": "Next", "action": "media.next" }
      ]},
      { "kind": "notice", "tone": "info", "text": "Playing on LUMA-PC" }
    ]
  }
}
```

Node kinds (draft allowlist): `column`, `row`, `grid`, `text`, `button`, `toggle`, `choice`, `progress`, `image`, `spacer`, `notice`. Images are references to host-served icon/image IDs with declared sizes; the frame never loads a URL from the document. There is no `html`, `style`, `webview`, `script`, or free-form text markup.

### Bounds to enforce in the renderer, not only the host

- Nesting depth and total node count are checked before inflation; excess is rejected as a whole document, not partially drawn.
- Text is plain, length-capped, single-style per node; roles (`title`, `body`, `caption`, `label`) map to existing native sizes so TV viewing distance stays consistent with the current UI.
- Lists are host-paged: the frame asks for the next page, it does not receive an unbounded collection. The frame is a 1 GB RK3326 device; a long `ScrollView` of inflated rows is the wrong primitive.
- Grids use fixed column counts at the frame's landscape size (1280x800 at 160 dpi); the host does not get to choose pixel positions.
- Every surface must contain or inherit a host-owned exit affordance and remain reachable by Back/Home. A surface cannot hide Settings, cannot trap focus, and cannot claim fullscreen from the document alone.
- Unknown node kinds render as an unavailable placeholder or are skipped depending on a declared `optional` flag; a document whose required nodes are unsupported falls back to the legacy remote screen.
- Actions are IDs from the negotiated catalog. The renderer sends `{ action, params }` to Luma and applies the response; it never invents behavior or executes a command string from the document.
- Bitmaps decode through the existing bounded path (sample size chosen for the target dp size); no new unbounded cache.

### Why native and bounded

The frame has no WebView, and adding one would import a browser engine into a 1 GB device with a launcher that must stay responsive. A native description renderer keeps touch/focus behavior, the existing palette logic, and existing resource behavior under the launcher's control. The cost is that the frame's surface vocabulary will always be smaller than the desktop UI; that is the intended trade, not a temporary limitation to be removed later.

## Capability negotiation (proposal)

### Exchange

1. Frame → `GET /display/capabilities` (frame bearer token, no `Origin`). Response: protocol version, supported surface document versions, node kinds, action namespaces, palette roles, byte and count limits, feature flags (images, lists, unsolicited push), and whether the display is currently able to render at all.
2. Frame caches the descriptor until pairing changes, the host reports a higher protocol, a token failure occurs, or the app restarts.
3. Frame → `GET /display/surface?id=<surface>&rev=<n>` with the negotiated surface version. Luma answers with the highest mutually supported version and the document; an unsupported request returns a typed "use legacy" result rather than a partial document.
4. Frame reports render failures and unsupported kinds back in the next request (a small header or query field) so the host can downgrade that surface instead of the whole display.

### Draft descriptor

```json
{
  "display": 3,
  "surface": { "min": 1, "max": 1, "maxBytes": 16384 },
  "kinds": ["column", "row", "grid", "text", "button", "choice", "progress", "image", "notice"],
  "actions": ["nav.*", "media.*", "setting.set", "app.launch", "surface.refresh"],
  "palette": ["scheme", "accent", "surface", "surfaceAlt", "text"],
  "features": { "images": ["icon"], "lists": "paged", "push": false }
}
```

Rules:

- **Feature detection, not version sniffing.** A frame that lacks a kind gets a surface without it, or the legacy screen, not a broken layout.
- **Authentication realm does not change.** Display routes authenticate with the display bearer token, exactly like `/state` and `/command`; they must not require the web remote's cookie or its same-origin rules. The working tree's `/phone/music/art` uses phone authentication while the frame calls it with its bearer token; surface endpoints must not repeat that split.
- **Additive minor versions only.** Unknown fields are ignored; unknown kinds are handled by the `optional` rule.
- **Old frame, new Luma:** no `/display/*` call succeeds (404), so the frame keeps today's fixed TV Remote; Luma must not assume any frame is new.
- **New frame, old Luma:** the frame keeps the legacy screen and says nothing about missing features.
- **Removed capability:** a surface is re-fetched after any negotiation change; a stale cached render of a revoked surface must not stay interactive.

**Unresolved:** whether negotiation is a separate GET or a header pair on the existing `/state` call. The frame's HTTP client writes headers manually and has a 3,500 ms read timeout, so an extra round trip has a real cost on reconnect. Recommendation: one GET at connect, then piggyback the negotiated version and surface revision on `/state` so steady-state polling does not grow.

## Extension UI descriptions and PC-side logic (proposal)

### Division of work

- **Luma (PC) owns behavior.** Extension handlers, background services, settings storage, app launching, credentials, and any native work run in the Luma process under host authority.
- **The frame owns presentation.** It receives a surface description with static strings and bound values already resolved by the host, and it returns action IDs with small parameters.
- **The web remote may render more.** It can present the full surface, settings cards, previews, and history because it is a browser UI with the app's token contract.

### Extension manifest additions (draft)

- `surfaces`: one or more descriptions, each with an ID, title, the action IDs it uses, and the node subset it requires. A surface that needs a richer environment than a display declares that requirement instead of degrading silently.
- `actions`: the PC handlers behind action IDs, with argument schema, whether they are read-only, and whether they need a grant. The action catalog is validated at install; a surface cannot reference an action that was not declared.
- `settings`: entries expressed in the existing `SettingChoice` shape (`group`, `section`, `kind`, `label`, `description`, `options`, `value`) so the web remote renders them with no page change, and the frame renders only `choice` and `toggle` in its own bounded form.
- `apps`: additional launchable items only through the existing compiled/validated app catalog rules; extensions do not get to send executable paths or arguments to the frame.
- `screens`: composition contributions for the desktop and the web remote. Not a frame feature unless a specific screen has a display-safe surface.
- `services`: background work, host-side, with an explicit lifecycle: start on demand or at Luma startup, stop on disable/uninstall, and no dependency on a mounted launcher webview.

### Hard rules

- The frame never receives extension code, credentials, executable paths, or account identifiers.
- Action results are typed (`accepted`, `rejected`, `unavailable`, `pending`) and produce visible feedback; no silent failures, no automatic replay.
- Permission expansion on update requires a new grant; disable/uninstall stops services and removes surfaces. Existing configuration is retained unless the user deletes it (matching roadmap Phase 7).
- A broken or incompatible extension must not prevent the frame from rendering its legacy remote screen or the desktop from starting.

### Background services and the tray

**Accepted intent:** extension background services are part of the broad capability goal, and the tray is supported where practical.

- Luma already serves the LAN API and customization store while the launcher window is destroyed and while running in the tray (`docs/DEVELOPMENT.md`, "System tray and Windows startup"; `src-tauri/src/background.rs`). That is the correct home for host-side service work.
- A surface must therefore render correctly when the launcher webview does not exist: static host state is available, live view/selection state may be `null`, and the surface says "unavailable while Luma is in the tray" rather than showing fabricated values.
- **Unresolved:** whether any extension work may run on the frame itself. Android 11 on this device restricts background execution; a frame-side service would need a declared foreground type, a user-visible notification rationale, and a memory budget agreed after measurement. Recommendation: host-only services for the first implementation; frame-side services are a separate, later decision.
- **Unresolved:** what the frame shows for a service that is running but whose data is stale. Recommendation: an explicit timestamp and a stale marker, not a spinner that never resolves.

## Web remote richer surfaces (proposal)

The web remote can carry the customization product surface that the frame cannot:

- Full-fidelity surface rendering from the same description document, with responsive portrait/landscape layouts matching the existing `RemoteScreen` breakpoints.
- Extension settings cards through the existing `settings` contract, plus a clear "manage on the PC" note for desktop-only settings, following the current pattern in `docs/DEVELOPMENT.md` ("Local remote server → Web remote").
- Layout and composition editors, including the existing Home column/span controls and preview.
- MCP and recovery surfaces: the generic copyable setup prompt, a pending-review banner, the ten-version list with restore, and appearance reset — with commit/Keep authority limited exactly as the [MCP plan](CUSTOMIZATION-MCP-PLAN.md) defines. See the next section.
- Version/diagnostic panels for frame pairing and capability state, so an unsupported frame is explainable without a PC log.

Limits that shape this:

- Every route body is capped at 4 KiB and incoming WebSocket frames at 4 KiB (`frame.rs:667,798`). A richer surface arrives by `GET`, not by pushing large documents through the socket.
- The page is a single inlined HTML file with a CSP of `default-src 'none'` and inline script/style only (`frame.rs:33,358-368`). Extension JavaScript cannot run in the remote without a deliberate, reviewed CSP change. **Recommendation:** do not execute extension code in the remote; render host-provided descriptions and host-generated previews.
- Same-origin POST rules and the `HttpOnly`, `SameSite=Strict` cookie stay unchanged (`docs/DEVELOPMENT.md`, "Local remote server"). New endpoints inherit them; no new credential type is introduced for displays.
- If a preview image is needed, Luma serves a bounded PNG generated from its own renderer, not a remote-supplied URL. The working-tree `/phone/music/art` endpoint is the existing precedent: host bytes, `no-store`, no client-chosen source.

## MCP settings, history, and reset (accepted intent, not implemented)

The service boundary, review flow, and recovery design live in [the MCP plan](CUSTOMIZATION-MCP-PLAN.md); this section records only what the display surfaces must respect.

Accepted intent from the user:

1. MCP Settings offers a **generic copyable setup prompt** with no IDE-specific instructions.
2. **User Keep is required** for agent-proposed changes.
3. The **last 10 saved versions** are retained.
4. **Reset appearance** preserves accounts and apps.
5. Tray operation is supported where practical.

Display-facing rules:

- **Keep is not a remote capability.** Per the MCP plan, Keep is a desktop-UI action backed by a Tauri command; no HTTP route or MCP tool reaches it. The web remote may show that a review is pending and may Keep appearance/composition candidates only if that open recommendation is accepted; the frame never Keeps anything and shows at most "review waiting on your PC". Do not build a surface action that commits customization.
- **History is host-owned.** Today's store keeps exactly **one** previous configuration (`src-tauri/src/customization/mod.rs:157`; `docs/CUSTOMIZATION-SERVER.md:45`). The ten-version design in the MCP plan adds a version row in the same transaction as each commit/restore/reset, prunes beyond the newest ten, and never rewinds revisions. The display implication is narrow: a restore or reset is a new revision, so surfaces and settings must refresh by revision, and the web remote's version list must not become a second writer.
- **Reset appearance is visual appearance.** Use the boundary in the MCP plan: theme and tokens, density, tile shape, text size, motion, and per-screen appearance overrides return to defaults; Home layout and widget visibility/placement also reset to defaults. Apps, app order, extension installs and settings, accounts, pairing, and app paths are untouched. The MCP plan leaves the interface sound set and level untouched by recommendation; this document does not widen that boundary. A frame surface showing extension-contributed appearance must fall back to the default palette after a reset, not keep a stale palette.
- **Setup prompt belongs on the PC side.** It goes in desktop Settings and/or the web remote, never on the frame: the prompt is long, must be selectable, and carries no credential.
- **Tray keeps the service reachable, not the review UI.** Status, pending-review indication, and revocation remain available with the launcher destroyed; review itself is a desktop UI, consistent with the MCP plan.

## Compatibility and lifecycle

| Situation | Expected behavior |
| --- | --- |
| Old frame app, new Luma | Frame never calls `/display/*`, so it keeps today's fixed remote UI; Luma must not assume a display understands surfaces. |
| New frame app, old Luma | Frame detects no `/display/*`, keeps the legacy screen, no user-facing error about unsupported customization. |
| Newer surface document than the frame understands | Host downgrades to the frame's maximum; if it cannot, it sends the legacy marker. |
| Frame reports a broken or unsupported kind | Host remembers it for that display and serves a reduced surface next time. |
| Surface revision changes while displayed | Frame re-fetches before the next action; an action carrying a stale revision is rejected with a visible "this changed on your PC" message, not applied. |
| Extension disabled or uninstalled | Its surfaces disappear, its actions reject as unavailable, open surfaces close with a notice, host services stop. |
| Pairing forgotten or replaced | 401/403 clears surface state and returns to pairing guidance; no cached surface remains interactive. |
| Luma updated but customization document is incompatible | Existing rule stands: refuse and preserve data rather than overwrite with defaults. The frame shows the legacy screen and a short explanation, never a blank surface. |

## Disconnected behavior

Facts: the frame stays in Luma mode on failure, reports the problem, and never silently switches to local playback; standalone requires an explicit action (`docs/project-status.md:14`; `MainActivity.java`, `lumaSetup`). Commands close the socket on error and are not replayed (`LumaClient.java:75-104`).

Proposed surface rules:

- Keep the last rendered surface visible with a stale marker and a plain retry affordance. Do not blank the screen on a transient failure; the frame is across the room and a blank panel looks broken.
- Disable actions that need the PC while disconnected. Navigation to another display surface may still work if it is already cached; anything that reads live PC state must show unavailable.
- No queued actions, no queued setting changes, no automatic retries beyond the user's next input. A failed update is not resent later against a changed revision.
- On reconnect, refresh capabilities and surface by revision, then enable actions. Resume polling at the existing cadence; do not accelerate polling while disconnected.
- On 401/403, stop rendering surfaces entirely and direct the user to pair again; a stale surface with revoked credentials must not look active.
- If Luma is in the tray with no launcher window, surfaces that depend on live view state show "unavailable" rather than defaults; static host data (PC name, sound, app order) remains valid.
- The frame must remain usable as a plain remote for the existing command set, even if surface rendering fails.

## Standalone isolation

- Luma mode and standalone mode continue to be separate user choices. Pairing, Spotify sessions, and Soloist sessions survive switching, as documented today.
- A surface is host-authored and has no meaning offline. Nothing in this design writes surface state into the frame's standalone pages, Spotify pages, or game state.
- The frame's Spotify credentials, PKCE tokens, and Soloist key remain frame-local. Surfaces never carry them; actions that need Spotify on the frame stay in the frame's existing Music UI.
- While Luma mode is enabled, Soloist startup stays suppressed without deleting its session (existing behavior). A Luma surface must not start Soloist playback or change the Spotify Connect device.
- No new listener, service, or exported component on the frame is required for surfaces. If a later design adds one, it must be `exported="false"`, require the existing pairing token path, and be unreachable from standalone.
- If the user switches to standalone while a surface is displayed, the surface is discarded with the mode change; there is no offline surface cache.

## Payload and resource measurements

No runtime measurement was performed for this document: Display Box was inspected read-only and Luma was not launched. The numbers below are either static facts from source or arithmetic bounds from schema constraints. They are not benchmarks and must not be cited as performance evidence.

| Item | Current fact or bound | Source |
| --- | --- | --- |
| Frame JSON response cap | 65,536 bytes; header line 4,096 bytes; 2,500 ms connect, 3,500 ms read | `LumaClient.java:91-104` |
| Frame icon response cap | 512 KiB per image | `LumaClient.java:59-62` |
| Frame artwork response cap | 3 MiB per image (working tree) | `LumaClient.java:64-68` |
| HTTP request body cap, all routes | 4 KiB | `frame.rs:798` |
| WebSocket incoming frame cap | 4 KiB | `frame.rs:667` |
| Web remote page payload | 171,203 bytes generated single-file HTML at inspection | `src-tauri/src/phone.html` |
| Appearance payload | 20 tokens selected, ≤ 24 tokens and ≤ 8 theme palettes per appearance object | `remote/server.rs:6-30` |
| `/state` size | Schema-bound: 9 palettes × 20 tokens plus 8 settings entries plus an optional music object (< 512 bytes per the working-tree test) ≈ 5–7 KiB per response (arithmetic estimate, not captured). Polled about every 3 s while Luma mode is active → roughly 1,200 responses/hour. | `remote/server.rs:98-147,192-227`; `MainActivity.java`, `pollLuma` |
| Host-served artwork | `GET /phone/music/art` returns current artwork bytes with `no-store`; the frame requests this fixed path and never a client-chosen URL | `frame.rs` working tree |
| Existing steady-state frame network behavior | One `/state` GET per ~3 s plus one extra before playback commands | `MainActivity.java`, `pollLuma`/`lumaPlayback` |

**Why this matters for surfaces:** the frame's 64 KiB JSON cap is the hard ceiling for any surface document, and `/state` already consumes part of each poll. A surface should stay well under that cap so a lowering of the cap or an added field does not silently break rendering. Recommended starting targets, to be confirmed by measurement rather than assumed:

- Surface document: ≤ 16 KiB typical, 64 KiB absolute hard failure at the frame client.
- Surface fetch: only on change, never on the 3 s poll cadence.
- Node count and text length bounded so inflation stays well under one frame's budget on the RK3326.
- Images: existing icon path only, bounded decode, no per-surface cache growth.

### Measurement plan (not yet run)

1. **Surface payload**: log the byte size of each surface document Luma serves, per display and per extension. Record the maximum and p95 after a representative session. Do not publish numbers from a single document.
2. **Steady-state bytes**: capture `/state` request/response sizes over several minutes in normal use, with and without surfaces, using Luma-side counters rather than host packet captures where possible. Compare to today's baseline on the same machine.
3. **Frame render cost**: on the emulator first, then the frame, measure first-surface time after resume, jank (`gfxinfo`) for a surface with the maximum supported node count, and RSS/PSS before/after surface use. Compare RSS with RSS and PSS with PSS in the same window, per Display Box guidance.
4. **Host background service cost**: Luma CPU/RSS idle with an extension service enabled versus disabled, including while in tray with the launcher destroyed. Distinguish idle average from bursts.
5. **Web remote cost**: served page bytes, number of requests to first interactive, WebSocket message sizes during a command burst, and settings-dialog render cost on a mid-range phone browser.
6. **Failure paths**: reconnect time after the PC sleeps, behavior when the token is rotated, and recovery after an extension crash.
7. Agree budgets from those measurements before any "responsive" or "low overhead" claim. Do not run long soaks or broad benchmarks unless the user asks.

## Acceptance gates for this layer

Not satisfied today; listed so they are not mistaken for completed work.

- [ ] User confirms Luma-mode-only scope (or specifies standalone behavior).
- [ ] Native surface renderer implemented with enforced bounds, D-pad traversal, Back/Home ownership, and legacy fallback.
- [ ] `/display/capabilities` and surface fetch implemented, authenticated like the existing frame routes, with typed downgrade results.
- [ ] Surface revision conflicts produce visible feedback and never apply a stale action.
- [ ] Extension manifest declares surfaces, actions, settings, apps, and services; host rejects undeclared action IDs.
- [ ] Custom settings from extensions appear in desktop Settings and the web remote without a page change; frame shows only its supported subset.
- [ ] Last 10 saved versions implemented with bounded storage; appearance reset verified to preserve accounts, app paths, composition, and extension installs.
- [ ] MCP settings surface renders the generic setup prompt and pending-review state; no display surface can commit or Keep customization.
- [ ] Disconnected, tray-only, revoked-pairing, and extension-disabled states each verified on a real frame or the approved emulator.
- [ ] Standalone mode verified unchanged: no surface leakage, no Soloist start, sessions retained.
- [ ] Payload and resource measurements recorded against an agreed budget before any performance claim.
- [ ] Existing open gate stands: live desktop/TV/phone acceptance from the roadmap remains unverified.

## Sources

Luma (working tree at assessment):

- [`docs/CUSTOMIZATION-SERVER.md`](CUSTOMIZATION-SERVER.md), [`docs/CUSTOMIZATION-MCP-PLAN.md`](CUSTOMIZATION-MCP-PLAN.md), [`docs/CUSTOMIZATION-DISTRIBUTION.md`](CUSTOMIZATION-DISTRIBUTION.md), [`docs/DEVELOPMENT.md`](DEVELOPMENT.md)
- `src-tauri/src/frame.rs`: routes and limits, state, commands, apps/icons, phone socket, pairing and origin checks.
- `src-tauri/src/remote/server.rs`: `AppState`, appearance and settings contracts, snapshots.
- `src-tauri/src/customization/mod.rs`, `document.rs`: revisioned store, single previous revision, validation rules.
- `src-tauri/src/background.rs`: tray, window unload, background startup.
- Frontend: `src/lib/services/theme.ts`, `src/lib/types/index.ts` for token and view contracts.
- `remote/src`: `App.svelte`, `RemoteScreen.svelte`, `SettingsDialog.svelte`, `lib/appearance.ts`, `lib/socket.ts`, `lib/types.ts`.
- `src-tauri/src/phone.html`: generated single-file web remote.
- `scripts/inline-remote.mjs`, `package.json`: remote build and inlining.

Display Box (read-only inspection, no device commands or writes):

- `AGENTS.md`, `docs/project-status.md`, `launcher/README.md`, `launcher/AndroidManifest.xml`
- `launcher/src/com/isaac/framehome/LumaClient.java`, `MainActivity.java` (Luma mode, TV Remote composition, polling, playback), `Glyph.java` (native icon vocabulary)

Not verified in the available environment: any live frame or emulator interaction; the actual `/state` byte size; surface rendering; extension behavior; MCP adapter; ten-version history; web remote rendering changes. Live TV/phone acceptance from the roadmap also remains open.

## Known gaps in this document

- [The layer plan](CUSTOMIZATION-LAYERS.md) is the canonical cross-layer map. Where this document and the MCP plan overlap (Keep, history, reset), the MCP plan owns the service detail; this document owns only the display implications.
- No Svelte skill was available in this environment, so the Svelte analysis is limited to reading the remote's components, props, and state modules directly.
- The surface schema, capability descriptor, and manifest fields are drafts with no implementation or validation yet; they exist to make the open decisions concrete.
- The frame-side renderer's exact node subset depends on measurement and on whether the user accepts Luma-mode-only scope.
