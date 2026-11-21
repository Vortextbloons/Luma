#!/usr/bin/env node
import { spawn, spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { existsSync } from "node:fs";
import {
  cp,
  mkdir,
  readFile,
  readdir,
  rm,
  writeFile,
} from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import readline from "node:readline";
import { fileURLToPath } from "node:url";

const repoRoot = path.resolve(fileURLToPath(new URL("..", import.meta.url)));
const isWindows = process.platform === "win32";
const hostName = isWindows ? "luma-plugin-host.exe" : "luma-plugin-host";
const templateIgnored = new Set(["node_modules", ".git", ".luma-dev", "dist"]);

function fail(message, code = 1) {
  console.error(`luma-plugin: ${message}`);
  process.exit(code);
}

function usage() {
  console.log(`Luma plugin CLI

Usage:
  luma-plugin init <directory> [--template typescript|weather|<path>]
  luma-plugin validate <package-directory-or-zip> [--json]
  luma-plugin pack <package-directory> [--out <directory-or-zip>] [--json]
  luma-plugin dev [project-directory] [options]

Pack options:
  --out <path>       Directory receiving <id>-<version>.luma-extension.zip, or an exact .zip path.

Dev options:
  --out <path>       Built package directory. Defaults to plugin.dev.json, package.json lumaPlugin,
                     dist, or the project root when it contains luma-extension.json.
  --build <command>  Build command run in the project directory.
  --no-build         Use the existing built package output.
  --host <command>   Development host command. Defaults to "npm run tauri dev" in the Luma checkout.
  --luma <path>      Luma checkout used to build the host and run the default host command.
  --no-watch         Start the host without watching for changes.
  --no-launch        Watch and validate without starting the development host.

While dev is running:
  r  rebuild and reload now
  d  open DevTools for the active plugin surface
  s  print development session status
  q  stop the development host

Environment:
  LUMA_PLUGIN_HOST      Existing luma-plugin-host executable.
  LUMA_ROOT             Default Luma checkout for dev.
  LUMA_PLUGIN_DEV_SESSION, LUMA_PLUGIN_DEV_PACKAGE, LUMA_PLUGIN_DEV_ID
                        Low-level development host handshake used by the CLI.
`);
}

function parseArgs(argv) {
  const positional = [];
  const flags = {};
  for (let index = 0; index < argv.length; index += 1) {
    const argument = argv[index];
    if (!argument.startsWith("--")) {
      positional.push(argument);
      continue;
    }
    const equals = argument.indexOf("=");
    const name = equals === -1 ? argument.slice(2) : argument.slice(2, equals);
    if (equals !== -1) {
      flags[name] = argument.slice(equals + 1);
      continue;
    }
    const next = argv[index + 1];
    if (next !== undefined && !next.startsWith("--")) {
      flags[name] = next;
      index += 1;
    } else {
      flags[name] = true;
    }
  }
  return { positional, flags };
}

async function readJson(file) {
  try {
    return JSON.parse(await readFile(file, "utf8"));
  } catch {
    return null;
  }
}

function binary(name) {
  return isWindows ? `${name}.exe` : name;
}

function run(command, { cwd, env, capture = false } = {}) {
  return new Promise(resolve => {
    const child = spawn(command, {
      cwd,
      env: { ...process.env, ...env },
      shell: true,
      stdio: capture ? ["ignore", "pipe", "pipe"] : "inherit",
      windowsHide: true,
    });
    let stdout = "";
    let stderr = "";
    if (capture) {
      child.stdout.setEncoding("utf8");
      child.stderr.setEncoding("utf8");
      child.stdout.on("data", chunk => (stdout += chunk));
      child.stderr.on("data", chunk => (stderr += chunk));
    }
    child.on("error", error => resolve({ status: 1, error, stdout, stderr }));
    child.on("exit", code => resolve({ status: code ?? 1, stdout, stderr }));
  });
}

async function buildHost(lumaRoot) {
  const target = path.join(lumaRoot, "src-tauri", "target", "debug", hostName);
  if (process.env.LUMA_PLUGIN_HOST) {
    const configured = path.resolve(process.env.LUMA_PLUGIN_HOST);
    if (!existsSync(configured)) fail(`LUMA_PLUGIN_HOST does not exist: ${configured}`);
    return configured;
  }
  if (!existsSync(target)) {
    console.error("Building the Luma plugin host...");
    const result = await run(
      `cargo build --manifest-path "${path.join(lumaRoot, "src-tauri", "Cargo.toml")}" --bin luma-plugin-host`,
      { cwd: lumaRoot },
    );
    if (result.status !== 0) fail("The Luma plugin host could not be built.");
  }
  return target;
}

function hostPayload(stdout) {
  const lines = stdout.trim().split(/\r?\n/).filter(Boolean);
  for (let index = lines.length - 1; index >= 0; index -= 1) {
    try {
      return JSON.parse(lines[index]);
    } catch {}
  }
  return null;
}

async function invokeHost(lumaRoot, args) {
  const executable = await buildHost(lumaRoot);
  const result = spawnSync(executable, args, { encoding: "utf8", windowsHide: true });
  if (result.error) fail(`The Luma plugin host could not run: ${result.error.message}`);
  const payload = hostPayload(result.stdout ?? "");
  if (!payload) {
    fail(`The Luma plugin host returned no result.${result.stderr ? ` ${result.stderr.trim()}` : ""}`);
  }
  return payload;
}

function formatBytes(bytes) {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KiB`;
  return `${(bytes / (1024 * 1024)).toFixed(2)} MiB`;
}

function printDiagnostics(payload) {
  const diagnostics = Array.isArray(payload?.diagnostics) ? payload.diagnostics : [];
  if (!diagnostics.length) {
    console.error("luma-plugin: the package was rejected without a diagnostic.");
    return;
  }
  for (const diagnostic of diagnostics) {
    const location = diagnostic.path ? `${diagnostic.path}: ` : "";
    console.error(`error: ${location}${diagnostic.message}`);
  }
}

function describePackage(pkg) {
  const detail = [
    pkg.runtime,
    `${pkg.files} file${pkg.files === 1 ? "" : "s"}`,
    formatBytes(pkg.bytes),
  ];
  return `${pkg.name} ${pkg.version} (${pkg.id}) - ${detail.join(", ")}`;
}

async function commandValidate(input, flags, lumaRoot) {
  if (!input) fail("validate requires a package directory or ZIP path.");
  const payload = await invokeHost(lumaRoot, ["validate", path.resolve(input)]);
  if (flags.json) {
    console.log(JSON.stringify(payload));
  } else if (payload.ok) {
    console.log(`Valid ${describePackage(payload.package)}`);
  }
  if (!payload.ok) {
    if (!flags.json) printDiagnostics(payload);
    process.exit(1);
  }
}

function outputTarget(output) {
  return output ? path.resolve(output) : process.cwd();
}

async function commandPack(input, flags, lumaRoot) {
  if (!input) fail("pack requires a package directory.");
  const payload = await invokeHost(lumaRoot, [
    "pack",
    path.resolve(input),
    ...(flags.out ? ["--out", outputTarget(flags.out)] : []),
  ]);
  if (flags.json) {
    console.log(JSON.stringify(payload));
  } else if (payload.ok) {
    console.log(`Packed ${payload.output} - ${describePackage(payload.package)}`);
  }
  if (!payload.ok) {
    if (!flags.json) printDiagnostics(payload);
    process.exit(1);
  }
}

async function copyTemplate(source, target) {
  await mkdir(target, { recursive: true });
  await cp(source, target, {
    recursive: true,
    filter: file => {
      const relative = path.relative(source, file);
      if (!relative) return true;
      const segments = relative.split(path.sep);
      return !segments.some(segment => templateIgnored.has(segment) || segment.endsWith(".log"));
    },
  });
}

async function replacePlaceholders(target, values) {
  const pending = [target];
  while (pending.length) {
    const directory = pending.pop();
    for (const entry of await readdir(directory, { withFileTypes: true })) {
      const file = path.join(directory, entry.name);
      if (entry.isDirectory()) {
        pending.push(file);
        continue;
      }
      if (!/\.(json|jsonc|ts|js|mjs|md|html|css|ya?ml)$/i.test(entry.name)) continue;
      let text = await readFile(file, "utf8");
      let changed = false;
      for (const [key, value] of Object.entries(values)) {
        const token = `{{${key}}}`;
        if (text.includes(token)) {
          text = text.split(token).join(value);
          changed = true;
        }
      }
      if (changed) await writeFile(file, text);
    }
  }
}

async function directoryEmpty(directory) {
  try {
    return (await readdir(directory)).length === 0;
  } catch {
    return true;
  }
}

function slug(value) {
  return value
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "")
    .slice(0, 64) || "plugin";
}

async function commandInit(directory, flags) {
  if (!directory) fail("init requires a target directory.");
  const target = path.resolve(directory);
  const templateName = typeof flags.template === "string" ? flags.template : "typescript";
  const known = {
    typescript: path.join(repoRoot, "starters", "typescript"),
    weather: path.join(repoRoot, "examples", "plugins", "com.example.weather"),
  };
  const source = known[templateName] ?? path.resolve(templateName);
  if (!existsSync(source)) fail(`The template does not exist: ${source}`);
  if (existsSync(target) && !(await directoryEmpty(target))) {
    fail(`The target directory is not empty: ${target}`);
  }
  await copyTemplate(source, target);
  const name = path.basename(target);
  await replacePlaceholders(target, {
    id: `com.example.${slug(name)}`,
    name,
    version: "0.1.0",
  });
  if (templateName === "typescript") {
    const sdk = path.join(target, "vendor", "plugin-sdk");
    await mkdir(sdk, { recursive: true });
    for (const file of ["package.json", "index.js", "index.d.ts", "README.md"]) {
      await cp(path.join(repoRoot, "sdk", file), path.join(sdk, file));
    }
    await cp(path.join(repoRoot, "schemas"), path.join(target, "schemas"), { recursive: true });
    const packageFile = path.join(target, "package.json");
    const packageJson = await readJson(packageFile);
    packageJson.dependencies["@luma/plugin-sdk"] = "file:./vendor/plugin-sdk";
    await writeFile(packageFile, JSON.stringify(packageJson, null, 2) + "\n");
    const settingsFile = path.join(target, ".vscode", "settings.json");
    const settings = await readJson(settingsFile);
    settings["json.schemas"][0].url = "./schemas/luma-extension.schema.json";
    await writeFile(settingsFile, JSON.stringify(settings, null, 2) + "\n");
    const manifestFile = path.join(target, "luma-extension.json");
    const manifest = await readJson(manifestFile);
    manifest.id = `com.example.${slug(name)}`;
    manifest.name = name;
    await writeFile(manifestFile, JSON.stringify(manifest, null, 2) + "\n");
  }
  console.log(`Created ${name} from ${path.basename(source)} in ${target}`);
  console.log("Next: npm install, then luma-plugin dev (or press F5 in VS Code).");
}

async function projectConfig(project) {
  const local = await readJson(path.join(project, "plugin.dev.json"));
  if (local && typeof local === "object") return local;
  const pkg = await readJson(path.join(project, "package.json"));
  if (pkg?.lumaPlugin && typeof pkg.lumaPlugin === "object") return pkg.lumaPlugin;
  return {};
}

async function buildCommand(project, flags, config) {
  if (flags["no-build"]) return null;
  if (typeof flags.build === "string") return flags.build;
  if (typeof config.build === "string") return config.build;
  const pkg = await readJson(path.join(project, "package.json"));
  if (pkg?.scripts?.build) return "npm run build";
  return null;
}

async function packageOut(project, flags, config) {
  const configured =
    typeof flags.out === "string"
      ? flags.out
      : typeof config.out === "string"
        ? config.out
        : "dist";
  const resolved = path.resolve(project, configured);
  if (
    typeof flags.out !== "string" &&
    typeof config.out !== "string" &&
    !existsSync(resolved) &&
    existsSync(path.join(project, "luma-extension.json"))
  ) {
    return project;
  }
  return resolved;
}

function sessionRoot(project) {
  const hash = createHash("sha256").update(project).digest("hex").slice(0, 10);
  return path.join(
    tmpdir(),
    "luma-plugin-dev",
    `${slug(path.basename(project))}-${hash}-${Date.now().toString(36)}`,
  );
}

async function copySnapshot(source, target) {
  await rm(target, { recursive: true, force: true });
  await cp(source, target, { recursive: true });
}

function watchIgnored(relative) {
  const segments = relative.split(path.sep);
  return segments.some(
    segment => segment === "node_modules" || segment === ".git" || segment.endsWith(".log"),
  );
}

async function waitForHost(session, child) {
  const file = path.join(session, "host.json");
  const deadline = Date.now() + 900_000;
  let exited = null;
  child.once("exit", code => (exited = code ?? 1));
  while (Date.now() < deadline) {
    if (exited !== null) throw new Error(`The development host exited with code ${exited}.`);
    try {
      const host = JSON.parse(await readFile(file, "utf8"));
      await new Promise(resolve => setTimeout(resolve, 120));
      return host;
    } catch {
      await new Promise(resolve => setTimeout(resolve, 120));
    }
  }
  throw new Error("The development host did not start in time.");
}

async function api(host, route, options = {}) {
  const response = await fetch(`http://127.0.0.1:${host.port}${route}`, {
    ...options,
    headers: {
      authorization: `Bearer ${host.token}`,
      ...(options.body ? { "content-type": "application/json" } : {}),
      ...(options.headers ?? {}),
    },
    signal: AbortSignal.timeout(5000),
  });
  let payload = null;
  try {
    payload = await response.json();
  } catch {}
  return { status: response.status, payload };
}

