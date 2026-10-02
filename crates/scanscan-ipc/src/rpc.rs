use serde::{Deserialize, Serialize};
use serde_json::Value;

/// The only JSON-RPC version scanscan speaks.
pub const JSONRPC_VERSION: &str = "2.0";

/// Standard JSON-RPC 2.0 error codes plus scanscan-specific ones.
pub mod codes {
    pub const PARSE_ERROR: i64 = -32700;
    pub const INVALID_REQUEST: i64 = -32600;
    pub const METHOD_NOT_FOUND: i64 = -32601;
    pub const INVALID_PARAMS: i64 = -32602;
    pub const INTERNAL_ERROR: i64 = -32603;
    /// Requested snapshot/node does not exist.
    pub const NOT_FOUND: i64 = -32001;
    /// The core is busy or the operation conflicts with an active scan.
    pub const CONFLICT: i64 = -32002;
    /// The index format is unsupported by this core build.
    pub const UNSUPPORTED_FORMAT: i64 = -32003;
}

/// A JSON-RPC 2.0 request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RpcRequest {
    pub jsonrpc: String,
    pub id: u64,
    pub method: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

impl RpcRequest {
    pub fn new(id: u64, method: impl Into<String>, params: Option<Value>) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.to_string(),
            id,
            method: method.into(),
            params,
        }
    }
}

/// A JSON-RPC 2.0 response: exactly one of `result` / `error` is set.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RpcResponse {
    pub jsonrpc: String,
    pub id: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<RpcError>,
}

impl RpcResponse {
    pub fn success(id: u64, result: Value) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.to_string(),
            id,
            result: Some(result),
            error: None,
        }
    }

    pub fn failure(id: u64, error: RpcError) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.to_string(),
            id,
            result: None,
            error: Some(error),
        }
    }
}

/// A JSON-RPC 2.0 error object.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RpcError {
    pub code: i64,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl RpcError {
    pub fn new(code: i64, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            data: None,
        }
    }

    pub fn with_data(mut self, data: Value) -> Self {
        self.data = Some(data);
        self
    }
}

/// A server-push JSON-RPC 2.0 notification (no `id`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RpcNotification {
    pub jsonrpc: String,
    pub method: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

impl RpcNotification {
    pub fn new(method: impl Into<String>, params: Option<Value>) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.to_string(),
            method: method.into(),
            params,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn request_roundtrips() {
        let req = RpcRequest::new(1, "core.health", Some(json!({})));
        let text = serde_json::to_string(&req).unwrap();
        let back: RpcRequest = serde_json::from_str(&text).unwrap();
        assert_eq!(req, back);
    }

    #[test]
    fn success_response_omits_error() {
        let resp = RpcResponse::success(7, json!({"ok": true}));
        let value = serde_json::to_value(&resp).unwrap();
        assert!(value.get("error").is_none());
        assert_eq!(value["id"], 7);
    }

    #[test]
    fn failure_response_omits_result() {
        let resp = RpcResponse::failure(7, RpcError::new(codes::NOT_FOUND, "missing"));
        let value = serde_json::to_value(&resp).unwrap();
        assert!(value.get("result").is_none());
        assert_eq!(value["error"]["code"], codes::NOT_FOUND);
    }
}
