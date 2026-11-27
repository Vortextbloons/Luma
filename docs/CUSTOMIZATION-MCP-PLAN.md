# Customization and MCP plan

Status: MCP read/validate/preview, ten-version history, and declarative plugin staging shipped 2026-11-15. Executable runtimes and the transport/SDK experiments remain future work. [PLUGINS.md](PLUGINS.md) owns the exact shipped plugin tools, resources, schema, settings staging, and limits.
Assessed: 2026-11-15 against the working checkout at `ddf0122`, with unrelated phone-remote and Home work in progress. This plan does not depend on that work finishing.
Companion: [the customization layer model](CUSTOMIZATION-LAYERS.md) describes how appearance, composition, and executable extensions relate. This document covers the MCP boundary, review, versioning, and the concrete work needed to make those layers usable.

## How to read the status labels

| Label | Meaning |
| --- | --- |
| **Shipped** | Observed in this checkout; existing verification is recorded in the earlier roadmap. This documentation pass did not rerun tests or establish release status. |
| **Accepted intent** | Agreed for this effort. Implement unless the user revisits it; a change here needs an explicit decision. |
| **Proposal** | Recommended design, not agreed. Adjust during implementation without pretending it was required. |
| **Open** | Needs a product decision or spike evidence before code is written. Each item names a recommendation. |

Statements without one of these labels are background or consequences of the labeled items. Do not cite a **Proposal** as a shipped behavior.

## Shipped baseline

The following are facts about the current checkout, recorded so later sections can build on them rather than rediscovering them.

### Configuration ownership and review

- **Shipped.** `src-tauri/src/customization/` owns a strict version 3 document (`Document { version, revision, config }`) in `customization.sqlite3` under Tauri's application configuration directory. One row holds `document` and `previous`; commits are SQLite transactions (`src-tauri/src/customization/mod.rs:62`).
- **Shipped.** Every mutation takes an expected revision and rejects stale writers. Revisions increase only when the configuration actually changes; restores commit as a new revision instead of rewinding (`src-tauri/src/customization/mod.rs:144`).
- **Shipped.** Validation is closed-world: unknown fields and values are rejected, all seven Home components must appear exactly once, all twelve app entries must appear once with `Custom` last, and columns/spans accept only 1 or 2 (`src-tauri/src/customization/document.rs:178`).
- **Shipped.** Exactly one in-memory preview candidate exists. It has a random id, a base revision, and the validated configuration. Keep requires the matching id and base revision; any committed write, launcher destruction, explicit cancel, or process exit discards it. A stale id cannot keep or cancel a newer candidate (`src-tauri/src/customization/mod.rs:204`).
- **Shipped.** Staging a preview requires a visible, non-minimized `main` window; failure to deliver the preview to that window cancels the candidate (`src-tauri/src/customization/mod.rs:257`).
- **Shipped.** Only one previous configuration is retained. Restore is one step, and the layout-scoped restore replaces only Home rows and columns while preserving theme, sound, widgets, and app order (`src-tauri/src/customization/mod.rs:162`).
- **Shipped.** Version 1 and 2 documents migrate atomically at startup, current and previous together, preserving revisions and one-step restore. Unsupported versions are refused rather than overwritten (`src-tauri/src/customization/mod.rs:67`).

### HTTP surface

- **Shipped.** An opt-in REST server binds `127.0.0.1:9878`, enabled only by `LUMA_CUSTOMIZATION_TOKEN` with at least 32 printable ASCII characters (`src-tauri/src/customization/server.rs:16`).
- **Shipped.** Every route requires the bearer token, an exact `127.0.0.1:9878` Host header, and no `Origin` header; bodies are capped at 32 KiB and responses are `no-store` (`src-tauri/src/customization/server.rs:97`).
- **Shipped.** Token holders can read, validate, preview, Keep, commit directly, restore, and cancel. There is no separate read-only or review-only credential. The documentation states this explicitly (`docs/CUSTOMIZATION-SERVER.md:26`).
- **Shipped.** Routes are `/v1/config`, `/v1/validate`, `/v1/preview`, `/v1/preview/keep`, `/v1/preview/cancel`, and `/v1/restore` (`src-tauri/src/customization/server.rs:60`).
- **Shipped.** A local MCP endpoint listens on `127.0.0.1:9879` when Luma runs and is enabled from Settings → Add-ons → AI access with a session token. It exposes read, validate, preview, plugin list/validate/disabled-stage, and plugin-settings-stage tools; no tool can keep or activate anything (`src-tauri/src/customization/mcp.rs`).

### Frontend, appearance, surfaces

