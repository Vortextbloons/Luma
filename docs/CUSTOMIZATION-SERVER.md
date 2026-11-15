# Local customization server foundation

Implemented: versioned native ownership of existing Home/appearance preferences and a local REST API. MCP, custom layout containers, plugin loading, and AI chat remain future work. See [the roadmap](CUSTOMIZATION-ROADMAP.md).

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

Every route requires `Authorization: Bearer <token>` and the exact host `127.0.0.1:9878`; use that IP rather than `localhost`. Requests containing Origin are rejected. There is no CORS/browser UI. Request bodies are limited to 32 KiB. A token holder can commit supported changes directly; preview/Keep is not implemented in this slice.

| Method | Route | Body | Result |
| --- | --- | --- | --- |
| GET | `/v1/config` | None | Current document, or `null` before migration |
| POST | `/v1/validate` | Configuration object | `{ "valid": true }` or validation error; no write |
| PUT | `/v1/config` | `{ "expectedRevision": number, "config": configuration }` | Committed document |
| POST | `/v1/restore` | `{ "expectedRevision": number }` | Previous configuration committed as a new revision |

A document has `version: 1`, a positive `revision`, and `config`. Config contains `theme`, `density`, `appTileShape`, `home`, `widgets`, and `order`. Read the current document and modify its configuration: commits replace the complete supported configuration, not a partial patch. Unknown fields/values are rejected. Home must contain all seven built-in rows exactly once; app order must contain all twelve apps once with Custom last. Hidden rows retain their position.

Revisions increase only for actual changes. A stale expected revision conflicts even when the proposed configuration is valid. Restore commits the previous configuration with a new revision and retains the replaced configuration, allowing one-step reversal. It is not a history browser.

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

Save errors use Luma's existing message feedback. Revision conflicts load the latest committed configuration instead of silently overwriting another writer. Other write failures retain edited controls and do not retry in a loop; another edit can trigger a new save. A full draft/review UI is deferred.

## Verification

Frontend type checking and production build passed. Focused Node tests cover hydration, duplicate/stale events, serialized edits, conflicts, failed writes, and teardown, alongside existing Home/navigation tests. Native tests cover validation, migration idempotence, failed writes, stale revisions, restore, unsupported-version preservation, and file-backed reopening. HTTP tests run the real access middleware on loopback and verify authentication, host/origin checks, and denial before handler execution.

Live Tauri-to-HTTP writes, tray recreation with phone input, TV focus/readability, and startup/idle performance measurements remain manual acceptance work. Storage and middleware tests do not validate those complete desktop journeys.

The broader `cargo check --all-targets` check fails in the existing `examples/window_handoff.rs`: its included background module needs `crate::commands::session` and `crate::tv::restore_audio`, which the example does not define. That example was not changed by this work. Native library tests pass (84), the Node suite passes (54), and the frontend type check/build pass.
