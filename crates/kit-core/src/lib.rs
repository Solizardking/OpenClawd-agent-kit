//! kit-core — the OpenClawd agent kit's core.
//!
//! - [`constitution`] — the Clawd constitution every kit agent carries.
//! - [`tool`] — the tool abstraction + registry; `consequential` is the
//!   load-bearing flag.
//! - [`jev`] — TypeSafe Jev (System One) judgment client.
//! - [`gate`] — the Jev gate: policy in code, judgments from Jev, fail-closed.
//! - [`agent`] — the provider-agnostic reasoning loop (dry-run by default).

pub mod agent;
pub mod constitution;
pub mod gate;
pub mod jev;
pub mod tool;

pub use agent::{HumanApprover, LoopEvent, Message, Reasoner, ReasonerStep, ReasoningLoop};
pub use gate::{GateDecision, JevGate, JudgmentRecord};
pub use jev::{JevClient, JudgmentProvider, Question, UnconfiguredProvider};
pub use tool::{Tool, ToolCallIntent, ToolError, ToolRegistry};

use std::sync::Arc;

/// Build a gate from the environment: a live Jev client when
/// `TYPESAFE_API_KEY` is set, otherwise the fail-closed unconfigured
/// provider.
pub fn gate_from_env() -> Arc<JevGate> {
    match JevClient::from_env() {
        Some(client) => Arc::new(JevGate::new(Arc::new(client))),
        None => Arc::new(JevGate::new(Arc::new(UnconfiguredProvider))),
    }
}