- **Shipped.** `src/lib/services/customization.ts` subscribes before initialization, serializes saves, ignores stale events, reconciles conflicts by reading the native document, and tears down listeners. It exposes preview, Keep, cancel, restore, and layout restore to `src/App.svelte`.
- **Shipped.** Appearance is a fixed set of nine themes built on twenty semantic CSS tokens in `src/lib/services/theme.ts:3`. The Rust remote mirrors the palette, validating token names and hex values, with at most 24 tokens and 12 themes (`src-tauri/src/remote/server.rs:220`).
- **Shipped.** Remote setting presentation is data-driven through `SettingChoice { key, group, section, kind, label, description, options, value }`; the web remote renders choice and toggle kinds from that payload (`src-tauri/src/remote/server.rs:64`, `remote/src/lib/types.ts:49`).
- **Shipped.** Desktop Settings now has eight categories: Appearance & Home, Display & Sound, Apps & Services, Remotes & Devices, History & Privacy, System, Plugins, and MCP customization. Appearance & Home owns Theme, Home layout, and Widgets as Luma's own built-in settings; Plugins owns only Installed plugins and sits directly above MCP customization. MCP customization remains last (`src/lib/components/SettingsPanel.svelte`, `src/lib/services/settingsNavigation.ts`).
- **Shipped.** The Home layout section already presents a separate draft, Preview, Back to editor, reload, default-layout draft, previous-configuration restore, and a Home-only restore.
- **Shipped.** The LAN server on `0.0.0.0:9877` serves the Display Box protocol and the standalone Svelte web remote, with separate pairing credentials. Customization writes from the remote route through the native store (`src-tauri/src/remote/server.rs:165`).
- **Shipped.** Closing the launcher destroys the `main` webview and keeps the process in the tray; `background.rs` rebuilds the launcher on demand, and background startup creates no launcher (`src-tauri/src/background.rs:32`).
- **Shipped.** Tauri capabilities are assigned to the `main` window only (`src-tauri/capabilities/default.json`). The CSP allows `script-src 'self'`, restricts frames to the application origin through the `default-src` fallback, and blocks remote scripts (`src-tauri/tauri.conf.json:29`).
- **Shipped.** A second Luma process is intercepted by the single-instance plugin, which forwards arguments to the running instance and exits (`src-tauri/src/lib.rs:32`).

### Consequences carried forward

- An external agent cannot connect today without the user manually creating a developer token, and that token can bypass review entirely.
- History depth, appearance customization beyond the six palettes, arbitrary composition, custom screens, extension apps, and executable code all remain unimplemented.
- The current preview model assumes a mounted launcher, which conflicts with agent use while Luma sits in the tray. The plan separates staging from rendering to resolve this.

## Accepted intent

These requirements are agreed. They are not yet implemented, and the plan below exists to make them implementable.

1. **Three connected layers.** Appearance, composition, and executable code extensions share one customization service, one review flow, and one version history. Extension packages may contribute appearance, composition, custom settings, apps, screens, and desktop or web-remote UI, plus background services. See [CUSTOMIZATION-LAYERS.md](CUSTOMIZATION-LAYERS.md).
2. **MCP in Settings.** Settings offers a customization/MCP area with a generic, copyable setup prompt. The prompt contains no IDE-specific instructions, no product names beyond Luma and MCP, and no credentials.
3. **User Keep is mandatory.** No agent, remote, or extension can commit a customization candidate without an explicit Keep action in Luma's own UI. This is enforced by the native service, not by the agent's good behavior and not by the frontend alone.
4. **Last ten saved versions.** The service retains ten committed versions, each restorable, including the extension artifacts and extension configuration referenced by that version.
5. **Reset appearance.** A reset returns appearance to the built-in default while preserving accounts and installed apps. The exact boundary is specified below.
6. **Tray support where practical.** Pending review, extension status, and revocation remain reachable when the launcher webview is destroyed. Tray menus show status and open Luma; they do not attempt to render review UIs.
7. **Broad extension capabilities.** Custom settings, app entries, screens, desktop UI, web-remote UI, and background services are in scope for extensions, subject to explicit grants and host-enforced limits.
8. **One product, multiple surfaces.** Desktop, web remote, and extension-provided surfaces expose the same customization concepts. A surface may omit a capability it cannot render safely, but it must not invent a parallel write path.
9. **No agent required at runtime.** Rendering, navigation, remote control, and committed extensions keep working with the agent closed. Revocation stops future writes immediately without breaking the launcher.

## Target flow

```
user request ──> external MCP agent
                      │  (read capabilities, read config, validate, stage)
                      ▼
              native customization service  ── pending candidate + digest
                      │
                      ▼
          Luma review UI (desktop; remote optional)
                      │  user presses Keep
                      ▼
         atomic commit: config + history + extension records
                      │
        ┌─────────────┴──────────────┐
        ▼                            ▼
  desktop / screens            web remote / Display Box
```

The agent never holds the approval path. The preview id it receives identifies a candidate; it cannot be used to commit one.

### Layer responsibilities in this plan

| Layer | What it changes | Authority and artifacts |
| --- | --- | --- |
| Appearance | Palette, density, tile shape, text size, motion, backgrounds, per-screen visual overrides | Bounded token data; no code. Reset has its own boundary below. |
| Composition | Home layout, containers, screens, app entries, widget placement | Registered components and bounded properties; rendered by Luma. |
| Executable extensions | Custom settings, apps, screens, desktop UI, remote UI, background services | Package with manifest, permissions, supervised runtime, and versioned artifacts. |

The companion [layer model](CUSTOMIZATION-LAYERS.md) owns the conceptual detail. This document owns the service boundary, review, history, and delivery.

## MCP connection and setup

### Transport and SDK verification spike

**Proposal.** Resolve transport and SDK with a short, documented spike before exposing tools. Do not choose an SDK from familiarity, and do not assume the protocol version this document was written against is the version a user's client supports.

Spike questions and evidence to record:

