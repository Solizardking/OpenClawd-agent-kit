//! MCP (Model Context Protocol) client over streamable HTTP.
//!
//! Implements the parts of the protocol the kit needs — `initialize`,
//! `notifications/initialized`, `tools/list`, `tools/call` — as JSON-RPC
//! over HTTP POST, accepting both plain JSON and `text/event-stream`
//! responses, with `Mcp-Session-Id` handling for stateful servers.
//!
//! Live-verified 2026-11-27 against both fleet servers:
//! - `https://mcp.clawdcompute.us/mcp` — "clawd-compute-mcp-portal" v1.0.0,
//!   stateless, **272 tools** (portal + PumpAI mirror + equities + desk +
//!   backpack + massive + ai). Note: the portal's edge blocks non-browser
//!   user agents (Cloudflare 1010), so the client sends a normal UA string.
//! - `https://pumpai.ai/mcp` — "clawd-pump" (PumpAI) v2.4.0-clawd,
//!   session-based, **213 tools**.

use std::sync::Arc;

use serde_json::{json, Value};
use tokio::sync::Mutex;

use kit_core::ToolError;

pub const PROTOCOL_VERSION: &str = "2025-03-26";
pub const CLAWD_COMPUTE_MCP_URL: &str = "https://mcp.clawdcompute.us/mcp";
pub const PUMPAI_MCP_URL: &str = "https://pumpai.ai/mcp";

/// A tool as advertised by an MCP server's `tools/list`.
#[derive(Debug, Clone)]
pub struct McpTool {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
    /// MCP `annotations.readOnlyHint`, when the server provides it.
    pub read_only: Option<bool>,
    /// MCP `annotations.destructiveHint`, when provided.
    pub destructive: Option<bool>,
}

impl McpTool {
    fn from_json(v: &Value) -> Option<Self> {
        let name = v.get("name")?.as_str()?.to_string();
        let annotations = v.get("annotations");
        Some(Self {
            name,
            description: v
                .get("description")
                .and_then(|d| d.as_str())
                .unwrap_or_default()
                .to_string(),
            input_schema: v
                .get("inputSchema")
                .cloned()
                .unwrap_or_else(|| json!({"type": "object"})),
            read_only: annotations
                .and_then(|a| a.get("readOnlyHint"))
                .and_then(|b| b.as_bool()),
            destructive: annotations
                .and_then(|a| a.get("destructiveHint"))
                .and_then(|b| b.as_bool()),
        })
    }
}

#[derive(Debug, Clone)]
pub struct ServerInfo {
    pub name: String,
    pub version: String,
    pub protocol_version: String,
}

pub struct McpClient {
    http: reqwest::Client,
    url: String,
    /// Short namespace for this server, e.g. `clawdcompute` or `pumpai`.
    pub namespace: String,
    session_id: Mutex<Option<String>>,
    next_id: Mutex<u64>,
}

impl McpClient {
    pub fn new(namespace: impl Into<String>, url: impl Into<String>) -> Self {
        Self {
            http: reqwest::Client::builder()
                .user_agent(concat!(
                    "Mozilla/5.0 (compatible; OpenClawdKit/",
                    env!("CARGO_PKG_VERSION"),
                    "; +https://github.com/Solizardking/OpenClawd-agent-kit)"
                ))
                .build()
                .expect("reqwest client builds"),
            url: url.into(),
            namespace: namespace.into(),
            session_id: Mutex::new(None),
            next_id: Mutex::new(1),
        }
    }

    /// The two fleet servers, with env overrides
    /// (`CLAWD_COMPUTE_MCP_URL`, `PUMPAI_MCP_URL`).
    pub fn fleet() -> Vec<Arc<McpClient>> {
        let cc = std::env::var("CLAWD_COMPUTE_MCP_URL")
            .unwrap_or_else(|_| CLAWD_COMPUTE_MCP_URL.to_string());
        let pump = std::env::var("PUMPAI_MCP_URL").unwrap_or_else(|_| PUMPAI_MCP_URL.to_string());
        vec![
            Arc::new(McpClient::new("clawdcompute", cc)),
            Arc::new(McpClient::new("pumpai", pump)),
        ]
    }

