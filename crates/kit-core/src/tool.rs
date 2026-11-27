//! Tool abstraction + registry.
//!
//! Every capability the agent can invoke is a [`Tool`]. The one field the
//! whole safety story hangs on is [`Tool::consequential`]: a consequential
//! tool moves funds, signs, trades, sends, deletes, or otherwise changes
//! external state in a way that is hard to reverse. Consequential tools are
//! routed through the Jev gate (see [`crate::gate`]) before they run, and
//! are simulated instead of executed while the loop is in dry-run mode.
//!
//! When in doubt, mark a tool consequential. Read-only is a claim the tool
//! author must be able to defend.

use std::collections::BTreeMap;
use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;

#[derive(Debug, thiserror::Error)]
pub enum ToolError {
    #[error("tool `{0}` is not registered")]
    Unknown(String),
    #[error("tool `{name}` failed: {message}")]
    Failed { name: String, message: String },
    #[error("tool call denied by policy: {0}")]
    Denied(String),
    #[error("tool call blocked: payment required but payments are not enabled")]
    PaymentRequired,
}

/// A tool call the agent (or a human operator) wants to make.
#[derive(Debug, Clone)]
pub struct ToolCallIntent {
    pub tool: String,
    pub arguments: Value,
    /// Free-text context for the judgment model: what is being attempted and why.
    pub rationale: String,
}

#[async_trait]
pub trait Tool: Send + Sync {
    /// Unique name, e.g. `solana_get_balance` or `clawdcompute__portal_identity`.
    fn name(&self) -> &str;

    fn description(&self) -> &str;

    /// JSON Schema for the arguments object.
    fn input_schema(&self) -> Value;

    /// True when executing this tool changes external state in a way that is
    /// hard to reverse (funds, signatures, trades, sends, deletes, payments).
    fn consequential(&self) -> bool {
        false
    }

    async fn call(&self, arguments: Value) -> Result<Value, ToolError>;
}

/// Name → tool map. Insertion order is preserved for listings.
#[derive(Default)]
pub struct ToolRegistry {
    tools: BTreeMap<String, Arc<dyn Tool>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, tool: Arc<dyn Tool>) -> &mut Self {
        self.tools.insert(tool.name().to_string(), tool);
        self
    }

    pub fn get(&self, name: &str) -> Option<Arc<dyn Tool>> {
        self.tools.get(name).cloned()
    }

    pub fn names(&self) -> Vec<String> {
        self.tools.keys().cloned().collect()
    }

    pub fn len(&self) -> usize {
        self.tools.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tools.is_empty()
    }

    /// Is `name` a registered consequential tool? Unknown tools are treated
    /// as consequential — an unregistered name must never slip past the gate.
    pub fn is_consequential(&self, name: &str) -> bool {
        self.tools.get(name).map(|t| t.consequential()).unwrap_or(true)
    }

    pub async fn call(&self, intent: &ToolCallIntent) -> Result<Value, ToolError> {
        let tool = self
            .get(&intent.tool)
            .ok_or_else(|| ToolError::Unknown(intent.tool.clone()))?;
        tool.call(intent.arguments.clone()).await
    }
}
