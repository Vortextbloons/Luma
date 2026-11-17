import type { ActivityItem, YouTubeFeedItem } from '../../../src/lib/types';
import type {
  AppsResponse,
  CommandResponse,
  PanelAction,
  PanelActionResponse,
  PanelsResponse,
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
  const controller = new AbortController();
  const abort = () => controller.abort(options.signal?.reason);
  if (options.signal?.aborted) abort();
  else options.signal?.addEventListener("abort", abort, { once: true });
  const timer = setTimeout(() => controller.abort(), path === "/phone/search" ? 30_000 : 15_000);
  try {
    const response = await fetch(path, {
      credentials: "same-origin",
      cache: "no-store",
      ...options,
      headers,
      signal: controller.signal,
    });
    let data: T | null = null;
    try {
      data = (await response.json()) as T;
    } catch (error) {
      if (controller.signal.aborted) throw error;
    }
    return { response, data };
  } finally {
    clearTimeout(timer);
    options.signal?.removeEventListener("abort", abort);
  }
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
  panels: () => request<PanelsResponse>("/phone/panels"),
  panelAction: (id: string, action: PanelAction) =>
    request<PanelActionResponse>("/phone/panels/action", post({ id, action })),
  settings: (key: string, value: string) =>
    request<{ accepted?: boolean }>("/phone/settings", post({ key, value })),
  layout: (order: string[]) => request<CommandResponse>("/phone/layout", post({ order })),
  playback: (position?: number) =>
    request<PlaybackResponse>("/phone/playback", position === undefined ? {} : post({ position })),
  search: (text: string) => request<{ videos: YouTubeFeedItem[]; history: ActivityItem[]; sportsHtml: string | null; streamEast: unknown; sportsUnavailable: boolean; youtubeUnavailable: boolean }>("/phone/search", post({ text })),
  openSearch: (target: unknown) => request<{ accepted?: boolean; error?: string }>("/phone/search/open", post(target)),
  text: (text: string) => request<{ accepted?: boolean; error?: string }>("/text", post({ text })),
  disconnect: () => request<unknown>("/phone/disconnect", { method: "POST" }),
  pointer: (action: string, owner: string, x: number, y: number) =>
    request<{ released?: boolean }>("/pointer", post({ action, owner, x, y })),
};
