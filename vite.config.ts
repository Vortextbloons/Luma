import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import tailwindcss from "@tailwindcss/vite";

// @tauri-apps/cli sets this when running dev against a mobile device.
const host = process.env.TAURI_DEV_HOST;

// https://vite.dev/config/
export default defineConfig({
  plugins: [svelte(), tailwindcss()],

  // Vite options tailored for Tauri development.
  // 1. Don't clear the screen so Rust compiler errors stay visible.
  // 2. Use a fixed port that src-tauri/tauri.conf.json points at.
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host ? { protocol: "ws", host, port: 1421 } : undefined,
    watch: {
      // Rust rebuilds on their own.
      ignored: ["**/src-tauri/**"],
    },
  },
});
