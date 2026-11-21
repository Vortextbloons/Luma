# Luma Weather plugin (Svelte)

This is an API v3 Svelte 5 + Tailwind equivalent of the Luma Weather plugin example. It reads location and unit settings through `@luma/plugin-sdk`, queries Open-Meteo through the host network bridge, listens for the `openForecast` Home card command, and provides TV-first directional navigation.

## Build

```powershell
npm install
npm run build
```

The build compiles the package distribution into `dist/`:

```text
dist/
  luma-extension.json
  ui/
    index.html
    index.js
    index.css
```

`dist/` is the statically validated plugin package directory.

## Validate and pack

```powershell
luma-plugin validate dist
luma-plugin pack dist
```
