# Luma TypeScript plugin starter

This starter builds an API v2 `isolated-ui` package that reads its own settings through `@luma/plugin-sdk`.

## Build

```powershell
npm install
npm run build
```

The build writes the importable package to `dist/`:

```text
dist/
  luma-extension.json
  ui/
    index.html
    main.js
    style.css
```

`dist/` is the directory to validate, pack, and serve in a development session.

## Run in a development session

Open this folder in VS Code and press F5. The `Luma plugin: dev host` configuration runs the `luma-plugin: build` task and then launches `luma-plugin dev`. The CLI owns the disposable Luma instance, reload loop, and runtime diagnostics.

Use **Terminal → Run Task → luma-plugin: build** after an edit when working outside the launch configuration. The CLI watches the built package output, so it reloads after a successful build.

The CLI process runs under the VS Code Node debugger through the launch configuration. Build and validation output appears in the integrated terminal. Plugin surface DevTools belong to the development host; use its documented development action for the active surface.

## Validate and pack

Pass the built directory to the CLI:

```powershell
luma-plugin validate dist
luma-plugin pack dist
```

The CLI is the owner of the argument names, diagnostics, and archive output. The package format remains `<id>-<version>.luma-extension.zip`.

## Schema completion

The repository template uses the SDK and schema from the Luma checkout. `luma-plugin init` copies both into the new project and configures the local paths so builds and manifest completion also work outside the checkout.

The Rust validator remains authoritative for file existence, limits, and cross-field rules that JSON Schema cannot check. Do not add `$schema` to the manifest; Luma rejects unknown manifest fields.

## Plain JavaScript

The SDK package is plain ES module JavaScript with type declarations. JavaScript packages can call the same `extension_bridge` protocol directly. See [the SDK guide](../../docs/PLUGIN-SDK.md).