1. **Client transports.** For the intended target agents, which transports work today: stdio, Streamable HTTP, or both? Confirm against current official client documentation rather than memory. Record the client name, version, and the exact configuration field the user edits.
2. **Stdio shape.** Luma is a GUI process that the user starts, not a process that a client spawns on demand. A stdio MCP server therefore implies a small separate bridge binary that speaks MCP on stdin/stdout and authenticated loopback HTTP to the running Luma. Verify that the bridge is not intercepted by the single-instance plugin, that it exits when the client closes the pipe, and that a missing or tray-only Luma produces a clear error rather than a hang.
3. **Loopback HTTP shape.** Verify a dedicated MCP route or port distinct from `9877` (remote) and `9878` (existing REST), with the same host/origin discipline, bearer grant validation, body limits, and no CORS. Decide behavior when the port is occupied: a clear startup error naming the MCP endpoint, not silent scanning.
4. **Credential delivery.** Verify that each candidate client can send a custom authorization header or accept a launch command argument/env file without putting secrets in a URL or in chat history.
5. **Protocol and SDK.** Pin a protocol version and SDK version from official sources at implementation time. Record schema-generation approach, error mapping, and whether the SDK is stable enough for the tool surface or needs a thin hand-written adapter.
6. **Process lifecycle.** Confirm behavior when Luma quits, when the launcher webview is destroyed but the tray remains, when the user revokes a grant mid-session, and when the client restarts.
7. **Sandbox reality.** Check whether packaged or sandboxed clients can reach loopback or spawn local processes at all.

Gate: the spike ends with a written decision (transport, SDK, port, credential mechanism, failure states) and one successful end-to-end tool call from a real client. If stdio and HTTP both work, prefer shipping one transport first. **Proposal:** loopback HTTP with a download-free bridge only if the chosen client requires stdio; do not ship two transports in the first slice.

**Open (recommendation):** HTTP first when the target client supports it, because it needs no extra executable and no packaging change. Revisit if the primary client is stdio-only.

### No guaranteed universal auto-connect

**Accepted intent.** Luma must never claim that every agent will connect automatically.

- Client configuration formats differ; some require editing a JSON file, some a settings screen, some a launch command.
- Some clients cannot reach loopback, cannot spawn local processes, or require HTTPS.
- Corporate policy, proxies, and antivirus may block a new local listener.
- Port conflicts and a Luma instance that is not running are normal failures.

Therefore the setup flow is: explain the requirement, offer copyable text, offer a connection self-test inside Luma, and provide troubleshooting for the states above. The self-test proves Luma's side works; it cannot prove the third-party client is configured. State this in the UI copy.

### Setup flow in Settings

**Accepted intent** (steps are **Proposal** detail):

1. Open Settings → Customization → AI access. The server is off by default. The section explains what access does and shows no secret.
2. Choose scopes from the grant list below. The recommended default is read, appearance, and composition. Settings, apps, screens, and extension installation are advanced and individually explained.
3. Choose credential lifetime: this session, or until revoked. Create the grant with a short label so multiple clients stay distinguishable.
4. **Copy setup prompt** copies the generic prompt below. It contains no token.
5. **Copy connection details** copies the transport-specific snippet (endpoint and grant credential, or the bridge command). Luma warns that the text is a credential and must not be pasted into a chat, issue, or repository.
6. **Test connection** performs a local handshake against the endpoint or bridge and reports: reachable, version-compatible, grant valid, or a specific failure.
7. Show the grant list with label, scopes, created time, last use, and a Revoke button. Revoking invalidates the credential immediately and terminates active MCP sessions for that grant.
8. State plainly what the agent cannot do: commit without Keep, read credentials or history, launch applications, change app paths, install without review, or run without a grant.

**Proposal:** one grant per client, several grants allowed. A pending candidate belongs to the service, not to a grant; if the grant is revoked while a candidate is pending, the candidate is discarded and the review panel explains why.

### Generic copyable setup prompt

The prompt must work regardless of which client or model is used. It should contain placeholders only for what the user's client shows, and it must not name editors, extensions, or marketplace steps.

Template:

```
Connect to the Luma MCP server that the user has already added to your MCP client.

Luma is a Windows TV launcher. You can inspect its customization capabilities,
stage changes for the user to review, and read the current configuration.

Rules:
1. If the luma_* tools are not available, stop and tell the user to open
   Luma > Settings > Customization > AI access and add the server shown there.
   Do not guess connection details.
2. Start with luma_capabilities_get and luma_config_get. Do not invent
   components, properties, themes, screens, or extension permissions.
3. Prefer the smallest change that satisfies the request. Ask the user when the
   request is ambiguous, and never change app order, screens, or install an
   extension without saying so first.
4. Validate a complete candidate, then stage it. Tell the user that the change
   is waiting for review and that they must press Keep in Luma.
5. Never claim a change is applied, saved, or live until the user confirms they
   kept it, or until a subsequent read shows a new revision.
6. If validation fails, report the exact error. Do not retry an unchanged
   candidate.
7. Do not ask for, print, or store tokens, passwords, or account credentials.
   Luma never returns them through these tools.
```

**Proposal:** keep this text in the service, versioned with the capability schema, so it stays correct when tools or limits change. The Settings UI should show it in a read-only, selectable block with a copy button and a short note that it is safe to share.

### Capability discovery and guidance

**Proposal.** Discovery is part of the contract, not documentation the agent is expected to already know.

