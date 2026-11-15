import { spawn, spawnSync } from 'node:child_process';
import { existsSync, mkdirSync, readdirSync, statSync, writeFileSync } from 'node:fs';
import { homedir, tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';

const sleep = ms => new Promise(done => setTimeout(done, ms));

function argument(name, fallback) {
  const index = process.argv.indexOf(`--${name}`);
  return index === -1 ? fallback : process.argv[index + 1];
}
const flag = name => process.argv.includes(`--${name}`);

const root = resolve(import.meta.dirname, '..', '..');
const exe = resolve(argument('exe', join(root, 'src-tauri', 'target', 'debug', 'luma.exe')));
const label = argument('label', 'dev');
const port = Number(argument('port', String(9200 + Math.floor(Math.random() * 600))));
const settleMs = Number(argument('settle', '8000'));
const idleMs = Number(argument('idle', '30000'));
const jsonPath = argument('json', '');
const skipViews = flag('no-views');
const devServerUrl = argument('dev-url', 'http://127.0.0.1:1420/');

if (!existsSync(exe)) {
  console.error(`Missing executable: ${exe}\nBuild it with: cargo build --manifest-path src-tauri/Cargo.toml`);
  process.exit(1);
}
const exeBuilt = statSync(exe).mtime;

const reachable = async (url, timeoutMs = 700) => {
  try {
    const reply = await fetch(url, { signal: AbortSignal.timeout(timeoutMs) });
    return reply.ok || reply.status < 500;
  } catch {
    return false;
  }
};

let devServer = null;
const devServerWasRunning = await reachable(devServerUrl);
if (!devServerWasRunning) {
  devServer = spawn(process.platform === 'win32' ? 'npm.cmd' : 'npm', ['run', 'dev'], { cwd: root, stdio: 'ignore', windowsHide: true });
  const deadline = Date.now() + 45000;
  let up = false;
  while (Date.now() < deadline) {
    if (await reachable(devServerUrl)) { up = true; break; }
    await sleep(250);
  }
  if (!up) {
    console.error(`Vite never started on ${devServerUrl}. Run "npm run dev" yourself and retry.`);
    devServer.kill();
    process.exit(1);
  }
}

const running = spawnSync('tasklist', ['/FI', 'IMAGENAME eq luma.exe', '/FO', 'CSV', '/NH'], { encoding: 'utf8' }).stdout ?? '';
if (running.includes('luma.exe') && !flag('replace')) {
  console.error('Luma is already running; the single-instance guard would attach to it instead of starting a measured copy.\nClose it first, or pass --replace to stop it.');
  devServer?.kill();
  process.exit(1);
}
if (running.includes('luma.exe')) {
  spawnSync('taskkill', ['/IM', 'luma.exe', '/T', '/F'], { encoding: 'utf8' });
  await sleep(1500);
}

const strayWebviews = spawnSync('powershell', ['-NoProfile', '-Command',
  `Get-CimInstance Win32_Process -Filter "Name='msedgewebview2.exe'" | Where-Object { $_.CommandLine -match 'com\\.luma\\.desktop' } | ForEach-Object { $_.ProcessId }`], { encoding: 'utf8' }).stdout ?? '';
for (const id of strayWebviews.split(/\r?\n/).filter(Boolean)) {
  spawnSync('taskkill', ['/PID', id.trim(), '/T', '/F'], { encoding: 'utf8' });
}
if (strayWebviews.trim()) await sleep(1000);

const directorySize = path => {
  if (!existsSync(path)) return { files: 0, bytes: 0 };
  const result = spawnSync('powershell', ['-NoProfile', '-Command',
    `$s = Get-ChildItem -LiteralPath '${path}' -Recurse -File -ErrorAction SilentlyContinue | Measure-Object Length -Sum; "$($s.Count) $($s.Sum)"`], { encoding: 'utf8' });
  const [files, bytes] = (result.stdout ?? '0 0').trim().split(/\s+/).map(Number);
  return { files: files || 0, bytes: bytes || 0 };
};

const appData = join(process.env.APPDATA ?? homedir(), 'com.luma.desktop');
const localAppData = join(process.env.LOCALAPPDATA ?? homedir(), 'com.luma.desktop');
const storageBefore = { appData: directorySize(appData), webviewCache: directorySize(localAppData) };

const samplerScript = join(tmpdir(), `luma-perf-sampler-${process.pid}.ps1`);
writeFileSync(samplerScript, String.raw`
param([int]$HostPid, [int]$IntervalMs)
$ErrorActionPreference = 'SilentlyContinue'
$timer = [Diagnostics.Stopwatch]::StartNew()
while ($timer.ElapsedMilliseconds -lt 600000) {
  $hostProcess = Get-Process -Id $HostPid
  if (-not $hostProcess) { break }
  $all = Get-CimInstance Win32_Process -Filter "Name='luma.exe' OR Name='msedgewebview2.exe'"
  $members = New-Object System.Collections.Generic.HashSet[int]
  [void]$members.Add($HostPid)
  $grew = $true
  while ($grew) {
    $grew = $false
    foreach ($candidate in $all) {
      if ($members.Contains([int]$candidate.ParentProcessId) -and -not $members.Contains([int]$candidate.ProcessId)) {
        [void]$members.Add([int]$candidate.ProcessId); $grew = $true
      }
    }
  }
  $rows = foreach ($id in $members) {
    $process = Get-Process -Id $id
    if (-not $process) { continue }
    $cim = $all | Where-Object { [int]$_.ProcessId -eq $id } | Select-Object -First 1
    $read = 0
    $write = 0
    $role = 'browser'
    if ($process.ProcessName -eq 'luma') { $role = 'host' }
    if ($cim) {
      $read = [double]$cim.ReadTransferCount
      $write = [double]$cim.WriteTransferCount
      if ($cim.CommandLine -match '--type=([a-z-]+)') { $role = $Matches[1] }
    }
    [pscustomobject]@{
      id = $id
      name = $process.ProcessName
      ws = [double]$process.WorkingSet64
      private = [double]$process.PrivateMemorySize64
      cpu = [double]$process.CPU * 1000
      threads = $process.Threads.Count
      handles = $process.HandleCount
      read = $read
      write = $write
      role = $role
    }
  }
  [pscustomobject]@{ t = [DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds(); ms = $timer.ElapsedMilliseconds; procs = @($rows) } | ConvertTo-Json -Compress -Depth 4
  Start-Sleep -Milliseconds $IntervalMs
}
`, 'utf8');

const started = Date.now();
const app = spawn(exe, [], {
  detached: false,
  stdio: 'ignore',
  env: {
    ...process.env,
    WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${port} --remote-allow-origins=*`
  }
});
const hostPid = app.pid;
const sampler = spawn('powershell', ['-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', samplerScript, String(hostPid), '1000'], { stdio: ['ignore', 'pipe', 'ignore'] });
const samples = [];
let buffer = '';
sampler.stdout.setEncoding('utf8');
sampler.stdout.on('data', chunk => {
  buffer += chunk;
  const lines = buffer.split(/\r?\n/);
  buffer = lines.pop() ?? '';
  for (const line of lines) {
    if (!line.trim().startsWith('{')) continue;
    try { samples.push(JSON.parse(line)); } catch {}
  }
});

async function target() {
  const deadline = Date.now() + 40000;
  while (Date.now() < deadline) {
    try {
      const list = await (await fetch(`http://127.0.0.1:${port}/json/list`)).json();
      const page = list.find(entry => entry.type === 'page' && entry.webSocketDebuggerUrl);
      if (page) return page;
    } catch {}
    await sleep(100);
  }
  throw new Error('WebView2 DevTools endpoint never became reachable');
}

const page = await target();
const connectedAt = Date.now() - started;
const socket = new WebSocket(page.webSocketDebuggerUrl);
await new Promise((done, fail) => { socket.onopen = done; socket.onerror = fail; });
let messageId = 0;
const pending = new Map();
socket.onmessage = event => {
  const message = JSON.parse(event.data);
  const resolvePending = pending.get(message.id);
  if (resolvePending) { pending.delete(message.id); resolvePending(message); }
};
const send = (method, params = {}) => new Promise(done => {
  const id = ++messageId;
  pending.set(id, done);
  socket.send(JSON.stringify({ id, method, params }));
});
const evaluate = async expression => {
  const reply = await send('Runtime.evaluate', { expression, returnByValue: true, awaitPromise: true });
  return reply.result?.result?.value;
};

let readyAt = 0;
while (Date.now() - started < 40000) {
  const ready = await evaluate(`document.readyState === 'complete' && !!document.querySelector('main, dialog[open]')`);
  if (ready) { readyAt = Date.now() - started; break; }
  await sleep(100);
}
await send('Page.bringToFront');
const url = await evaluate('location.href');
const visibility = await evaluate('document.visibilityState');
const devFrontend = url.includes(':1420');
if (devFrontend && flag('strict')) {
  socket.close();
  sampler.kill();
  spawnSync('taskkill', ['/PID', String(hostPid), '/T', '/F'], { encoding: 'utf8' });
  devServer?.kill();
  console.error(`The executable loaded the Vite dev server (${url}) instead of bundled assets.\nUse --exe src-tauri/target/release/luma.exe for production numbers, or drop --strict.`);
  process.exit(1);
}
const newestSource = [join(root, 'src'), join(root, 'src-tauri', 'src')]
  .flatMap(directory => readdirSync(directory, { recursive: true, withFileTypes: true })
    .filter(entry => entry.isFile())
    .map(entry => statSync(join(entry.parentPath ?? entry.path, entry.name)).mtimeMs))
  .reduce((newest, time) => Math.max(newest, time), 0);
const staleExe = newestSource > exeBuilt.getTime();

for (let attempt = 0; attempt < 4; attempt++) {
  if (!(await evaluate('!!document.querySelector("dialog[open]")'))) break;
  await evaluate(`window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }))`);
  await sleep(400);
}

const total = (sample, key) => sample.procs.reduce((sum, process) => sum + process[key], 0);
const snapshot = async () => {
  const live = (await evaluate(`({
    nodes: document.querySelectorAll('*').length,
    active: document.activeElement ? (document.activeElement.id || document.activeElement.className || document.activeElement.tagName) : ''
  })`)) ?? {};
  const heap = await send('Runtime.getHeapUsage');
  const latest = samples.at(-1);
  return {
    ...live,
    heap: Math.round(heap.result?.usedSize ?? 0),
    ws: latest ? total(latest, 'ws') : 0,
    private: latest ? total(latest, 'private') : 0,
    processes: latest ? latest.procs.length : 0
  };
};

const clickNav = name => `(() => {
  const scope = document.querySelector('.nav-pill') ?? document;
  const label = ${JSON.stringify(name.toLowerCase())};
  const button = [...scope.querySelectorAll('button, [role="button"], a')]
    .find(element => (element.textContent || '').trim().toLowerCase() === label)
    ?? [...document.querySelectorAll('button, [role="button"], a')]
      .find(element => (element.getAttribute('aria-label') || '').toLowerCase() === 'open ' + label);
  if (!button) return false;
  button.click();
  return true;
})()`;

const view = async name => {
  const clicked = await evaluate(clickNav(name));
  if (!clicked) return null;
  await sleep(2500);
  const readings = [];
  for (let index = 0; index < 4; index++) { readings.push(await snapshot()); await sleep(400); }
  const settled = readings.slice(-2);
  return {
    view: name,
    ws: Math.round(settled.reduce((sum, item) => sum + item.ws, 0) / settled.length),
    private: Math.round(settled.reduce((sum, item) => sum + item.private, 0) / settled.length),
    heap: Math.round(settled.reduce((sum, item) => sum + item.heap, 0) / settled.length),
    nodes: Math.round(settled.reduce((sum, item) => sum + item.nodes, 0) / settled.length)
  };
};

await sleep(settleMs);
const idleStart = Date.now();
const idleStartSample = samples.at(-1);
await sleep(idleMs);
const idleEndSample = samples.at(-1);
const idleElapsed = Date.now() - idleStart;

const views = [];
if (!skipViews) {
  await evaluate(`document.querySelector('[aria-label="Open settings"]')?.click()`);
  await sleep(2500);
  const settingsView = await snapshot();
  views.push({ view: 'Settings', ws: Math.round(settingsView.ws), private: Math.round(settingsView.private), heap: settingsView.heap, nodes: settingsView.nodes });
  await evaluate(`window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }))`);
  await sleep(1500);
  for (const name of ['Home', 'Watch', 'Sports', 'Library', 'Music', 'Apps']) {
    const reading = await view(name);
    if (reading) views.push(reading);
  }
  await view('Home');
}

const roles = new Map();

socket.close();
sampler.kill();
spawnSync('taskkill', ['/PID', String(hostPid), '/T', '/F'], { encoding: 'utf8' });
if (devServer) spawnSync('taskkill', ['/PID', String(devServer.pid), '/T', '/F'], { encoding: 'utf8' });
await sleep(1200);

const storageAfter = { appData: directorySize(appData), webviewCache: directorySize(localAppData) };

const plateau = samples.filter(sample => sample.ms >= settleMs);
for (const process of plateau.at(-1)?.procs ?? []) {
  roles.set(process.role, (roles.get(process.role) ?? 0) + 1);
}
const cpuDelta = idleEndSample && idleStartSample
  ? ((total(idleEndSample, 'cpu') - total(idleStartSample, 'cpu')) / 1000) / (idleElapsed / 1000)
  : 0;
const ioDelta = idleEndSample && idleStartSample
  ? total(idleEndSample, 'read') + total(idleEndSample, 'write') - total(idleStartSample, 'read') - total(idleStartSample, 'write')
  : 0;
const median = values => {
  const sorted = [...values].sort((a, b) => a - b);
  return sorted.length ? sorted[Math.floor(sorted.length / 2)] : 0;
};
const peak = values => (values.length ? Math.max(...values) : 0);
const processes = (idleEndSample?.procs ?? []).map(process => {
  const before = idleStartSample?.procs.find(entry => entry.id === process.id);
  return {
    pid: process.id,
    role: process.role,
    ws: Math.round(process.ws),
    private: Math.round(process.private),
    threads: process.threads,
    handles: process.handles,
    cpu: Math.round(process.cpu - (before?.cpu ?? process.cpu)),
    disk: Math.round(((process.read - (before?.read ?? process.read)) + (process.write - (before?.write ?? process.write))) / 1024)
  };
});
const exeBytes = spawnSync('powershell', ['-NoProfile', '-Command', `(Get-Item -LiteralPath '${exe}').Length`], { encoding: 'utf8' }).stdout?.trim();
const cores = Number(spawnSync('powershell', ['-NoProfile', '-Command', '(Get-CimInstance Win32_Processor | Measure-Object NumberOfLogicalProcessors -Sum).Sum'], { encoding: 'utf8' }).stdout?.trim()) || 1;

const report = {
  label,
  exe,
  exeBytes: Number(exeBytes) || 0,
  exeBuilt: exeBuilt.toISOString(),
  staleExe,
  url,
  devFrontend,
  devServerStarted: !devServerWasRunning,
  visibility,
  logicalCores: cores,
  startup: { devtoolsMs: connectedAt, interactiveMs: readyAt },
  idle: {
    sampledMs: idleElapsed,
    cpuOfOneCore: Number(cpuDelta.toFixed(2)),
    cpuOfMachine: Number((cpuDelta / cores).toFixed(2)),
    diskBytes: Math.round(ioDelta)
  },
  memory: {
    wsMedian: Math.round(median(plateau.map(sample => total(sample, 'ws')))),
    wsPeak: Math.round(peak(plateau.map(sample => total(sample, 'ws')))),
    privateMedian: Math.round(median(plateau.map(sample => total(sample, 'private')))),
    privatePeak: Math.round(peak(plateau.map(sample => total(sample, 'private')))),
    processes: plateau.at(-1)?.procs.length ?? 0,
    threads: plateau.at(-1) ? total(plateau.at(-1), 'threads') : 0,
    handles: plateau.at(-1) ? total(plateau.at(-1), 'handles') : 0,
    appProcessWs: plateau.at(-1) ? plateau.at(-1).procs.filter(process => process.role === 'host').reduce((sum, process) => sum + process.ws, 0) : 0
  },
  webviewRoles: Object.fromEntries(roles),
  processes,
  storage: {
    executable: Number(exeBytes) || 0,
    appData: storageAfter.appData,
    webviewCache: storageAfter.webviewCache,
    appDataDelta: storageAfter.appData.bytes - storageBefore.appData.bytes,
    webviewCacheDelta: storageAfter.webviewCache.bytes - storageBefore.webviewCache.bytes
  },
  views
};

if (jsonPath) {
  mkdirSync(dirname(resolve(jsonPath)), { recursive: true });
  writeFileSync(resolve(jsonPath), JSON.stringify(report, null, 2), 'utf8');
}

const mb = bytes => `${(bytes / 1048576).toFixed(1)} MB`;
console.log(`\n# Luma perf — ${label}\n`);
console.log(`- exe: \`${exe}\` (${mb(report.exeBytes)}, built ${new Date(report.exeBuilt).toLocaleString()})`);
console.log(`- frontend: ${url} ${devFrontend ? '(dev server: unminified modules + HMR)' : '(bundled)'}`);
if (staleExe) console.log(`- warning: sources are newer than the executable; rebuild before trusting these numbers`);
if (report.devServerStarted) console.log(`- started a temporary Vite dev server for this run`);
console.log(`- startup: DevTools reachable ${connectedAt} ms, app interactive ${readyAt} ms`);
console.log(`- idle (${(idleElapsed / 1000).toFixed(0)}s): ${report.idle.cpuOfOneCore.toFixed(2)}% of one core (${report.idle.cpuOfMachine.toFixed(2)}% of ${cores} cores), disk ${(report.idle.diskBytes / 1024).toFixed(0)} KB`);
console.log(`- memory: ${mb(report.memory.wsMedian)} working set (peak ${mb(report.memory.wsPeak)}), ${mb(report.memory.privateMedian)} private commit`);
console.log(`- ${report.memory.processes} processes, ${report.memory.threads} threads, ${report.memory.handles} handles`);
console.log(`- webview roles: ${Object.entries(report.webviewRoles).map(([role, count]) => `${role}×${count}`).join(', ') || 'n/a'}`);
console.log(`- storage: app data ${mb(report.storage.appData.bytes)} (${report.storage.appData.files} files), webview cache ${mb(report.storage.webviewCache.bytes)} (${report.storage.webviewCache.files} files)`);
console.log(`- per launch: app data +${(report.storage.appDataDelta / 1024).toFixed(0)} KB, webview cache +${(report.storage.webviewCacheDelta / 1024).toFixed(0)} KB\n`);
if (views.length) {
  console.log('| View | Working set | Private | JS heap | DOM nodes |');
  console.log('| --- | --- | --- | --- | --- |');
  for (const entry of views) console.log(`| ${entry.view} | ${mb(entry.ws)} | ${mb(entry.private)} | ${(entry.heap / 1048576).toFixed(1)} MB | ${entry.nodes} |`);
}
console.log('\n| Process | PID | Working set | Private | Threads | Handles | CPU while idle | Disk while idle |');
console.log('| --- | --- | --- | --- | --- | --- | --- | --- |');
for (const process of processes.sort((a, b) => b.private - a.private)) {
  console.log(`| ${process.role} | ${process.pid} | ${mb(process.ws)} | ${mb(process.private)} | ${process.threads} | ${process.handles} | ${process.cpu} ms (${((process.cpu / idleElapsed) * 100).toFixed(2)}% of 1 core) | ${(process.disk / 1024).toFixed(1)} MB |`);
}
