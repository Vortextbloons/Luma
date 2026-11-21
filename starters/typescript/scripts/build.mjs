import { build } from "esbuild";
import { copyFile, mkdir, rm } from "node:fs/promises";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

const root = fileURLToPath(new URL("..", import.meta.url));
const dist = join(root, "dist");

await rm(dist, { recursive: true, force: true });
await mkdir(join(dist, "ui"), { recursive: true });

await build({
  entryPoints: [join(root, "src/main.ts")],
  outfile: join(dist, "ui/main.js"),
  bundle: true,
  format: "esm",
  target: ["chrome120"],
  logLevel: "info",
});

await copyFile(join(root, "luma-extension.json"), join(dist, "luma-extension.json"));
await copyFile(join(root, "src/index.html"), join(dist, "ui/index.html"));
await copyFile(join(root, "src/style.css"), join(dist, "ui/style.css"));
