const invoke = (command, args) => window.__TAURI_INTERNALS__.invoke(command, args);
const bridge = (op, args) =>
  args === undefined ? invoke("extension_bridge", { op }) : invoke("extension_bridge", { op, args });
const sleep = ms => new Promise(resolve => setTimeout(resolve, ms));

function messageOf(value) {
  if (typeof value === "string") return value;
  if (value && typeof value.message === "string") return value.message;
  if (value && typeof value.error === "string") return value.error;
  try {
    return JSON.stringify(value);
  } catch {
    return String(value);
  }
}

const steps = [];
async function step(name, run) {
  try {
    steps.push({ name, ok: true, value: await run() });
  } catch (cause) {
    steps.push({ name, ok: false, error: messageOf(cause), code: cause?.code ?? null });
  }
}

(async () => {
  const status = document.querySelector("#status");
  await step("ready", () => bridge("surface.ready"));
  await step("identity", () => bridge("identity"));
  let settings = null;
  await step("settings", async () => {
    settings = await bridge("settings.read");
    return settings;
  });
  await step("permission", () => bridge("playback.read"));
  await step("core", () => invoke("acceptance_core_probe"));
  const token = location.pathname.split("/")[1] || "";
  const mutated = (token[0] === "0" ? "1" : "0") + token.slice(1);
  const fallback =
    location.protocol +
    "//" +
    location.host.replace(token, mutated) +
    "/" +
    mutated +
    "/ui/index.html";
  const foreignTarget = settings?.values?.foreignOrigin
    ? settings.values.foreignOrigin + "/"
    : fallback;
  await step("foreign", async () => {
    const before = location.pathname;
    location.href = foreignTarget;
    await sleep(400);
    return {
      blocked: location.pathname === before,
      from: before,
      target: foreignTarget,
      path: location.pathname,
    };
  });
  await step("csp", async () => {
    let violation = null;
    const onViolation = event => {
      if (event.violatedDirective?.startsWith("connect-src")) {
        violation = event.blockedURI || event.violatedDirective;
      }
    };
    document.addEventListener("securitypolicyviolation", onViolation);
    let failed = false;
    try {
      await fetch("http://127.0.0.1:" + settings.values.probePort + "/leak", { mode: "no-cors" });
    } catch {
      failed = true;
    }
    await sleep(150);
    document.removeEventListener("securitypolicyviolation", onViolation);
    return { blocked: violation !== null, failed, violation, port: settings.values.probePort };
  });
  window.__lumaAcceptance = {
    done: true,
    steps,
    origin: location.origin,
    path: location.pathname,
    token,
  };
  status.textContent = "Probes complete";
})();
