import { createConnection } from "node:net";
import type { Socket } from "node:net";

import type { RpcRequest, RpcResponse } from "@scanscan/api-types";

export interface CoreStatus {
  connected: boolean;
  socket: string;
  version?: string;
}

/** Minimal surface the HTTP layer needs from the core. */
export interface CoreApi {
  call<T>(method: string, params?: unknown): Promise<T>;
  status(): CoreStatus;
}

interface Pending {
  resolve: (value: unknown) => void;
  reject: (error: Error) => void;
}

/**
 * JSON-RPC 2.0 client for the Rust core daemon over a Unix-domain socket.
 *
 * A single persistent connection is used; requests are newline-delimited and
 * matched to responses by id. Reconnects lazily on the next call.
 */
export class CoreClient implements CoreApi {
  private socket?: Socket;
  private buffer = "";
  private pending = new Map<number, Pending>();
  private nextId = 1;
  private connected = false;

  constructor(private readonly socketPath: string) {}

  status(): CoreStatus {
    return { connected: this.connected, socket: this.socketPath };
  }

  private ensure(): Promise<void> {
    if (this.socket && this.connected) {
      return Promise.resolve();
    }
    return new Promise((resolve, reject) => {
      const socket = createConnection(this.socketPath);
      socket.setEncoding("utf8");
      socket.on("connect", () => {
        this.connected = true;
        this.socket = socket;
        resolve();
      });
      socket.on("data", (chunk: string) => this.onData(chunk));
      socket.on("error", (error: Error) => {
        this.connected = false;
        reject(error);
      });
      socket.on("close", () => {
        this.connected = false;
        this.failAll(new Error("core connection closed"));
      });
    });
  }

  private onData(chunk: string): void {
    this.buffer += chunk;
    let index = this.buffer.indexOf("\n");
    while (index >= 0) {
      const line = this.buffer.slice(0, index).trim();
      this.buffer = this.buffer.slice(index + 1);
      if (line) this.handleLine(line);
      index = this.buffer.indexOf("\n");
    }
  }

  private handleLine(line: string): void {
    let response: RpcResponse;
    try {
      response = JSON.parse(line) as RpcResponse;
    } catch {
      return;
    }
    const pending = this.pending.get(response.id);
    if (!pending) return;
    this.pending.delete(response.id);
    if (response.error) {
      pending.reject(new Error(response.error.message));
    } else {
      pending.resolve(response.result);
    }
  }

  async call<T>(method: string, params?: unknown): Promise<T> {
    await this.ensure();
    const socket = this.socket;
    if (!socket) throw new Error("core not connected");
    const id = this.nextId;
    this.nextId += 1;
    const request: RpcRequest = {
      jsonrpc: "2.0",
      id,
      method,
      ...(params === undefined ? {} : { params }),
    };
    return new Promise<T>((resolve, reject) => {
      this.pending.set(id, { resolve: (value) => resolve(value as T), reject });
      socket.write(`${JSON.stringify(request)}\n`, (error?: Error | null) => {
        if (error) {
          this.pending.delete(id);
          reject(error);
        }
      });
    });
  }

  private failAll(error: Error): void {
    for (const pending of this.pending.values()) {
      pending.reject(error);
    }
    this.pending.clear();
  }
}
