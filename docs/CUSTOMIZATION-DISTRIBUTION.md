# Extension sharing, bundled examples, and themes

Status: product planning, 2026-11-15. No package loader, marketplace, or example extension is implemented by this document. This supplements [the layer architecture](CUSTOMIZATION-LAYERS.md), [MCP and recovery](CUSTOMIZATION-MCP-PLAN.md), and [remote surfaces](CUSTOMIZATION-DISPLAY-BOX.md).

## Intended experience

Product update: initial use is a small family tool. Direct sharing is the selected initial distribution approach; no public catalog, publisher accounts, or marketplace service is required. The AI creates an extension in its working environment, stages it through Luma, and exports a self-contained shareable file. The user sends that file through their existing sharing method; a recipient chooses Import extension in Luma, reviews compatibility/capabilities, previews where supported, and explicitly accepts activation. Luma and its agents do not send files or messages to other people automatically.

Appearance/composition exports can be plain versioned JSON when no bundled assets or code are needed. Code extensions use one archive containing readable source, runtime-ready assets/code, and a manifest; the precise suffix and build format remain runtime decisions. Recipients should not need a source checkout, coding IDE, AI account, or local build toolchain to install an exported extension. A source folder may also be shared with another coding agent for editing, but importing an installable file is the normal family flow. Do not compile or execute arbitrary install scripts during import.

Export includes only the selected extension, its redistributable assets, and portable visual configuration. Exclude credentials, machine paths, pairing details, personal library/history, and runtime storage. Missing account setup is completed on the recipient's installation. Updating is another explicitly imported version using the same review and recovery path. Export/import remain available without a marketplace or online account.

Users can install other people's extensions and select bundled examples. Distribution must use the same extension contracts as locally AI-authored code. Bundled extensions must not rely on private host APIs that outside authors cannot access.

