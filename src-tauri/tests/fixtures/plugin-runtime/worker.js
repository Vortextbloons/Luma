const invoke = (command, args) => window.__TAURI_INTERNALS__.invoke(command, args);

window.__lumaService = { ready: false, beat: 0 };

setInterval(() => {
  window.__lumaService.beat += 1;
  const state = document.querySelector("#state");
  if (state) state.textContent = "Beat " + window.__lumaService.beat;
}, 1000);

(async () => {
  try {
    await invoke("extension_bridge", { op: "surface.ready" });
    window.__lumaService.ready = true;
  } catch (cause) {
    window.__lumaService.error = String(cause?.message ?? cause);
  }
})();
