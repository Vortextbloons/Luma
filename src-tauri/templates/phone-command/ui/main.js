const invoke = (command, args) => window.__TAURI_INTERNALS__.invoke(command, args);
const bridge = (op, args) =>
  args === undefined ? invoke("extension_bridge", { op }) : invoke("extension_bridge", { op, args });

const surface = document.querySelector("#surface");
const status = document.querySelector("#status");
const identity = document.querySelector("#identity");
const toggle = document.querySelector("#toggle");
const next = document.querySelector("#next");
const close = document.querySelector("#close");

const labels = {
  togglePlayback: "Play or pause",
  nextTrack: "Next",
};

let busy = false;

function messageOf(value) {
  if (typeof value === "string" && value.trim()) return value.trim();
  if (value && typeof value === "object" && typeof value.message === "string" && value.message.trim()) {
    return value.message.trim();
  }
  return "The host rejected the command.";
}

function render(state, message) {
  surface.dataset.state = state;
  status.textContent = message;
  toggle.setAttribute("aria-busy", String(busy));
  next.setAttribute("aria-busy", String(busy));
  toggle.setAttribute("aria-disabled", String(busy));
  next.setAttribute("aria-disabled", String(busy));
}

async function run(command) {
  if (busy) return;
  const label = labels[command];
  if (!label) {
    render("error", `Unknown command: ${command}`);
    return;
  }
  busy = true;
  render("busy", `${label}…`);
  try {
    await bridge(command === "togglePlayback" ? "playback.toggle" : "playback.next");
    render("ready", `${label} delivered.`);
  } catch (cause) {
    render("error", messageOf(cause));
  } finally {
    busy = false;
  }
}

function exit() {
  bridge("surface.close").catch(cause => render("error", messageOf(cause)));
}

toggle.addEventListener("click", () => void run("togglePlayback"));
next.addEventListener("click", () => void run("nextTrack"));
close.addEventListener("click", exit);
window.addEventListener("luma:command", event => {
  const command = event?.detail?.command;
  if (command) void run(command);
});
window.addEventListener("keydown", event => {
  if (event.key === "Escape" || event.key === "BrowserBack") {
    event.preventDefault();
    exit();
  }
});

async function initialize() {
  try {
    const info = await bridge("identity");
    identity.textContent = `${info.id} ${info.version} · API ${info.apiVersion}`;
  } catch (cause) {
    identity.textContent = `Identity unavailable: ${messageOf(cause)}`;
  }
  try {
    await bridge("surface.ready");
    render("ready", "Ready for a Home card or paired-phone command.");
    toggle.focus();
  } catch (cause) {
    render("error", `This package surface could not start: ${messageOf(cause)}`);
    return;
  }
  const pending = await bridge("commands.pending").catch(() => null);
  if (pending?.command) void run(pending.command);
}

initialize();
