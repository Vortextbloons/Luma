import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import tailwindcss from "@tailwindcss/vite";

const backend = "http://localhost:9877";

function proxyToBackend() {
  return {
    target: backend,
    changeOrigin: true,
    configure(proxy: { on: (event: string, handler: (request: { setHeader: (name: string, value: string) => void }) => void) => void }) {
      const rewrite = (request: { setHeader: (name: string, value: string) => void }) => request.setHeader("origin", backend);
      proxy.on("proxyReq", rewrite);
      proxy.on("proxyReqWs", rewrite);
    },
  };
}

export default defineConfig({
  root: "remote",
  plugins: [svelte(), tailwindcss()],
  clearScreen: false,
  build: {
    outDir: "../.remote-dist",
    emptyOutDir: true,
    cssCodeSplit: false,
    assetsInlineLimit: 100_000_000,
    rollupOptions: {
      output: {
        codeSplitting: false,
      },
    },
  },
  server: {
    port: 1421,
    strictPort: true,
    host: true,
    proxy: {
      "/phone": { ...proxyToBackend(), ws: true },
      "/apps": proxyToBackend(),
      "/pointer": proxyToBackend(),
      "/text": proxyToBackend(),
      "/manifest.webmanifest": proxyToBackend(),
      "/icons": proxyToBackend(),
    },
  },
});
