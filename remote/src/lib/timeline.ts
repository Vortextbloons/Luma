export interface TimelineResult {
  response: {
    ok: boolean;
    status: number;
  };
  data: {
    timeline?: {
      position?: number;
      duration?: number;
      error?: string;
    } | null;
    error?: string;
  } | null;
}

export type TimelineRequest = (position?: number) => Promise<TimelineResult>;

export function formatClock(value: number): string {
  const safe = Number.isFinite(value) ? Math.max(0, value) : 0;
  return `${Math.floor(safe / 60)}:${String(Math.floor(safe % 60)).padStart(2, "0")}`;
}

export class TimelineModel {
  position = 0;
  duration = 0;
  dragging = false;
  #busy = false;
  #seek: number | null = null;
  #request: TimelineRequest;
  #active: () => boolean;
  #onChange: () => void;
  #onNotice?: (message: string) => void;

  constructor(
    request: TimelineRequest,
    active: () => boolean,
    onChange: () => void,
    onNotice?: (message: string) => void,
  ) {
    this.#request = request;
    this.#active = active;
    this.#onChange = onChange;
    this.#onNotice = onNotice;
  }

  get available(): boolean {
    return this.duration > 0;
  }

  beginDrag(): void {
    this.dragging = true;
  }

  preview(value: number): void {
    this.dragging = true;
    this.position = value;
    this.#onChange();
  }

  endDrag(value: number): void {
    this.dragging = false;
    this.position = value;
    this.#seek = value;
    this.#onChange();
    void this.refresh();
  }

  cancelDrag(): void {
    this.dragging = false;
  }

  async refresh(): Promise<void> {
    if (this.#busy || !this.#active()) return;
    const seek = this.#seek;
    this.#seek = null;
    this.#busy = true;
    try {
      const { response, data } = await this.#request(seek === null ? undefined : seek);
      if (!response.ok || data?.error) {
        throw new Error(data?.error || "Playback position is unavailable.");
      }
      if (this.dragging || this.#seek !== null) return;
      const timeline = data?.timeline;
      if (timeline?.error) throw new Error(timeline.error);
      if (seek !== null && !timeline) {
        throw new Error("This player is not exposing playback controls.");
      }
      const duration = typeof timeline?.duration === "number" && Number.isFinite(timeline.duration)
        ? Math.max(0, timeline.duration)
        : 0;
      this.duration = duration;
      if (duration > 0) {
        const position = typeof timeline?.position === "number" && Number.isFinite(timeline.position)
          ? timeline.position
          : 0;
        this.position = seek === null ? Math.max(0, Math.min(duration, position)) : seek;
      }
      this.#onChange();
    } catch (error) {
      this.duration = 0;
      this.#onChange();
      if (seek !== null) {
        this.#onNotice?.(error instanceof Error ? error.message : "Playback position is unavailable.");
      }
    } finally {
      this.#busy = false;
      if (this.#seek !== null) void this.refresh();
    }
  }
}
