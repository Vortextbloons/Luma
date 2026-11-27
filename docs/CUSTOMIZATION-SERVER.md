# Local customization server foundation

Implemented: native Home/appearance ownership, registered Home sections, one/two-column layouts, recoverable preview/Keep, ten saved versions, the opt-in MCP endpoint, and declarative appearance/Home plugins with settings and portable ZIP import/export. Executable plugin runtimes, nested containers, and AI chat remain future work. See [plugins](PLUGINS.md) for the current contract.

## Enable locally

The server is disabled by default. Launch a fresh Luma process with `LUMA_CUSTOMIZATION_TOKEN` set to a random value of at least 32 printable, non-whitespace ASCII characters. It binds only `127.0.0.1:9878`, separately from the phone/frame LAN server. Quit an existing Luma instance first: a second process cannot reconfigure the running instance's environment.

Generate a token without printing it and start the desktop app in PowerShell:

```powershell
$tokenBytes = New-Object byte[] 32
$tokenGenerator = [System.Security.Cryptography.RandomNumberGenerator]::Create()
$tokenGenerator.GetBytes($tokenBytes)
$tokenGenerator.Dispose()
$env:LUMA_CUSTOMIZATION_TOKEN = [Convert]::ToBase64String($tokenBytes)
npm run tauri dev
```

Use that token in your local API client. Do not put it in URLs or repository files. Stopping Luma stops the server; unset the variable before the next launch to disable it. Startup configuration/binding errors are reported on stderr while normal application startup continues.

Open the updated launcher once before writing. It imports its normalized existing preferences only when native storage is empty. Background-only startup does not commit defaults or mark migration complete. Later launcher creations load the native document instead of older localStorage values.

## API

Every route requires `Authorization: Bearer <token>` and exact host `127.0.0.1:9878`. Requests containing Origin are rejected. There is no CORS/browser UI. Bodies are limited to 32 KiB. Token holders have full supported configuration write authority, including direct commit and Keep; the token is not a review-only grant.

| Method | Route | Body | Result |
| --- | --- | --- | --- |
| GET | `/v1/config` | None | Current document, or `null` before migration |
| POST | `/v1/validate` | Configuration object | `{ "valid": true }` or validation error; no write |
| PUT | `/v1/config` | `{ "expectedRevision": number, "config": configuration }` | Committed document |
| POST | `/v1/restore` | `{ "expectedRevision": number }` | Previous configuration committed as a new revision |
| GET | `/v1/preview` | None | Active candidate or `null` |
| POST | `/v1/preview` | `{ "expectedRevision": number, "config": configuration }` | `{ "id": string, "baseRevision": number, "config": configuration }` |
| POST | `/v1/preview/keep` | `{ "previewId": string }` | Committed document |
| POST | `/v1/preview/cancel` | `{ "previewId": string }` | `{ "cancelled": true }` |

A document has `version: 3`, positive `revision`, and `config`. The REST document is the effective rendered configuration, including enabled declarative plugin contributions; plugin installation and export are user actions in Settings, not REST operations. Config contains `theme`, `density`, `appTileShape`, `textSize`, `reducedMotion`, `interfaceSounds`, `soundLevel`, `home`, `homeColumns`, `widgets`, and `order`. Older v3 documents that predate the reading and sound fields load with `standard`, `false`, `real`, and `standard` respectively. Writes replace the complete configuration. Unknown fields/values are rejected. All seven Home kinds must occur exactly once; all twelve apps occur once with Custom last. Hidden rows retain their position. `homeColumns` and each row's required `span` accept only 1 or 2. In two columns, span 1 uses half the grid and span 2 uses both columns. The leading hero stays full width. Below 1000 px, sections stack in order. If a write changes a field controlled by an enabled plugin, that plugin's control of the field is detached and the user's value is stored.

Each Home row is `{ "id": "personal-dock", "component": "apps", "visible": true, "span": 1 }`. IDs identify instances, must be unique, and contain 1?64 ASCII letters, digits, underscores, or hyphens. Components are `hero`, `continue`, `apps`, `popular`, `featured`, `recent`, and `recentApps`. Renaming an instance preserves its kind and label. Ordering, hiding, and sizing these singleton sections are supported; extra components and nesting are not.

Widgets are a map keyed by widget instance ID. Each instance is `{ "type": "spotify", "enabled": true, "placement": "bar", "visibility": "available", "position": null, "size": "medium", "variant": "glass", "presence": ["home", "idle"], "order": 0, "settings": {} }`. Placement is `bar`, a screen corner or center, or a left/right rail. Presence is any combination of `home`, `music`, and `idle`. Old v3 documents that predate these fields load with `type` derived from the instance key, medium size, glass variant, home and idle presence, order from their map position, and empty settings. Settings are widget-specific; built-in widgets interpret their own keys.

At startup, v1/v2 current and previous documents are validated and upgraded together in one transaction. Older layouts gain one column and full-width spans. Revisions and restore are preserved. Incompatible/corrupt documents prevent partial migration and are not replaced with defaults. API writes require the v3 shape; read before editing instead of replaying old payloads.

