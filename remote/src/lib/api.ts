import type {
  AppsResponse,
  CommandResponse,
  PlaybackResponse,
  StateResponse,
} from "./types";

export interface ApiResult<T> {
  response: Response;
  data: T | null;
}

export async function request<T>(path: string, options: RequestInit = {}): Promise<ApiResult<T>> {
  const headers = new Headers(options.headers);
  if (options.body) headers.set("Content-Type", "application/json");
  const response = await fetch(path, {
    credentials: "same-origin",
    cache: "no-store",
    ...options,
    headers,
  });
  let data: T | null = null;
  try {
    data = (await response.json()) as T;
  } catch (_) {}
  return { response, data };
}

const post = (body: unknown): RequestInit => ({
  method: "POST",
  body: JSON.stringify(body),
});

export const api = {
  state: () => request<StateResponse>("/phone/state"),
  pair: (code: string) => request<{ pcName?: string }>("/phone/pair", post({ code })),
  command: (command: string) => request<CommandResponse>("/phone/command", post({ command })),
  apps: () => request<AppsResponse>("/apps"),
  settings: (key: string, value: string) =>
    request<{ accepted?: boolean }>("/phone/settings", post({ key, value })),
  layout: (order: string[]) => request<CommandResponse>("/phone/layout", post({ order })),
  playback: (position?: number) =>
    request<PlaybackResponse>("/phone/playback", position === undefined ? {} : post({ position })),
  text: (text: string) => request<{ accepted?: boolean; error?: string }>("/text", post({ text })),
  disconnect: () => request<unknown>("/phone/disconnect", { method: "POST" }),
  pointer: (action: string, owner: string, x: number, y: number) =>
    request<{ released?: boolean }>("/pointer", post({ action, owner, x, y })),
};
