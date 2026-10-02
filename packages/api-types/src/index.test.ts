import { describe, expect, test } from "bun:test";

import { isTerminalState, PROTOCOL_VERSION, RPC_CODES } from "./index.js";

describe("api-types", () => {
  test("protocol version is exported", () => {
    expect(PROTOCOL_VERSION).toBe(1);
  });

  test("terminal states are recognised", () => {
    expect(isTerminalState("completed")).toBe(true);
    expect(isTerminalState("failed")).toBe(true);
    expect(isTerminalState("cancelled")).toBe(true);
    expect(isTerminalState("running")).toBe(false);
    expect(isTerminalState("queued")).toBe(false);
  });

  test("rpc codes match the Rust core", () => {
    expect(RPC_CODES.notFound).toBe(-32001);
    expect(RPC_CODES.conflict).toBe(-32002);
  });
});