Revisions increase only for actual changes. A stale expected revision conflicts even when the proposed configuration is valid. Restore commits the previous configuration with a new revision and retains the replaced configuration, allowing one-step reversal through REST. Settings separately exposes the last ten saved versions; the REST route remains the one-step compatibility path.

Example from a PowerShell session with the same token available, after migration:

```powershell
$apiBase = 'http://127.0.0.1:9878'
$apiHeaders = @{ Authorization = "Bearer $env:LUMA_CUSTOMIZATION_TOKEN" }
$layoutDocument = Invoke-RestMethod "$apiBase/v1/config" -Headers $apiHeaders
$layoutDocument.config.density = 'compact'
$candidateBody = @{ expectedRevision = $layoutDocument.revision; config = $layoutDocument.config } | ConvertTo-Json -Depth 12
Invoke-RestMethod "$apiBase/v1/config" -Method Put -Headers $apiHeaders -ContentType 'application/json' -Body $candidateBody
```

The example requires a non-null document. For validation, send only `config`. For restore, send the current expected revision.

Errors contain `code` and `message`. Authentication failure is 401; rejected host/origin is 403; conflicts, missing initialization, and unavailable previous configuration are 409; invalid configuration choices are 422; malformed JSON/content and oversized bodies use the corresponding HTTP rejection status; storage/document-version failures are 500. Read again and review the current configuration before retrying a conflict. Failed writes leave committed native data unchanged.

## Ownership and limitations

`customization.sqlite3` lives in Tauri's application configuration directory. SQLite transactions commit the document and previous copy together. This API excludes credentials, executable paths, playback history, fullscreen, startup settings, sound, and shortcuts.

Manual controls, remote edits to migrated fields, and server writes share this store. Frontend synchronization subscribes before initialization, serializes writes, ignores old/duplicate events, and cleans up on teardown. Server writes persist while the launcher is destroyed; the next launcher loads them. Remote palette metadata is supplied by the existing frontend theme cache, so a background-only session may have saved theme/order before the launcher has supplied palettes and choice metadata.

LocalStorage remains a legacy mirror for rollback and unrelated settings. The native document is authoritative once created. Unsupported versions and invalid documents are rejected rather than overwritten with defaults. Database-open failures leave customization unavailable while Luma can still launch and report the storage error. Automatic repair and a corruption-recovery UI are not implemented. Back up the database before manual repair.

Home layout controls in desktop Settings edit a draft. Choose columns and widths, move/hide sections, then Preview and Keep layout. Back to editor retains the draft. Reload saved layout discards draft changes; default layout prepares a draft for review. Restore previous saved customization immediately restores all supported configuration fields as a new revision, including appearance and app order. Other preference controls keep autosaving.

The desktop Home layout section also offers a scoped restore that returns only Home rows and columns to the previous saved layout, preserving theme, sound, widgets, and app order.

Only one native preview can exist. Starting one requires a visible, non-minimized main launcher. Preview writes neither SQLite nor committed localStorage. Keep uses the exact candidate ID and base revision; failed persistence retains it for retry. Any committed write invalidates it. Explicit cancel, main-window destruction, or process exit discards it. HTTP disconnection alone leaves it active because this API has no client sessions; cancel explicitly when abandoning an edit. A stale ID cannot keep or cancel another candidate. Core navigation and Settings remain outside the configurable sections, including when every section is hidden.

Save errors use existing feedback; preview errors also appear next to recovery actions. Revision conflicts load the latest committed configuration. Other failures retain input without retry loops. `preview_busy`, `preview_mismatch`, and `launcher_unavailable` use HTTP 409.

## Verification

Phase 3 checkpoint (2026-11-15): 34 focused Home/renderer/navigation/customization Node tests and 40 native customization tests passed. Production build passed. Official Svelte component analysis reported no issues; suggestions concerning existing effect side effects, class syntax, and temporary Sets were reviewed without unrelated changes. `git diff --check` reported no whitespace errors.

Full-check results are not clean in this shared checkout: the concurrent phone-remote rewrite has frontend type/markup diagnostics, Node phone tests expecting its previous inline implementation, and a native phone-page assertion expecting the previous markup. The full native run at the checkpoint had 105 passing tests and that one phone-page failure. These failures do not replace the pending live desktop/TV/phone acceptance gate below.

Frontend type checking and production build passed. Focused Node tests cover hydration, duplicate/stale events, serialized edits, conflicts, failed writes, and teardown, alongside existing Home/navigation tests. Native tests cover validation, migration idempotence, failed writes, stale revisions, restore, unsupported-version preservation, and file-backed reopening. HTTP tests run the real access middleware on loopback and verify authentication, host/origin checks, and denial before handler execution.

Live Tauri-to-HTTP writes, tray recreation with phone input, TV focus/readability, and startup/idle performance measurements remain manual acceptance work. Storage and middleware tests do not validate those complete desktop journeys.

The earlier broader `cargo check --all-targets` check failed in the existing `examples/window_handoff.rs`: its included background module needed `crate::commands::session` and `crate::tv::restore_audio`, which the example did not define. That example was not changed by the customization work. At the Phase 2 checkpoint, native library tests pass (97), the Node suite passes (71), and the frontend type check/build pass. New tests cover v1-to-v2 migration, preserved undo, component rendering with renamed instance IDs, editor/preview identity, and Home focus recovery.