- MCP server `instructions` summarizes the read → validate → stage → Keep loop, the fact that commit requires a user, and the error-handling rules.
- `luma_capabilities_get` returns a versioned capability document: supported layers, component kinds and allowed properties, container/nesting limits, appearance token names and bounds, supported app entries, screen ids, extension permission names, artifact size and count caps, and which surfaces (desktop, remote) support each capability.
- A guidance resource (Markdown) carries worked examples: recolor appearance, rearrange Home, add a screen, add a settings section, stage an extension. Each example states what the agent must not do.
- An error catalog maps service codes to agent actions: `conflict` → re-read; `validation` → report the field; `grant_scope` → ask the user to extend the grant in Settings; `review_pending` → tell the user a review is already waiting; `grant_revoked` → stop.
- Limits are reported as numbers in the capability document rather than being implied by prose. Populate them from validation constants so the two cannot drift.

**Proposal:** expose the capability document by both a tool and a resource so clients that only read resources still see it. Do not expose the current configuration as a resource; reading committed settings belongs to an explicit tool call so logs and conversations are attributable.

### Initial tool surface

**Shipped 2026-11-15:** `luma_capabilities_get`, `luma_config_get`, `luma_candidate_validate`, `luma_candidate_preview`, `luma_candidate_get`, `luma_candidate_cancel`, `luma_status`, `luma_plugins_list`, `luma_plugin_validate`, disabled-only `luma_plugin_stage`, and review-only `luma_plugin_settings_stage`, plus `luma://plugins/schema` and `luma://plugins/example`. [PLUGINS.md](PLUGINS.md) owns the exact contract. No tool enables, keeps, disables, removes, exports, commits, restores, resets, launches, or executes a plugin.

**Proposal.** Read and stage only. Nothing here commits.

| Tool | Scope | Behavior |
| --- | --- | --- |
| `luma_capabilities_get` | read | Capability document and limits |
| `luma_config_get` | read | Current committed document and revision; no credentials, history, or account state |
| `luma_candidate_validate` | read | Validate a complete candidate; return field-level errors; no write |
| `luma_candidate_stage` | write (layer scopes) | Stage a validated candidate as the single pending review; return candidate id, digest, and a summary |
| `luma_candidate_get` | read | The pending candidate this grant created, if any |
| `luma_candidate_discard` | write (own candidate) | Withdraw the pending candidate |
| `luma_versions_list` | read | Last ten versions as metadata: revision, time, source, summary |
| `luma_extensions_list` | read | Installed and staged extensions with enabled state and declared permissions |
| `luma_extension_stage_install` | extensions.install | Place an extension artifact in staging with manifest validation; never activates |
| `luma_grants_get` | read | The scopes and limits of the calling grant |

Explicitly absent: commit, Keep, restore, reset, activate, launch, file-system access, credential reads, history reads, and anything that writes without review. An agent that needs one of these must ask the user to act in Luma.

## Grants, scopes, and service-enforced Keep

### Scope model

**Proposal.** Grants are named, scoped, revocable credentials stored as hashes, never plaintext, in the customization database.

| Scope | Allows |
| --- | --- |
| `read` | Capability, config, version metadata, extension inventory |
| `appearance` | Candidates that change appearance fields |
| `composition` | Candidates that change Home layout, widgets, spans, and containers |
| `settings` | Candidates that change supported preference fields |
| `apps` | Candidates that change app entries and order |
| `screens` | Candidates that add or change screens |
| `extensions.install` | Stage extension artifacts and updates without activation |
| `extensions.configure` | Candidates that change an installed extension's settings |

Rules:

- A candidate that touches several layers requires all matching scopes. The service reports which scope is missing.
- `extensions.install` never implies activation. Activation is a user action in Luma.
- Revocation is immediate and persistent. In-flight calls fail with `grant_revoked`; staged-only artifacts stay staged and inert.

**Open (recommendation):** keep the existing `LUMA_CUSTOMIZATION_TOKEN` REST API as a developer-only capability, off by default and never generated by Settings. Document that it bypasses review, and never mention it in the MCP setup flow. The alternative — removing direct commit from the REST API entirely — is cleaner but breaks the documented developer workflow and is a separate decision. Do not present the developer token as "the way to connect an agent".

### Service-enforced Keep

This is the central security requirement, and it must survive a compromised agent, a confused agent, and a malicious extension.

**Proposal.**

1. Staging computes a canonical digest over: configuration JSON, schema version, the extension artifact ids and content hashes referenced by the candidate, and per-extension settings included in it.
2. The pending candidate record stores digest, base revision, originating grant, creation time, and a summary of what changed.
3. Keep is a Tauri command whose handler accepts only the candidate id. It re-reads the pending candidate, recomputes the digest, and compares it to the id binding. A mismatched or replaced candidate fails with `candidate_stale`.
4. The commit path used by Keep checks the caller is the `main` webview through a capability and window-label check, and that a user interaction occurred in the review UI. There is no HTTP route and no MCP tool that reaches this path.
5. Grant credentials are validated by the native service on every call. A request from an MCP session carries only a grant, so it can never satisfy the approval check.
6. Extension webviews get their own Tauri capability file with no customization, window, updater, or shell permissions. They cannot invoke Keep, read grants, or read the customization document directly.
7. Keep is single-use and bound to the digest. Any committed write from another path invalidates the pending candidate rather than silently merging with it.
8. There is no auto-apply setting, no "trust this agent" bypass, and no keep-on-behalf endpoint. The only exception is the developer-only token described above, which is never part of the MCP product path.

