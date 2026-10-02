import type { RpcRequest, RpcResponse } from "@scanscan/api-types";

export interface CoreStatus {
  connected: boolean;
  socket: string;
  version?: string;
}

/**
 * JSON-RPC 2.0 client for the Rust core daemon over a Unix-domain socket.
 *
 * Phase 0 only tracks connection state; Phase 1 adds the newline-delimited
 * request/response transport and the server-push notification streams.
 */
export class CoreClient {
  private requestId = 0;

  constructor(private readonly socket: string) {}

  status(): CoreStatus {
    return { connected: false, socket: this.socket };
  }

  nextId(): number {
    this.requestId += 1;
    return this.requestId;
  }

  buildRequest(method: string, params?: unknown): RpcRequest {
    return {
      jsonrpc: "2.0",
      id: this.nextId(),
      method,
      ...(params === undefined ? {} : { params }),
    };
  }

  static isSuccess(response: RpcResponse): boolean {
    return response.error === undefined || response.error === null;
  }
}
