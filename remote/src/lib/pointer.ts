export interface PointerResult {
  response: {
    status: number;
    ok: boolean;
  };
  data: {
    released?: boolean;
  } | null;
}

export interface PointerControllerOptions {
  post: (body: { action: string; owner: string; x: number; y: number }) => Promise<PointerResult>;
  owner: string;
  onUnauthorized: () => void;
  onError: (message: string) => void;
  onReleased: () => void;
}

interface PointerAction {
  action: "start" | "move" | "stop";
  x: number;
  y: number;
}

export class PointerController {
  #released = false;
  #needStart = true;
  #pending: PointerAction | null = null;
  #inflight = false;
  #options: PointerControllerOptions;

  constructor(options: PointerControllerOptions) {
    this.#options = options;
  }

  start(): void {
    this.#released = false;
    this.#needStart = true;
    this.#queue("start", 0, 0);
  }

  move(x: number, y: number): void {
    if (this.#released) return;
    this.#queue("move", x, y);
  }

  stop(): void {
    this.#queue("stop", 0, 0);
  }

  release(): void {
    this.#released = true;
    this.#needStart = true;
    this.#pending = null;
  }

  #queue(action: PointerAction["action"], x: number, y: number): void {
    if (this.#released && action !== "stop") return;
    this.#pending = { action, x, y };
    if (!this.#inflight) void this.#flush();
  }

  async #flush(): Promise<void> {
    if (this.#inflight) return;
    this.#inflight = true;
    try {
      while (this.#pending) {
        const next = this.#pending;
        this.#pending = null;
        if (this.#released && next.action !== "stop") break;
        const action = this.#needStart && next.action !== "stop" ? "start" : next.action;
        const x = action === "stop" ? 0 : next.x;
        const y = action === "stop" ? 0 : next.y;
        let result: PointerResult;
        try {
          result = await this.#options.post({ action, owner: this.#options.owner, x, y });
        } catch (_) {
          this.release();
          this.#options.onError("Touchpad could not reach Luma.");
          break;
        }
        if (result.response.status === 401) {
          this.release();
          this.#options.onUnauthorized();
          break;
        }
        if (!result.response.ok) {
          this.release();
          this.#options.onError("Touchpad is unavailable right now.");
          break;
        }
        if (action === "start") this.#needStart = false;
        if (result.data?.released) {
          this.release();
          this.#options.onReleased();
          break;
        }
      }
    } finally {
      this.#inflight = false;
      if (this.#pending && !this.#released) void this.#flush();
    }
  }
}
