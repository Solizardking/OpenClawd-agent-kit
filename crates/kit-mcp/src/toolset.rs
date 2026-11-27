//! MCP servers as kit tools, behind a **fail-closed spend policy**.
//!
//! Classification (from MCP annotations):
//! - `readOnlyHint == true` → read-only: callable under every policy, and
//!   not consequential for the Jev gate.
//! - anything else (including *no* annotation) → state-changing: treated as
//!   consequential, and refused outright unless the operator explicitly
//!   opted in with [`SpendPolicy::allow_state_changing_calls`].
//!
//! Payments: the Clawd Compute gateway charges x402 ($0.01/call) for its
//! payable tools and answers `402 Payment Required`. The kit **never pays
//! implicitly** — there is no wallet, no signer, and no payment code path in
//! this crate. A 402 surfaces as [`ToolError::PaymentRequired`]. Enabling
//! payments is a deliberate future integration (a host-provided
//! `PaymentHandler`), not a config flag someone can fat-finger.

use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;

use kit_core::{Tool, ToolError, ToolRegistry};

use crate::client::{McpClient, McpTool};

#[derive(Debug, Clone)]
#[derive(Default)]
pub struct SpendPolicy {
    /// Permit calling tools that are not annotated read-only. Default
    /// `false`: the client is a catalog + read-only client.
    pub allow_state_changing_calls: bool,
    /// Reserved for a future host-provided payment handler. There is no
    /// built-in payer; setting this without a handler changes nothing.
    pub payments_enabled: bool,
}


impl SpendPolicy {
    /// May this tool be called at all under the policy?
    pub fn permits(&self, tool: &McpTool) -> bool {
        tool.read_only == Some(true) || self.allow_state_changing_calls
    }
}

/// One MCP tool, namespaced as `<server>__<tool>`, exposed to the kit's
/// registry (and therefore to the Jev gate when consequential).
pub struct NamespacedMcpTool {
    client: Arc<McpClient>,
    tool: McpTool,
    policy: SpendPolicy,
    namespaced_name: String,
}

impl NamespacedMcpTool {
    pub fn new(client: Arc<McpClient>, tool: McpTool, policy: SpendPolicy) -> Self {
        let namespaced_name = format!("{}__{}", client.namespace, tool.name);
        Self {
            client,
            tool,
            policy,
            namespaced_name,
        }
    }
}

#[async_trait]
impl Tool for NamespacedMcpTool {
    fn name(&self) -> &str {
        &self.namespaced_name
    }

    fn description(&self) -> &str {
        &self.tool.description
    }

    fn input_schema(&self) -> Value {
        self.tool.input_schema.clone()
    }

    fn consequential(&self) -> bool {
        // Anything not explicitly read-only is consequential: it goes
        // through the Jev gate before it can run.
        self.tool.read_only != Some(true)
    }

    async fn call(&self, arguments: Value) -> Result<Value, ToolError> {
        if !self.policy.permits(&self.tool) {
            return Err(ToolError::Denied(format!(
                "MCP tool `{}` is not annotated read-only and the spend policy \
                 does not allow state-changing calls (fail-closed default)",
                self.tool.name
            )));
        }
        self.client.call_tool(&self.tool.name, arguments).await
    }
}

/// Connect to servers, list their tools, and register them all into a
/// [`ToolRegistry`]. Returns the per-server tool counts for reporting.
pub async fn register_mcp_servers(
    registry: &mut ToolRegistry,
    clients: &[Arc<McpClient>],
    policy: SpendPolicy,
) -> Vec<(String, usize)> {
    let mut counts = Vec::new();
    for client in clients {
        match client.list_tools().await {
            Ok(tools) => {
                counts.push((client.namespace.clone(), tools.len()));
                for tool in tools {
                    registry.register(Arc::new(NamespacedMcpTool::new(
                        client.clone(),
                        tool,
                        policy.clone(),
                    )));
                }
            }
            Err(e) => {
                tracing::warn!(server = %client.namespace, error = %e, "MCP tools/list failed");
                counts.push((client.namespace.clone(), 0));
            }
        }
    }
    counts
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn tool(read_only: Option<bool>) -> McpTool {
        McpTool {
            name: "some_tool".to_string(),
            description: String::new(),
            input_schema: json!({"type": "object"}),
            read_only,
            destructive: None,
        }
    }

    #[test]
    fn default_policy_is_read_only() {
        let policy = SpendPolicy::default();
        assert!(policy.permits(&tool(Some(true))));
        assert!(!policy.permits(&tool(Some(false))));
        assert!(!policy.permits(&tool(None)), "unannotated tools are not presumed safe");
    }

    #[test]
    fn opt_in_allows_state_changing_but_never_pays() {
        let policy = SpendPolicy {
            allow_state_changing_calls: true,
            payments_enabled: false,
        };
        assert!(policy.permits(&tool(None)));
    }

    #[test]
    fn namespacing_uses_double_underscore() {
        let client = Arc::new(McpClient::new("pumpai", "https://pumpai.ai/mcp"));
        let t = NamespacedMcpTool::new(client, tool(Some(true)), SpendPolicy::default());
        assert_eq!(t.name(), "pumpai__some_tool");
        assert!(!t.consequential());
    }
}
