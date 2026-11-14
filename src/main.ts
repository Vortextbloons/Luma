import "./app.css";
import { mount } from "svelte";
import App from "./App.svelte";
import { applyTheme, resolveThemeId, themeById } from "./lib/services/theme";

try {
  const saved: unknown = JSON.parse(localStorage.getItem("luma-v1") ?? "null");
  const data = saved && typeof saved === "object" ? saved as { theme?: unknown; accent?: unknown } : {};
  applyTheme(themeById(resolveThemeId(data.theme, data.accent)).tokens);
} catch {
  applyTheme(themeById(resolveThemeId(undefined)).tokens);
}

const target = document.getElementById("app");
if (!target) {
  throw new Error('Missing "#app" mount target in index.html');
}

export default mount(App, { target });