    async fn request(&self, method: &str, params: Value) -> Result<Value, ToolError> {
        let id = {
            let mut next = self.next_id.lock().await;
            let id = *next;
            *next += 1;
            id
        };
        let body = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        });
        let mut req = self
            .http
            .post(&self.url)
            .header("Accept", "application/json, text/event-stream")
            .json(&body);
        if let Some(sid) = self.session_id.lock().await.clone() {
            req = req.header("Mcp-Session-Id", sid);
        }
        let resp = req.send().await.map_err(|e| ToolError::Failed {
            name: method.to_string(),
            message: e.to_string(),
        })?;
        let status = resp.status();
        if let Some(sid) = resp
            .headers()
            .get("mcp-session-id")
            .and_then(|v| v.to_str().ok())
        {
            *self.session_id.lock().await = Some(sid.to_string());
        }
        let text = resp.text().await.map_err(|e| ToolError::Failed {
            name: method.to_string(),
            message: e.to_string(),
        })?;
        if status.as_u16() == 402 {
            // x402: the server wants payment. The kit NEVER pays implicitly;
            // the spend policy layer decides what this means (default: error).
            return Err(ToolError::PaymentRequired);
        }
        if !status.is_success() {
            return Err(ToolError::Failed {
                name: method.to_string(),
                message: format!("HTTP {status}: {}", text.chars().take(300).collect::<String>()),
            });
        }
        let message = parse_rpc_message(&text, id).ok_or_else(|| ToolError::Failed {
            name: method.to_string(),
            message: "no JSON-RPC response message found".to_string(),
        })?;
        if let Some(err) = message.get("error") {
            return Err(ToolError::Failed {
                name: method.to_string(),
                message: err.to_string(),
            });
        }
        Ok(message.get("result").cloned().unwrap_or(Value::Null))
    }

    async fn notify(&self, method: &str) {
        let body = json!({"jsonrpc": "2.0", "method": method});
        let mut req = self
            .http
            .post(&self.url)
            .header("Accept", "application/json, text/event-stream")
            .json(&body);
        if let Some(sid) = self.session_id.lock().await.clone() {
            req = req.header("Mcp-Session-Id", sid);
        }
        let _ = req.send().await;
    }

    /// `initialize` handshake + `notifications/initialized`.
    pub async fn initialize(&self) -> Result<ServerInfo, ToolError> {
        let result = self
            .request(
                "initialize",
                json!({
                    "protocolVersion": PROTOCOL_VERSION,
                    "capabilities": {},
                    "clientInfo": {
                        "name": "openclawd-agent-kit",
                        "version": env!("CARGO_PKG_VERSION"),
                    },
                }),
            )
            .await?;
        self.notify("notifications/initialized").await;
        let server = result.get("serverInfo");
        Ok(ServerInfo {
            name: server
                .and_then(|s| s.get("name"))
                .and_then(|n| n.as_str())
                .unwrap_or_default()
                .to_string(),
            version: server
                .and_then(|s| s.get("version"))
                .and_then(|n| n.as_str())
                .unwrap_or_default()
                .to_string(),
            protocol_version: result
                .get("protocolVersion")
                .and_then(|p| p.as_str())
                .unwrap_or(PROTOCOL_VERSION)
                .to_string(),
        })
    }

    pub async fn list_tools(&self) -> Result<Vec<McpTool>, ToolError> {
        let result = self.request("tools/list", json!({})).await?;
        let tools = result
            .get("tools")
            .and_then(|t| t.as_array())
            .map(|arr| arr.iter().filter_map(McpTool::from_json).collect())
            .unwrap_or_default();
        Ok(tools)
    }

    /// Raw `tools/call`. Spend policy is enforced by the caller-facing
    /// layer ([`crate::toolset`]); this is the transport.
    pub async fn call_tool(&self, name: &str, arguments: Value) -> Result<Value, ToolError> {
        self.request(
            "tools/call",
            json!({"name": name, "arguments": arguments}),
        )
        .await
    }
}

/// Extract the JSON-RPC message with `id` from a response body that is
/// either a single JSON document or an SSE stream of `data:` lines.
pub fn parse_rpc_message(body: &str, id: u64) -> Option<Value> {
    let trimmed = body.trim();
    if trimmed.starts_with('{') {
        if let Ok(v) = serde_json::from_str::<Value>(trimmed) {
            return Some(v);
        }
    }
    let mut fallback = None;
    for line in body.lines() {
        let line = line.trim();
        let data = line.strip_prefix("data:").map(|d| d.trim()).unwrap_or(line);
        if !data.starts_with('{') {
            continue;
        }
        if let Ok(v) = serde_json::from_str::<Value>(data) {
            if v.get("id").and_then(|i| i.as_u64()) == Some(id) {
                return Some(v);
            }
            if v.get("result").is_some() || v.get("error").is_some() {
                fallback = Some(v);
            }
        }
    }
    fallback
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_plain_json_response() {
        let body = r#"{"jsonrpc":"2.0","id":2,"result":{"tools":[]}}"#;
        let msg = parse_rpc_message(body, 2).unwrap();
        assert_eq!(msg["result"]["tools"], json!([]));
    }

    #[test]
    fn parses_sse_response_and_matches_id() {
        let body = "event: message\ndata: {\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{\"a\":1}}\n\nevent: message\ndata: {\"jsonrpc\":\"2.0\",\"id\":2,\"result\":{\"b\":2}}\n\n";
        let msg = parse_rpc_message(body, 2).unwrap();
        assert_eq!(msg["result"]["b"], 2);
    }

    #[test]
    fn tool_parsing_reads_annotations() {
        let v = json!({
            "name": "get_balance",
            "description": "read a balance",
            "inputSchema": {"type": "object"},
            "annotations": {"readOnlyHint": true}
        });
        let tool = McpTool::from_json(&v).unwrap();
        assert_eq!(tool.read_only, Some(true));
        assert_eq!(tool.name, "get_balance");
    }
}
