# Luma Svelte plugin starter

This starter builds an API v2 `isolated-ui` package using Svelte 5 runes, Tailwind CSS, TypeScript, and Vite. It reads and refreshes its own settings through `@luma/plugin-sdk`.

## Build

```powershell
npm install
npm run build
```

The build writes the package distribution to `dist/`:

```text
dist/
  luma-extension.json
  ui/
    index.html
    index.js
    index.css
```

`dist/` is the directory to validate, pack, and serve in a development session.

## Run in a development session

Open this folder in VS Code and press F5. The `Luma plugin: dev host` configuration runs the `luma-plugin: build` task and then launches `luma-plugin dev`. The CLI owns the disposable Luma instance, reload loop, and runtime diagnostics.

Use **Terminal → Run Task → luma-plugin: build** after an edit when working outside the launch configuration. The CLI watches the built package output and reloads after a successful build.

## Validate and pack

Pass the built directory to the CLI:

```powershell
luma-plugin validate dist
luma-plugin pack dist
```

## Schema completion

The repository template uses the SDK and schema from the Luma checkout. `luma-plugin init <dir> --template svelte` copies both into the new project and configures the local paths so builds and manifest completion also work outside the checkout.