async function printStatus(host) {
  const { status, payload } = await api(host, "/status");
  if (status !== 200 || !payload?.ok) {
    console.error(`luma-plugin: status failed with HTTP ${status}.`);
    return;
  }
  console.log(`Session: ${payload.session}`);
  console.log(`App data: ${payload.appDataDir}`);
  if (payload.plugin) {
    const state = payload.plugin.runtimeStatus?.state ?? (payload.plugin.enabled ? "enabled" : "disabled");
    console.log(
      `Plugin: ${payload.plugin.name} ${payload.plugin.version} (${payload.plugin.id}) - ${state}`,
    );
    if (payload.plugin.reviewRequired) {
      console.log("Access review is required in the development window: Settings > Plugins.");
    }
  } else {
    console.log("Plugin: not loaded");
  }
  if (payload.lastError) console.log(`Last error: ${payload.lastError}`);
}

async function terminate(child) {
  if (!child || child.exitCode !== null || child.signalCode !== null) return;
  if (isWindows && child.pid) {
    spawnSync("taskkill", ["/pid", String(child.pid), "/T", "/F"], {
      stdio: "ignore",
      windowsHide: true,
    });
    return;
  }
  child.kill("SIGTERM");
}

async function commandDev(projectInput, flags) {
  const project = path.resolve(projectInput ?? process.cwd());
  if (!existsSync(project)) fail(`The project directory does not exist: ${project}`);
  const lumaRoot = path.resolve(flags.luma ?? process.env.LUMA_ROOT ?? repoRoot);
  if (!existsSync(path.join(lumaRoot, "src-tauri", "Cargo.toml"))) {
    fail(`The Luma checkout was not found at ${lumaRoot}. Pass --luma.`);
  }
  const config = await projectConfig(project);
  const out = await packageOut(project, flags, config);
  const build = await buildCommand(project, flags, config);
  const session = sessionRoot(project);
  const snapshots = path.join(session, "snapshots");
  await mkdir(snapshots, { recursive: true });
  let host = null;
  let child = null;
  let watcher = null;
  let stopping = false;
  let building = false;
  let snapshotCount = 0;
  let activeSnapshot = null;
  let queuedReload = null;
  let timer = null;

  process.once("SIGINT", () => void shutdown(0));
  process.once("SIGTERM", () => void shutdown(0));

  async function validateAndSnapshot() {
    const validation = await invokeHost(lumaRoot, ["validate", out]);
    if (!validation.ok) {
      printDiagnostics(validation);
      return null;
    }
    if (flags["no-launch"]) return { package: validation.package };
    const snapshot = path.join(snapshots, String(++snapshotCount).padStart(4, "0"));
    await copySnapshot(out, snapshot);
    return { snapshot, package: validation.package };
  }

  async function reload(reason) {
    console.log(`\nReloading ${reason}...`);
    if (build) {
      const built = await run(build, { cwd: project });
      if (built.status !== 0) {
        console.error("Build failed. The last working snapshot is still active.");
        return false;
      }
    }
    const result = await validateAndSnapshot();
    if (!result) {
      console.error("Validation failed. The last working snapshot is still active.");
      return false;
    }
    if (!host) {
      activeSnapshot = result.snapshot ?? null;
      console.log(`Valid ${describePackage(result.package)}`);
      return true;
    }
    const response = await api(host, "/reload", {
      method: "POST",
      body: JSON.stringify({ path: result.snapshot }),
    }).catch(error => ({ status: 0, payload: { ok: false, diagnostics: [{ code: "runtime", message: error.message }] } }));
    if (response.status !== 200 || !response.payload?.ok) {
      if (response.status === 400) await discardSnapshot(result.snapshot);
      printDiagnostics(response.payload);
      console.error("Reload failed. The last working snapshot is still active.");
      return false;
    }
    await discardSnapshot(activeSnapshot);
    activeSnapshot = result.snapshot;
    const reloaded = response.payload.reload ?? {};
    console.log(reloaded.message ?? `Reloaded ${describePackage(result.package)}.`);
    const state = reloaded.plugin?.runtimeStatus?.state;
    if (state) console.log(`Runtime: ${state}`);
    if (reloaded.reviewRequired || reloaded.plugin?.reviewRequired) {
      console.log("Access review is required in the development window: Settings > Plugins.");
    }
    return true;
  }

  async function discardSnapshot(snapshot) {
    if (snapshot && path.dirname(path.resolve(snapshot)) === snapshots) {
      await rm(snapshot, { recursive: true, force: true }).catch(error => {
        console.error(`Could not remove superseded snapshot: ${error.message}`);
      });
    }
  }

  function queueReload(reason) {
    queuedReload = reason;
    if (building || stopping) return;
    if (timer) clearTimeout(timer);
    timer = setTimeout(async () => {
      if (stopping) return;
      const reason = queuedReload;
      queuedReload = null;
      building = true;
      try {
        await reload(reason);
      } catch (error) {
        console.error(`Reload failed: ${error.message}`);
      } finally {
        building = false;
        if (queuedReload) queueReload(queuedReload);
      }
    }, 250);
  }

  async function watch() {
    watcher?.close();
    try {
      watcher = (await import("node:fs")).watch(project, { recursive: true }, (_event, file) => {
        if (!file || stopping || watchIgnored(file)) return;
        const outputRelative = path.relative(out, path.resolve(project, file));
        if (build && out !== project &&
            (outputRelative === "" || (!path.isAbsolute(outputRelative) &&
              outputRelative !== ".." && !outputRelative.startsWith(`..${path.sep}`)))) return;
        queueReload(`change in ${file}`);
      });
      watcher.on("error", error => console.error(`luma-plugin: watcher: ${error.message}`));
    } catch (error) {
      console.error(`luma-plugin: cannot watch ${project}: ${error.message}`);
    }
  }

  async function shutdown(exitCode = 0) {
    if (stopping) return;
    stopping = true;
    watcher?.close();
    if (timer) clearTimeout(timer);
    if (host) await api(host, "/stop", { method: "POST" }).catch(() => {});
    if (child) {
      await new Promise(resolve => {
        if (child.exitCode !== null || child.signalCode !== null) return resolve();
        const timer = setTimeout(() => resolve(), 2000);
        child.once("exit", () => {
          clearTimeout(timer);
          resolve();
        });
      });
      await terminate(child);
    }
    console.log(`Development session stopped. Files remain in ${session}`);
    process.exit(exitCode);
  }

  const initial = await reload(build ? "initial build" : "existing package");
  if (!initial) {
    await shutdown(1);
    return;
  }

  if (flags["no-launch"]) {
    if (!flags["no-watch"]) {
      await watch();
      console.log("Watching the project. Press Ctrl+C to stop.");
    } else {
      console.log("Package is valid.");
    }
    return;
  }

  const executable = await buildHost(lumaRoot);
  if (!existsSync(executable)) fail(`The Luma plugin host does not exist: ${executable}`);
  const command =
    typeof flags.host === "string"
      ? flags.host
      : typeof config.host === "string"
        ? config.host
        : "npm run tauri dev";
  console.log(`Starting the development host: ${command}`);
  console.log("Waiting for the development session handshake...");
  child = spawn(command, {
    cwd: lumaRoot,
    env: {
      ...process.env,
      LUMA_PLUGIN_DEV_SESSION: session,
      LUMA_PLUGIN_DEV_PACKAGE: activeSnapshot,
    },
    shell: true,
    stdio: "inherit",
    windowsHide: true,
  });
  child.once("exit", code => {
    if (stopping) return;
    console.error(`\nThe development host exited with code ${code ?? 1}.`);
    void shutdown(code ?? 1);
  });
  try {
    host = await waitForHost(session, child);
  } catch (error) {
    console.error(`luma-plugin: ${error.message}`);
    await shutdown(1);
    return;
  }
  console.log(`Development host ready on port ${host.port}.`);
  await printStatus(host);
  if (!flags["no-watch"]) await watch();
  if (process.stdin.isTTY) {
    readline.createInterface({ input: process.stdin, terminal: false }).on("line", line => {
      const command = line.trim().toLowerCase();
      if (command === "r" || command === "reload") void queueReload("manual reload");
      else if (command === "d" || command === "devtools") {
        void api(host, "/devtools", { method: "POST" }).then(({ status, payload }) => {
          if (status !== 200 || !payload?.ok) printDiagnostics(payload);
          else console.log("Opened DevTools for the active plugin surface.");
        });
      } else if (command === "s" || command === "status") void printStatus(host);
      else if (command === "q" || command === "quit") void shutdown(0);
      else if (command) console.log("Commands: r reload, d DevTools, s status, q quit.");
    });
  }
}

async function main() {
  const { positional, flags } = parseArgs(process.argv.slice(2));
  const [command, ...rest] = positional;
  if (!command || flags.help || command === "help") {
    usage();
    return;
  }
  const lumaRoot = path.resolve(flags.luma ?? process.env.LUMA_ROOT ?? repoRoot);
  if (command === "init") return commandInit(rest[0], flags);
  if (command === "validate") return commandValidate(rest[0], flags, lumaRoot);
  if (command === "pack") return commandPack(rest[0], flags, lumaRoot);
  if (command === "dev") return commandDev(rest[0], flags);
  fail(`Unknown command: ${command}. Run luma-plugin --help.`);
}

main().catch(error => fail(error.stack ?? error.message));