**Proposal:** a candidate expires after a bounded idle period (set the value during implementation) and on process exit. Expiry must be visible to the agent as `review_expired`, not as an indefinite pending state.

**Proposal:** a candidate may be staged while the launcher webview is absent. Staging is service-side. Rendering the candidate still requires a mounted launcher, and the review panel renders it when the user opens Luma. This removes the current limitation that staging requires a visible window without weakening review.

## Reviewing an exact candidate

**Accepted intent:** the user reviews the exact thing that will be committed, not a regenerated approximation.

**Proposal** for the review panel:

- Header: what the agent asked for in plain language if it supplied a summary, the originating grant label, and the base revision.
- Layer sections in a stable order: appearance, composition, screens, apps and settings, extensions. Each section lists concrete differences (for example "Home columns 1 → 2", "Theme onyx → custom: user palette"). Unchanged layers are collapsed, not hidden.
- Validation status and candidate digest prefix, so a technical user can confirm identity. Never show the token.
- Extension section: artifact names, versions, hashes, declared permissions added or removed, and background service declarations. Permission expansion is called out as requiring a new grant.
- Warnings placed above the actions, not below: extension code cannot be rolled back by preview, background services can cause side effects, and any artifact that failed signature or hash checks.
- Actions: Keep, Reject, and Back to customization list. Keep is never the default focused control when the candidate includes executable code or new permissions.
- Focus and input: the panel is reachable with keyboard and both remotes, Back returns to the previous Settings section, and all-hidden Home layouts still leave Settings reachable.
- Failure states: validation failure (cannot Keep), storage failure (candidate retained for retry), conflict (candidate retained, banner explains the base changed elsewhere), expired candidate, and revoked grant.

**Open (recommendation):** allow Keep from the web remote only for candidates that change appearance and composition and contain no extension artifacts; code-bearing candidates must be kept on the desktop. Phone review is convenient, but a small screen at arm's length is a poor place to approve a permission expansion. A banner on the remote can show that a review is pending and offer to open the TV-side panel.

**Proposal:** the remote shows a pending-review banner and a read-only summary. It never receives the agent credential or the candidate digest authority.

## Ten-version history

**Accepted intent:** the last ten saved versions are kept and restorable.

**Proposal.**

- Add a `versions` table keyed by revision: creation time, source (`manual`, `remote`, `agent`, `extension`, `migration`, `reset`, `restore`), summary, full configuration JSON, the digest, and the referenced extension artifact records.
- Every commit, restore, reset, and extension install or update that changes customization writes one version row in the same SQLite transaction as the configuration change.
- Keep the newest ten rows. Prune older rows after a successful commit. Revisions never rewind; restoring version N writes a new revision N+1 whose content equals version N.
- The existing single `previous` column becomes the newest history row. Startup migration stores the current `previous` document as a history entry so one-step restore survives the change.
- Extension artifacts are recorded by id, version, and content hash. Artifact bytes are retained while they appear in the ten-version window when the size cap allows it. When bytes were pruned, deleted externally, or exceed the cap, the version still restores configuration but the extension stays disabled with an explicit "artifact missing" state. Never fabricate a substitute artifact.
- Restoring a version that includes extension references does not activate anything. Activation follows the normal path: explicit enable, permission check, then host start.
- History is user-visible in Settings → Customization → Versions as a compact list: time, source, what changed, Restore. Restoring is a user-initiated commit and does not require an agent grant.
- History never stores tokens, account credentials, pairing codes, playback history, or app executable paths. It stores only customization data.

**Proposal:** pruning is by count, not by age, so a user who has not customized for months still has ten versions. Storage growth is bounded by ten configurations plus retained artifacts; set the artifact cap from measured package sizes rather than guessing.

**Proposal:** version entries are immutable. Editing a version, naming a version, and export are not part of this slice.

## Reset appearance

**Accepted intent:** reset appearance to the built-in default while preserving accounts and apps.

**Proposal** for the boundary:

| Field or artifact | Reset behavior |
| --- | --- |
| Theme, palette tokens, custom palette definitions | Returned to the built-in default theme and its tokens; custom palettes removed from active appearance |
| Density, tile shape, text size, reduced motion | Returned to defaults |
| Per-screen appearance overrides | Removed |
| Background image or appearance-related media reference | Cleared from appearance; the file is not deleted |
| Home rows/order/visibility, columns, spans, widget visibility/placement | Returned to defaults; confirmation names layout and widgets |
| Screens, app entries, app order | Unchanged |
| Extension installs, enabled state, extension settings | Unchanged; any extension that contributed the active appearance becomes dormant for appearance, not uninstalled |
| Accounts, credentials, pairing | Unchanged |
| App paths, detected applications | Unchanged |
| Activity history and privacy choices | Unchanged |
| Version history | Unchanged; the reset itself is a new version entry and earlier versions stay restorable |

**Open (recommendation):** keep interface sound set and level out of "appearance" reset, because they describe audio feedback rather than visual style. If the user considers sound part of appearance, add those two fields and say so in the confirmation dialog. The dialog must list exactly which categories reset, since this is the boundary users will question.

**Proposal:** reset is a single command, reports the new revision, and requires confirmation in the desktop UI. It is available from Settings and is not exposed as an MCP tool.

## Executable extensions