Start with installation from a local package file. A curated catalog can subsequently point to downloadable versioned packages. A public marketplace with publisher accounts, submissions, review, search, reporting, hosting, and update infrastructure is a later service decision, not a prerequisite for sharing packages. VS Code likewise supports package distribution independently of its marketplace: [official packaging documentation](https://code.visualstudio.com/api/working-with-extensions/publishing-extension).

## Distribution stages

### Portable authoring and file workflow

The normal authoring unit is a folder, not a single pasted code snippet. Keep the proposed existing `luma-extension.json` manifest at its root, with package-relative source, runtime entry files, and assets. The extension ID and version identify the package regardless of its folder name or installation location. Do not embed the author's project directory in imports, artwork paths, or runtime entry points.

Proposed sharing filename: `<extension-id>-<version>.luma-extension.zip`, an ordinary ZIP that can also be opened with standard archive tools. The manifest schema/API version and runtime entry format remain dependent on the runtime gate; the filename is a convention, not a shipped parser. A plain appearance-only preset may remain versioned JSON. An extension with several source files, themes, icons, and target-specific views still exports as one archive.

The authoring journey is: AI discovers the public extension API and examples through MCP; creates or edits the folder using its existing coding tools; validates the manifest and build outputs; requests staging through the future extension-install capability; user reviews and activates; user exports the accepted immutable package. MCP access currently implemented for configuration preview does not yet stage or export extension code. Do not claim those future tools exist.

The recipient chooses **Import extension**, selects the shared file, reviews identity/version/compatibility/capabilities, and accepts installation/activation. Luma copies it into its own application-data directory; it does not keep running from Downloads, an email attachment, a USB drive, or the author's folder. Moving or deleting the shared archive after installation must not break the installed extension. All referenced assets must be bundled. External services remain explicit dependencies and show their own setup/disconnected states.

The author can choose **Export extension** from an installed extension's detail page and save a fresh copy anywhere. This exports code/source/assets and portable settings, not caches, runtime storage, or credentials. Another AI can unpack that file, edit its source, rebuild it using the documented author toolchain, increment the version, and return an updated package. The recipient's local settings and credentials remain local when replacing a compatible version. Luma must refuse a different payload with the same installed ID/version and require a new version; it must not silently overwrite artifacts referenced by history.

Acceptance includes moving the package to a differently named folder on a second PC, installing without author tools, deleting the imported ZIP, restarting, switching themes, exporting again, and verifying that the installed extension still works. Imports must validate archive paths, duplicate/case-colliding filenames, symlinks, entry existence, file counts, and compressed/uncompressed size limits before activation. Set practical limits from representative packages. Import never executes build or install scripts.

1. Define one package format and implement local import, manifest inspection, compatibility checks, staging, user review, activation, disable/remove, and recovery. Show identity, version, origin, supported surfaces, requested capabilities, and background activity before activation. Reject unsafe archive paths and incompatible packages before extraction/activation. Package content must be immutable once accepted; retain artifacts needed by the ten-version history.
2. Add a curated catalog using the same installer. Define catalog ownership, package integrity, publisher identity, and unavailable/removed-version behavior before enabling downloads. A checksum verifies bytes against trusted metadata; it does not establish author trust by itself. Updates are explicit candidates, with renewed approval for capability expansion. Existing installed versions keep working when the catalog is offline.
3. Consider open publication only after install/update/recovery are proven. Define publisher verification, moderation/reporting, compromised-package handling, storage/bandwidth ownership, and operational responsibility. Do not promise a public marketplace release date before these decisions.

Relative effort: local package sharing is bounded product work once the runtime and lifecycle exist; a curated catalog adds download/discovery infrastructure; an open marketplace adds continuing service and moderation work. These are complexity comparisons, not measured estimates.

## Theme contract

The existing `src/lib/services/theme.ts` already supplies semantic surface, text, accent, border, and feedback tokens. New host components and extension views should consume a versioned public appearance contract derived from those tokens, with text size, density, reduced motion, and visible focus behavior. Isolated extension views receive an initial snapshot and appearance-change events; document-root CSS variables alone cannot cross an isolation boundary. Native Display Box maps supported values to Android Views.

Distinguish a theme pack, which supplies appearance values, from a UI extension, which changes composition and behavior. A custom music interface should follow the selected Luma theme by default. Optional extension-specific accents must remain scoped to that extension. Theme packs require coordinated changes to the native fixed-theme validator, frontend types/selection, persistence, discovery, and remote appearance contract; adding a CSS file alone does not complete theme support.

## Bundled reference extensions

Use a small appearance pack to prove the surface layer, then an Apple Music-inspired music interface to exercise composition and executable UI. This is a visual reference, not a new Apple Music account or playback integration. Reuse existing authorized music capabilities and their shared owners; expose unavailable functionality honestly rather than inventing unsupported library/search/queue APIs.

Before implementation, confirm whether the music example targets Luma desktop, the web remote, Display Box Music, or several surfaces. The current Luma widget registry contains a Spotify MiniPlayer; Display Box has its own richer native Music experience. These are different implementation targets.

The example should demonstrate screen contribution, supported browsing/playback actions, shared data ownership, extension settings, theme changes, loading/empty/error/disconnected states, and keyboard/remote focus and Back behavior. A phone presentation and bounded native Display Box presentation can reuse the same extension logic where the chosen scope requires them.

Install the example through the public package lifecycle, even when bundled with Luma. Verify disable, restore, update, and removal without breaking the default music experience or account state. Compare a built-in and externally imported copy using the same APIs. Record API gaps discovered by building it and resolve those gaps in the public contract instead of adding example-only host hooks.

## Acceptance

- A package exported by one installation imports on another without personal paths, credentials, or account data.
- An incompatible, malformed, or rejected package leaves the running configuration intact.
- A staged executable package does not gain background execution merely because its metadata is being reviewed. Executing a live preview requires explicit capability approval; external effects cannot be undone by layout history.
- Bundled and third-party views respond to theme changes without restart, retain readable focus, and release appearance subscriptions when closed.
- The reference music extension works through shared playback/data owners; switching views does not create a second player or login session.
- Restoring history can locate the exact retained extension artifacts. Missing artifacts produce a recoverable outcome rather than silently substituting new code.
