import {
  bridge,
  toBridgeError,
  type Identity,
  type SettingValue,
} from "@luma/plugin-sdk";

const surface = element<HTMLElement>("surface");
const greetingElement = element<HTMLElement>("greeting");
const statusElement = element<HTMLElement>("status");
const identityElement = element<HTMLElement>("identity");
const refreshButton = element<HTMLButtonElement>("refresh");
const closeButton = element<HTMLButtonElement>("close");

let identity: Identity | null = null;
let greeting = "Hello from Luma";
let busy = false;
let failed = false;

function element<T extends HTMLElement>(id: string): T {
  const node = document.getElementById(id);
  if (!node) throw new Error(`The starter markup is missing #${id}.`);
  return node as T;
}

function messageOf(cause: unknown): string {
  return toBridgeError(cause).message;
}

function greetingFrom(values: Record<string, SettingValue>): string {
  const value = values.greeting;
  return typeof value === "string" && value.trim() ? value : "Hello from Luma";
}

function render() {
  surface.dataset.state = failed ? "error" : busy ? "busy" : "ready";
  greetingElement.textContent = greeting;
  identityElement.textContent = identity
    ? `${identity.id} ${identity.version} · API ${identity.apiVersion}`
    : "";
  refreshButton.setAttribute("aria-busy", String(busy));
  refreshButton.textContent = busy ? "Refreshing…" : "Refresh greeting";
}

async function refreshGreeting() {
  if (busy) return;
  busy = true;
  render();
  try {
    const snapshot = await bridge.readSettings();
    greeting = greetingFrom(snapshot.values);
    failed = false;
    statusElement.textContent = "Settings refreshed from Luma.";
  } catch (cause) {
    failed = true;
    statusElement.textContent = `Could not read this package's settings: ${messageOf(cause)}`;
  } finally {
    busy = false;
    render();
    refreshButton.focus();
  }
}

function exit() {
  bridge.close().catch(cause => {
    statusElement.textContent = `Could not close this surface: ${messageOf(cause)}`;
  });
}

function focusables(): HTMLButtonElement[] {
  return [refreshButton, closeButton];
}

function moveFocus(step: number) {
  const items = focusables();
  const current = items.indexOf(document.activeElement as HTMLButtonElement);
  const next =
    current === -1
      ? step > 0
        ? 0
        : items.length - 1
      : (current + step + items.length) % items.length;
  items[next].focus();
}

window.addEventListener("keydown", event => {
  if (event.key === "Escape" || event.key === "BrowserBack") {
    event.preventDefault();
    exit();
    return;
  }
  const step =
    event.key === "ArrowRight" || event.key === "ArrowDown"
      ? 1
      : event.key === "ArrowLeft" || event.key === "ArrowUp"
        ? -1
        : 0;
  if (step === 0) return;
  event.preventDefault();
  moveFocus(step);
});

refreshButton.addEventListener("click", refreshGreeting);
closeButton.addEventListener("click", exit);
history.pushState({ surface: "starter" }, "");
window.addEventListener("popstate", exit);

async function initialize() {
  render();
  refreshButton.focus();

  const [identityResult, settingsResult] = await Promise.allSettled([
    bridge.identity(),
    bridge.readSettings(),
  ]);

  if (identityResult.status === "fulfilled") {
    identity = identityResult.value;
  } else {
    failed = true;
    statusElement.textContent = `Identity unavailable: ${messageOf(identityResult.reason)}`;
  }

  if (settingsResult.status === "fulfilled") {
    greeting = greetingFrom(settingsResult.value.values);
  } else {
    failed = true;
    statusElement.textContent = `Could not read this package's settings: ${messageOf(settingsResult.reason)}`;
  }

  render();
  refreshButton.focus();

  try {
    await bridge.ready();
    if (!failed) {
      statusElement.textContent = "Ready. Press Refresh greeting to read settings again.";
    }
  } catch (cause) {
    failed = true;
    statusElement.textContent = `This surface could not start: ${messageOf(cause)}`;
    render();
  }
}

initialize();