**Accepted intent:** extensions may add functionality broadly: custom settings, apps, screens, desktop UI, web-remote UI, and background services. Broad does not mean unlimited native access.

### What an extension is

**Proposal:** a package with:

- identity: id, name, author display, version, minimum and maximum Luma API version;
- entry points by surface: UI bundle, background service, settings schema, app entries, screen definitions;
- declared permissions, each a named capability with a human-readable reason;
- declared background behavior: none, on-demand, or always-on; timers or event subscriptions; whether it needs the launcher mounted;
- storage declaration: namespace and estimated size;
- appearance and composition contributions, which are declarative documents, not code;
- artifact content hash for every file, and an optional publisher signature.

Manifest limits (nesting depth, file count, artifact size, permission count) are enforced by the host at staging time and published in the capability document.

### Staging installation versus activation

This distinction is required in the UI, the API, and the docs, because it is where users get hurt.

- **Staging** validates the manifest, hashes, version compatibility, and permissions; unpacks into a versioned directory keyed by content hash; writes a `staged` database record; and shows the artifact and permission diff in review. It does not run code, does not start services, does not register settings pages, and does not mount UI. Staging may happen while the launcher is absent.
- **Activation** happens only after a user Keep or an explicit Enable for an already-installed extension. It checks grants and permissions, starts the host, mounts the UI or registers the settings section, and flips the record to `enabled`. A failed activation leaves the extension `disabled` with a visible error, bytes intact for retry or diagnosis.
- **Deactivation** stops the host and its services, unmounts UI, unregisters settings contributions, releases resources, and preserves extension storage unless the user removes the extension.
- **Removal** deactivates first, then deletes artifacts and asks whether to delete extension storage.
- **Update** stages the new version beside the old one, shows permission additions, and requires a new grant when permissions expand. Rollback selects the previous version without replaying any external effects.

**Proposal:** cross-store atomicity. The configuration, history row, extension record, and activation intent commit in one SQLite transaction. Artifact files are written before the transaction and garbage-collected on the next start if the transaction failed. A crash must never leave an extension marked enabled without its bytes, or bytes without a record.

### Preview cannot undo code side effects

**Accepted intent.** Preview is honest about what it can and cannot prove.

- Appearance and composition candidates are data. Preview renders them; rejecting restores the previous render with no external effect.
- Extension candidates cannot be previewed by running them. Running code may make network requests, write files outside Luma, display windows, change an account, or spend money. Those effects are not undoable by Keep, Reject, version restore, or reset.
- Therefore extension review is static: manifest, artifact hashes or signatures, declared permissions, background behavior, settings schema, and a rendered placeholder for contributed UI with declared size and empty state. The UI says the extension has not been run.
- After activation, the ten-version history restores configuration and artifact selection, not the outside world. Say this in the review dialog and in the extension details page.
- A crash, hang, or policy violation disables the extension and preserves the user's configuration and other extensions. It must never block the launcher, Settings, or recovery controls.

### Background services

**Proposal.**

- Background execution is a separate permission and a separate toggle, never implied by installing a UI extension.
- The host supervises each service: start, stop, restart on crash with bounded backoff, and hard stop on deactivate, uninstall, revoke, or application exit.
- Services may run while the launcher webview is destroyed, but only if the user granted background execution. Tray mode keeps them; quitting Luma stops them.
- Service output is delivered to the launcher through the host when a launcher exists; with no launcher, events are dropped rather than queued without bound. State that clearly so extensions do not promise delivery.
- Resource limits, startup delay budgets, and crash-loop thresholds are set from measurement, not invented here.
- Network access, local listeners, and credential access are separate permissions with specific host-mediated operations. An extension never receives raw account tokens.
- Windows sleep, session change, and forced termination are not controllable by Luma. Do not promise guaranteed continuous background execution.

### Surface integration

**Proposal**, with concrete integration points that must change:

| Capability | Surface behavior | Integration requirement |
| --- | --- | --- |
| Custom settings | Desktop Settings section plus remote controls through the existing `SettingChoice` contract, namespaced per extension | Host validates values against the extension schema; remote accepts only declared keys for enabled extensions (`src-tauri/src/remote/server.rs:165`) |
| App entries | Extension-provided app tiles and order participation | `APP_IDS` and `validate_order` must become registry-aware; `Custom`-last invariant revisited; remote app order payload and `normalize_app_order` updated together (`src-tauri/src/customization/document.rs:10`, `src-tauri/src/remote/server.rs:288`) |
| Screens | Registered screen id, reachable from desktop navigation; optional remote entry | Screen registry with stable ids; `view` reporting and remote navigation command must accept only registered ids; Back/Escape always returns to Home (`src/App.svelte:321`) |
| Desktop UI | Composition nodes, widgets, overlays rendered inside Luma with host-supplied data | Isolated runtime and narrow bridge; no direct Tauri command access for extension content |
| Web remote UI | Declarative panels or settings sections rendered by the remote app | Remote renderer supports only known control kinds; no arbitrary HTML or script in the remote page (`remote/src/lib/types.ts:49`) |
| Background service | Supervised host process with declared permissions | Tray lifecycle, exit cleanup, and supervisor in `background.rs` / a new extension module |

**Proposal:** every one of these surfaces reports the same customization revision and the same pending candidate. Do not let the remote, a screen, or an extension create a second writer.

### Trust and distribution boundaries

