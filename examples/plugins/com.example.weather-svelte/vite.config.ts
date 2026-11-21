import { copyFileSync } from "node:fs";
import { resolve } from "node:path";
import { defineConfig, type Plugin } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import tailwindcss from "@tailwindcss/vite";

function copyPluginManifest(): Plugin {
  return {
    name: "copy-plugin-manifest",
    closeBundle() {
      copyFileSync(
        resolve(import.meta.dirname, "luma-extension.json"),
        resolve(import.meta.dirname, "dist/luma-extension.json"),
      );
    },
  };
}

export default defineConfig({
  plugins: [svelte(), tailwindcss(), copyPluginManifest()],
  base: "./",
  build: {
    outDir: "dist",
    emptyOutDir: true,
    assetsInlineLimit: 0,
    rollupOptions: {
      input: {
        index: "ui/index.html",
      },
      output: {
        entryFileNames: "ui/[name].js",
        chunkFileNames: "ui/[name].js",
        assetFileNames: "ui/[name].[ext]",
      },
    },
  },
});
