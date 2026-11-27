//! kit-http — the OpenClawd kit's optional HTTP service (agent-as-a-service).
//!
//! The original kit served its agents over actix-web behind the `http`
//! feature. The rewrite serves the same idea on **axum**: a small JSON API
//! over the kit's registry + Jev gate.
//!
//! Endpoints:
//! - `GET  /health`         → `{ "status": "ok" }`
//! - `GET  /tools`          → registered tools with descriptions + flags
//! - `POST /gate/evaluate`  → run the Jev gate on a hypothetical call
//! - `POST /tools/call`     → gated tool call; consequential calls are
//!   simulated (dry-run) unless the service was started with
//!   `KIT_HTTP_LIVE=1` — and even then the gate and the registry's own
//!   policies (e.g. the MCP spend policy) still apply.
//!
//! This crate is in the workspace but not in `default-members`:
//! `cargo check -p kit-http`, run via [`serve`].

use std::sync::Arc;

use axum::{extract::State, routing::{get, post}, Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};

use kit_core::{GateDecision, JevGate, ToolCallIntent, ToolRegistry};

pub struct ServiceState {
    pub tools: Arc<ToolRegistry>,
    pub gate: Arc<JevGate>,
    /// Dry-run unless the operator explicitly started the service live.
    pub dry_run: bool,
}

impl ServiceState {
    pub fn new(tools: Arc<ToolRegistry>, gate: Arc<JevGate>) -> Self {
        Self {
            tools,
            gate,
            dry_run: std::env::var("KIT_HTTP_LIVE").ok().as_deref() != Some("1"),
        }
    }
}

pub fn router(state: Arc<ServiceState>) -> Router {
    Router::new()
        .route("/health", get(|| async { Json(json!({"status": "ok"})) }))
        .route("/tools", get(list_tools))
        .route("/gate/evaluate", post(gate_evaluate))
        .route("/tools/call", post(call_tool))
        .with_state(state)
}

async fn list_tools(State(state): State<Arc<ServiceState>>) -> Json<Value> {
    let mut out = Vec::new();
    for name in state.tools.names() {
        if let Some(tool) = state.tools.get(&name) {
            out.push(json!({
                "name": tool.name(),
                "description": tool.description(),
                "consequential": tool.consequential(),
                "input_schema": tool.input_schema(),
            }));
        }
    }
    Json(json!({"tools": out, "dry_run": state.dry_run}))
}

#[derive(Deserialize)]
struct CallRequest {
    tool: String,
    #[serde(default)]
    arguments: Value,
    #[serde(default)]
    rationale: String,
}

fn record_json(record: &kit_core::JudgmentRecord) -> Value {
    json!({
        "tool": record.tool,
        "decision": format!("{:?}", record.decision),
        "reason": record.reason,
        "consequential_probability": record.consequential_probability,
        "verdict_choice": record.verdict_choice,
        "verdict_confidence": record.verdict_confidence,
    })
}

async fn gate_evaluate(
    State(state): State<Arc<ServiceState>>,
    Json(req): Json<CallRequest>,
) -> Json<Value> {
    let intent = ToolCallIntent {
        tool: req.tool,
        arguments: req.arguments,
        rationale: req.rationale,
    };
    if !state.tools.is_consequential(&intent.tool) {
        return Json(json!({"decision": "not-required", "reason": "tool is read-only"}));
    }
    let record = state.gate.evaluate(&intent).await;
    Json(record_json(&record))
}

async fn call_tool(
    State(state): State<Arc<ServiceState>>,
    Json(req): Json<CallRequest>,
) -> Json<Value> {
    let intent = ToolCallIntent {
        tool: req.tool,
        arguments: req.arguments,
        rationale: req.rationale,
    };
    if state.tools.is_consequential(&intent.tool) {
        let record = state.gate.evaluate(&intent).await;
        if record.decision != GateDecision::Proceed {
            return Json(json!({
                "executed": false,
                "gate": record_json(&record),
            }));
        }
        if state.dry_run {
            return Json(json!({
                "executed": false,
                "simulated": true,
                "gate": record_json(&record),
                "note": "dry-run service: gate cleared the call but it was not executed",
            }));
        }
    }
    match state.tools.call(&intent).await {
        Ok(v) => Json(json!({"executed": true, "result": v})),
        Err(e) => Json(json!({"executed": false, "error": e.to_string()})),
    }
}

/// Serve on `addr` (e.g. `0.0.0.0:8080`) until the process is stopped.
pub async fn serve(state: Arc<ServiceState>, addr: &str) -> anyhow::Result<()> {
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!("kit-http listening on {addr} (dry_run={})", state.dry_run);
    axum::serve(listener, router(state)).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use kit_core::UnconfiguredProvider;

    #[tokio::test]
    async fn health_route_shape() {
        let state = Arc::new(ServiceState {
            tools: Arc::new(ToolRegistry::new()),
            gate: Arc::new(JevGate::new(Arc::new(UnconfiguredProvider))),
            dry_run: true,
        });
        let app = router(state);
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        let body: Value = reqwest::get(format!("http://{addr}/health"))
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        assert_eq!(body["status"], "ok");
    }
}