**Proposal for this effort:** support local, side-loaded packages with explicit warnings. Verify content hashes; verify a publisher signature when present; require a new grant for permission expansion.

Out of scope until separately approved: a marketplace, automatic package updates, paid distribution, native DLL or driver loading, arbitrary shell or executable-path changes, and extensions that can read Luma account credentials. Native Windows integration that needs broad access stays a separately reviewed Luma feature, not a third-party package capability.

## Surfaces and the tray

**Accepted intent:** tray support where practical.

**Proposal.**

- Tray menu: Open Luma, Review pending change (only when one exists, opens Settings review), Extensions running/stopped summary, Exit Luma.
- No Keep from the tray. A tray menu cannot show the diff and is the wrong place to approve permissions.
- The tray icon or tooltip reflects a pending review and extension failure states only if it can do so without new dependencies and without polling. Event-driven updates only.
- Closing the launcher while a candidate is pending keeps the candidate (until expiry) and keeps background services running per their grants. Reopening Luma restores the review panel.
- Revoking a grant never requires the launcher to be open; if it is closed, the revocation applies on the next start and the credential is invalid immediately in the service.
- The web remote shows the pending review banner and extension status but does not receive secrets or approval authority.

## Concrete work areas

Ordered by dependency, not by calendar. Each area names the primary files; details belong in implementation, and no area authorizes a repository-wide refactor.

### A. Foundations: history, grants, approval gate

`src-tauri/src/customization/history.rs` (new), `grants.rs` (new), `mod.rs`, `src/lib/services/customization.ts`.

- History table, migration of the single `previous` value, pruning to ten, restore-as-new-revision, source labels.
- Grant storage with hashed credentials, scope checks, revocation, and last-use tracking.
- Candidate digest, pending-candidate record with base revision and grant, expiry, and the `main`-only approval path.
- Separate staging from rendering so candidates can be created while the launcher is absent.
- Existing commands keep working; new commands added for history, grants, reset, review, and pending state.

Acceptance:

- Ten versions round-trip; the eleventh prunes the oldest; restoring an old version creates a new revision and leaves ten entries.
- A grant without the required scope cannot stage that layer; revocation fails the next call immediately.
- No HTTP route or grant can commit; review requires the desktop approval command and a candidate digest match.
- A stale or replaced candidate cannot be kept; a failed commit retains the candidate.
- A pending candidate survives launcher destruction within its expiry and is gone after process restart.

### B. Transport and SDK spike

Scratch implementation experiments only; the durable output is a decision record appended to this document.

Acceptance:

- One real client completes handshake, capability read, and config read against a running Luma.
- Recorded evidence for: stdio bridge interception by single-instance, port conflict behavior, revocation mid-session, client restart, Luma restart, and sandboxed-client failure mode.
- A named SDK, protocol version, transport, port, and credential-delivery decision, including what was rejected and why.

### C. MCP adapter, discovery, and setup UI

`src-tauri/src/customization/mcp/` (new), `server.rs`, `src/lib/components/SettingsPanel.svelte`, new settings components.

- Tool schemas generated from the same constants that validate configuration, so limits cannot drift.
- Capability document, instructions text, guidance resource, error catalog.
- Settings → Customization category with AI access (grants, copy prompt, copy connection details, test connection, revoke), Versions, Extensions, and Reset appearance.
- No MCP tool can commit, restore, reset, activate, or launch.

Acceptance:

- The copyable prompt contains no client-specific instructions and no credential.
- The setup self-test distinguishes "Luma not running", "server disabled", "invalid grant", and "version mismatch".
- An agent with read scope can discover supported components and receive an actionable error for an unsupported request.
- An agent with appearance scope can stage a recolor, and the user can Keep or Reject it on the desktop.
- An agent cannot stage a composition change with an appearance-only grant.
- With the agent closed, the launcher renders and navigates normally.

### D. Review experience

`src/lib/components/` review panel and Settings wiring; optional remote banner.

- Exact-candidate diff, digest prefix, permission expansion, side-effect warnings, failure states, focus and remote behavior.
- Keep requires deliberate action; with code or new permissions, Keep is not the initially focused control.
- Pending state is visible from Settings and the tray entry point.

Acceptance:

- Keyboard and both remotes can open review, move through every layer, Keep, Reject, and return with focus preserved.
- A validation failure and a revision conflict both keep the candidate and explain the recovery.
- A code-bearing candidate is clearly labeled as not executed and not revertible by preview.
- Every hidden Home section still leaves Settings and recovery reachable.

### E. Appearance layer beyond built-in themes

`src/lib/services/theme.ts`, `document.rs` validation, remote appearance sync, Settings theme section.

- Custom palettes and background references as configuration data with bounded tokens and validated values.
- Remote and Display Box receive the resolved palette, not arbitrary CSS.
- Reset appearance uses the documented boundary.

Acceptance:

- Custom appearance validates, survives restart, and reaches the remote; unknown tokens and invalid colors are rejected.
- Reset restores default appearance and leaves accounts, app paths, app order, extensions, and history untouched.
- Remote theme selection still works and cannot send a palette Luma would reject.

### F. Registry-aware apps and screens

`document.rs`, `src/lib/services/home.ts`, `src/lib/types/index.ts`, `remote/server.rs`, `App.svelte`, `HomeRenderer.svelte`.

