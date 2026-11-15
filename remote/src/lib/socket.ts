import type { SocketReply } from "./types";

const OPEN = 1;

export interface SocketLike {
  readyState: number;
  send(data: string): void;
  close(): void;
  onopen: ((event: Event) => void) | null;
  onmessage: ((event: MessageEvent) => void) | null;
  onclose: ((event: CloseEvent) => void) | null;
}

export interface CommandSocketOptions {
  create: () => SocketLike;
  onOpen?: () => void;
  onMessage?: (reply: SocketReply) => void;
  onClose?: () => void;
  timeout?: number;
  maxPending?: number;
}

export interface CommandSocket {
  connect(): void;
  close(): void;
  isOpen(): boolean;
  hasPending(): boolean;
  request(command?: string): Promise<SocketReply>;
}

interface Pending {
  resolve: (reply: SocketReply) => void;
  reject: (error: Error) => void;
  timer: ReturnType<typeof setTimeout>;
}

export function createCommandSocket(options: CommandSocketOptions): CommandSocket {
  const timeout = options.timeout ?? 5000;
  const maxPending = options.maxPending ?? 32;
  let socket: SocketLike | null = null;
  let sequence = 0;
  const pending = new Map<number, Pending>();

  function rejectPending(message: string): void {
    for (const entry of pending.values()) {
      clearTimeout(entry.timer);
      entry.reject(new Error(message));
    }
    pending.clear();
  }

  function connect(): void {
    if (socket) return;
    const connection = options.create();
    socket = connection;
    connection.onopen = () => options.onOpen?.();
    connection.onmessage = event => {
      let reply: SocketReply;
      try {
        reply = JSON.parse(event.data) as SocketReply;
      } catch (_) {
        return;
      }
      if (typeof reply.id === "number") {
        const entry = pending.get(reply.id);
        if (entry) {
          clearTimeout(entry.timer);
          pending.delete(reply.id);
          entry.resolve(reply);
        }
      }
      options.onMessage?.(reply);
    };
    connection.onclose = () => {
      if (socket !== connection) return;
      socket = null;
      rejectPending("Connection interrupted. Press again to retry.");
      options.onClose?.();
    };
  }

  function close(): void {
    socket?.close();
  }

  function isOpen(): boolean {
    return Boolean(socket && socket.readyState === OPEN);
  }

  function hasPending(): boolean {
    return pending.size > 0;
  }

  function request(command?: string): Promise<SocketReply> {
    return new Promise((resolve, reject) => {
      if (pending.size >= maxPending) {
        reject(new Error("Wait for your previous input to finish."));
        return;
      }
      if (!socket || socket.readyState !== OPEN) {
        reject(new Error("Connection interrupted. Press again to retry."));
        return;
      }
      const id = ++sequence;
      const timer = setTimeout(() => {
        pending.delete(id);
        socket?.close();
        reject(new Error("Luma did not respond. Check your connection."));
      }, timeout);
      pending.set(id, { resolve, reject, timer });
      socket.send(JSON.stringify({ id, ...(command ? { command } : {}) }));
    });
  }

  return { connect, close, isOpen, hasPending, request };
}