- Version 4 document when app entries, screens, or composition nodes need new shapes; atomic migration from v3, current and previous.
- Screen registry with stable ids, validated navigation, and Home reachability.
- App entries from the registry rather than a fixed twelve-name list, while keeping the remote order contract consistent.

Acceptance:

- A custom screen can be opened from the desktop, remote-listed only if registered, and always exits to Home with Back or Escape.
- Extension app entries participate in ordering without breaking existing stores, and an incompatible document is refused rather than rewritten.
- v1/v2/v3 documents migrate atomically with revisions and history preserved.

### G. Extension staging and activation without background services

`src-tauri/src/customization/extensions/` (new), capability files, Settings extension pages.

- Manifest validation, hashing, staged directory layout, record states, activation and deactivation, settings schema and rendered settings, contributed appearance/composition, UI runtime (spike-gated).
- Isolated runtime with a narrow bridge and no inherited native permissions; CSP and capability file evidence.

Acceptance:

- Staging an extension runs no code and registers no settings or UI.
- Activation requires an explicit user action and a matching grant; permission expansion requires a new grant.
- A crashing or incompatible extension is disabled without affecting the launcher or other extensions.
- Uninstall stops work and releases resources; storage deletion is an explicit choice.
- Preview clearly states that it did not run the extension and cannot undo its effects.

### H. Background services and tray

Extension supervisor, `background.rs`, exit path in `lib.rs`.

- Background permission, supervisor lifecycle with bounded backoff, tray status, exit cleanup, launcher-absent event behavior.
- Prefer event-driven updates; use bounded polling only when an external provider requires it, and stop it with its owner.

Acceptance:

- Services start and stop with grants, survive launcher destruction only when background execution was granted, and stop on exit or uninstall.
- A crash loop is contained and visible; it does not spin the CPU or block the launcher.
- Tray review entry appears only when a candidate is pending and opens the review panel.

### I. Web remote extension surface

`remote/src/`, `remote/server.rs`, phone payload types.

- Namespaced extension settings through the existing contract; pending-review banner; extension status.
- Custom extension screens use the runtime boundary selected by the layer plan; declarative settings use the shared settings contract. No account secrets or commit authority for code candidates are exposed to remote UI.

Acceptance:

- Only declared settings for enabled extensions appear, and rejected values surface a clear message on both surfaces.
- The remote cannot keep a code-bearing candidate and explains where to finish review.
- A remote-only user can still see that Luma is healthy and that a change is waiting.

## Cross-cutting acceptance

- Committed customization survives restart and tray recreation; the remote and desktop agree on revision and content.
- Every write path — desktop control, agent candidate, remote setting, extension install — goes through the native service and the same revision checks.
- Every failure state has user-visible, actionable copy and does not lose the user's input or committed data.
- Back, focus, and Home remain predictable on a 1080p and a 4K display with keyboard and both remotes.
- Startup and idle resource use are measured on the target machine before any equivalence or improvement claim. Background services are measured visible, hidden, and with the launcher destroyed.
- Existing developer REST behavior is documented honestly, including its review bypass, and is absent from the MCP setup flow.

## Open decisions

1. **Transport for the first release.** Recommendation: loopback HTTP if the chosen client supports it; add a stdio bridge only when a required client cannot. Decide from spike evidence.
2. **Fate of the developer REST token.** Recommendation: keep it, label it developer-only, and never generate or display it from Settings. Removing direct commit is a separate, later decision.
3. **Remote Keep.** Recommendation: appearance and composition only, no extension artifacts or permission changes.
4. **Reset appearance and sound.** Recommendation: reset visual appearance only; leave interface sound set, level, and audio output untouched.
5. **Extension trust model for the first release.** Recommendation: side-loaded packages with hash verification, explicit warnings, and new grants on permission expansion; no marketplace and no signing requirement initially.
6. **UI runtime for extension content.** Recommendation: decide only from the isolation spike; declarative-only extensions ship first if the runtime is not ready.
7. **Whether extension-contributed app entries may launch executables.** Recommendation: only user-configured paths or a declared launch action with an explicit grant and a confirmation; never an agent-supplied path.
8. **History artifact retention size.** Recommendation: retain within a measured cap; missing bytes degrade to config-only restore with an explicit disabled extension.
9. **Settings placement.** Recommendation: one new top-level Customization category rather than spreading these controls through six existing categories. Note the navigation cost of a seventh category.

## Explicit non-goals

- No embedded AI chat, model credentials, or agent execution inside Luma.
- No automatic application of agent changes, and no "trust this agent" bypass of Keep.
- No arbitrary code in the main webview, no native DLL or driver loading, no shell execution from customization data.
- Initial package sharing and later catalog/marketplace decisions follow [the distribution plan](CUSTOMIZATION-DISTRIBUTION.md). Paid distribution and automatic updates are not selected for the initial slice.
- No export of credentials, pairing tokens, playback history, or installation-specific paths in customization documents.
- No guarantee that a specific third-party client connects, and no promise that background services run continuously across sleep or forced termination.

## Verification status of this document

This is a planning document; no build, type check, test suite, or manual acceptance run was performed for it. Every claim labeled **Shipped** was read from the files cited above. Claims about clients, SDK versions, sandbox behavior, and resource cost are unverified until the spikes and measurements named in the work areas are completed. Update [CUSTOMIZATION-SERVER.md](CUSTOMIZATION-SERVER.md) as implementation lands, and keep shipped facts separate from the proposals here.
