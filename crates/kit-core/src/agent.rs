//! The reasoning loop.
//!
//! Deliberately provider-agnostic: the loop drives any [`Reasoner`] (an LLM
//! backend, a scripted planner, a test double) through propose → gate →
//! execute cycles. The loop itself owns the safety semantics:
//!
//! - Read-only tools run immediately.
//! - Consequential tools go through the [`JevGate`] first. `Proceed` runs
//!   them (or simulates them in dry-run), `Deny` refuses them, and
//!   `AskHuman` routes to the configured [`HumanApprover`].
//! - **Dry-run is the default.** In dry-run a consequential tool that clears
//!   the gate is *simulated*: the loop reports what would have run and feeds
//!   a synthetic result back, but the tool's `call` is never invoked.
//! - The default [`HumanApprover`] denies. An unattended kit therefore never
//!   executes a consequential call on an escalation — by design.

use std::sync::Arc;

use async_trait::async_trait;
use serde_json::{json, Value};

use crate::gate::{GateDecision, JevGate};
use crate::tool::{ToolCallIntent, ToolError, ToolRegistry};

#[derive(Debug, Clone)]
pub enum Message {
    System(String),
    User(String),
    Assistant(String),
    ToolResult { tool: String, result: Value },
}

#[derive(Debug, Clone)]
pub enum ReasonerStep {
    /// Final answer; the loop ends.
    Final(String),
    /// The reasoner wants a tool executed.
    CallTool(ToolCallIntent),
}

/// The "brain" the loop drives. Implementations wrap an LLM provider and
/// translate its native tool-calling into [`ReasonerStep`]s.
#[async_trait]
pub trait Reasoner: Send + Sync {
    async fn next_step(&self, history: &[Message]) -> Result<ReasonerStep, ToolError>;
}

/// Decides escalations. The kit ships a deny-by-default implementation;
/// interactive hosts plug in a real prompt, services plug in an approval API.
#[async_trait]
pub trait HumanApprover: Send + Sync {
    async fn approve(&self, intent: &ToolCallIntent, gate_reason: &str) -> bool;
}

/// Deny every escalation. The safe default for unattended agents.
pub struct DenyApprover;

#[async_trait]
impl HumanApprover for DenyApprover {
    async fn approve(&self, _intent: &ToolCallIntent, _gate_reason: &str) -> bool {
        false
    }
}

/// Approve every escalation. For interactive local use only — never ship an
/// unattended agent with this.
pub struct AllowApprover;

#[async_trait]
impl HumanApprover for AllowApprover {
    async fn approve(&self, _intent: &ToolCallIntent, _gate_reason: &str) -> bool {
        true
    }
}

#[derive(Debug, Clone)]
pub enum LoopEvent {
    Final(String),
    ToolExecuted {
        tool: String,
        result: Value,
    },
    ToolSimulated {
        tool: String,
    },
    ToolDenied {
        tool: String,
        reason: String,
    },
    Escalated {
        tool: String,
        approved: bool,
    },
}

pub struct ReasoningLoop {
    reasoner: Arc<dyn Reasoner>,
    tools: Arc<ToolRegistry>,
    gate: Arc<JevGate>,
    approver: Arc<dyn HumanApprover>,
    /// Dry-run is the default; only an explicit operator choice turns it off.
    pub dry_run: bool,
    pub max_steps: usize,
}

impl ReasoningLoop {
    pub fn new(reasoner: Arc<dyn Reasoner>, tools: Arc<ToolRegistry>, gate: Arc<JevGate>) -> Self {
        Self {
            reasoner,
            tools,
            gate,
            approver: Arc::new(DenyApprover),
            dry_run: true,
            max_steps: 32,
        }
    }

    pub fn with_approver(mut self, approver: Arc<dyn HumanApprover>) -> Self {
        self.approver = approver;
        self
    }

    pub fn live(mut self) -> Self {
        self.dry_run = false;
        self
    }

    /// Run until the reasoner returns `Final`, a step budget is exhausted,
    /// or an unrecoverable error occurs. Returns the event transcript.
    pub async fn run(&self, prompt: &str) -> Result<Vec<LoopEvent>, ToolError> {
        let mut history = vec![
            Message::System(crate::constitution::clawd_system_preamble()),
            Message::User(prompt.to_string()),
        ];
        let mut events = Vec::new();

        for _ in 0..self.max_steps {
            match self.reasoner.next_step(&history).await? {
                ReasonerStep::Final(text) => {
                    history.push(Message::Assistant(text.clone()));
                    events.push(LoopEvent::Final(text));
                    return Ok(events);
                }
                ReasonerStep::CallTool(intent) => {
                    let result = self.execute_gated(&intent, &mut events).await;
                    history.push(Message::ToolResult {
                        tool: intent.tool.clone(),
                        result,
                    });
                }
            }
        }
        Err(ToolError::Failed {
            name: "<loop>".to_string(),
            message: format!("step budget exhausted ({})", self.max_steps),
        })
    }

    async fn execute_gated(&self, intent: &ToolCallIntent, events: &mut Vec<LoopEvent>) -> Value {
        if !self.tools.is_consequential(&intent.tool) {
            return match self.tools.call(intent).await {
                Ok(v) => {
                    events.push(LoopEvent::ToolExecuted {
                        tool: intent.tool.clone(),
                        result: v.clone(),
                    });
                    v
                }
                Err(e) => json!({"error": e.to_string()}),
            };
        }

        // Consequential: the Jev gate decides first.
        let record = self.gate.evaluate(intent).await;
        match record.decision {
            GateDecision::Deny => {
                events.push(LoopEvent::ToolDenied {
                    tool: intent.tool.clone(),
                    reason: record.reason.clone(),
                });
                json!({"denied": true, "reason": record.reason})
            }
            GateDecision::AskHuman => {
                let approved = self.approver.approve(intent, &record.reason).await;
                events.push(LoopEvent::Escalated {
                    tool: intent.tool.clone(),
                    approved,
                });
                if approved {
                    self.execute_consequential(intent, events).await
                } else {
                    json!({"denied": true, "reason": format!("escalated to a human and not approved: {}", record.reason)})
                }
            }
            GateDecision::Proceed => self.execute_consequential(intent, events).await,
        }
    }

    async fn execute_consequential(
        &self,
        intent: &ToolCallIntent,
        events: &mut Vec<LoopEvent>,
    ) -> Value {
        if self.dry_run {
            events.push(LoopEvent::ToolSimulated {
                tool: intent.tool.clone(),
            });
            return json!({
                "simulated": true,
                "note": "dry-run: the gate cleared this call but the tool was NOT executed",
                "tool": intent.tool,
                "arguments": intent.arguments,
            });
        }
        match self.tools.call(intent).await {
            Ok(v) => {
                events.push(LoopEvent::ToolExecuted {
                    tool: intent.tool.clone(),
                    result: v.clone(),
                });
                v
            }
            Err(e) => json!({"error": e.to_string()}),
        }
    }
}

/// A scripted reasoner for demos and tests: replays a fixed list of steps,
/// then answers `Final("done")`.
pub struct ScriptedReasoner {
    steps: tokio::sync::Mutex<std::collections::VecDeque<ReasonerStep>>,
}

impl ScriptedReasoner {
    pub fn new(steps: Vec<ReasonerStep>) -> Self {
        Self {
            steps: tokio::sync::Mutex::new(steps.into()),
        }
    }
}

#[async_trait]
impl Reasoner for ScriptedReasoner {
    async fn next_step(&self, _history: &[Message]) -> Result<ReasonerStep, ToolError> {
        Ok(self
            .steps
            .lock()
            .await
            .pop_front()
            .unwrap_or(ReasonerStep::Final("done".to_string())))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jev::UnconfiguredProvider;
    use crate::tool::Tool;
    use async_trait::async_trait;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct CountingTool {
        consequential: bool,
        calls: Arc<AtomicUsize>,
    }

    #[async_trait]
    impl Tool for CountingTool {
        fn name(&self) -> &str {
            if self.consequential {
                "danger"
            } else {
                "peek"
            }
        }
        fn description(&self) -> &str {
            "test tool"
        }
        fn input_schema(&self) -> Value {
            json!({"type": "object"})
        }
        fn consequential(&self) -> bool {
            self.consequential
        }
        async fn call(&self, _arguments: Value) -> Result<Value, ToolError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(json!({"ok": true}))
        }
    }

    async fn build_loop(consequential_calls: Arc<AtomicUsize>, steps: Vec<ReasonerStep>) -> ReasoningLoop {
        let mut registry = ToolRegistry::new();
        registry.register(Arc::new(CountingTool {
            consequential: false,
            calls: Arc::new(AtomicUsize::new(0)),
        }));
        registry.register(Arc::new(CountingTool {
            consequential: true,
            calls: consequential_calls,
        }));
        let gate = Arc::new(JevGate::new(Arc::new(UnconfiguredProvider)));
        ReasoningLoop::new(
            Arc::new(ScriptedReasoner::new(steps)),
            Arc::new(registry),
            gate,
        )
    }

    #[tokio::test]
    async fn consequential_call_without_jev_is_never_executed() {
        let calls = Arc::new(AtomicUsize::new(0));
        let loopy = build_loop(
            calls.clone(),
            vec![ReasonerStep::CallTool(ToolCallIntent {
                tool: "danger".to_string(),
                arguments: json!({}),
                rationale: "test".to_string(),
            })],
        )
        .await;
        let events = loopy.run("go").await.unwrap();
        assert_eq!(calls.load(Ordering::SeqCst), 0, "fail-closed: no execution");
        assert!(events.iter().any(|e| matches!(e, LoopEvent::Escalated { approved: false, .. })));
    }

    #[tokio::test]
    async fn read_only_tool_runs_immediately() {
        let calls = Arc::new(AtomicUsize::new(0));
        let loopy = build_loop(
            calls,
            vec![ReasonerStep::CallTool(ToolCallIntent {
                tool: "peek".to_string(),
                arguments: json!({}),
                rationale: "look".to_string(),
            })],
        )
        .await;
        let events = loopy.run("go").await.unwrap();
        assert!(events.iter().any(|e| matches!(e, LoopEvent::ToolExecuted { tool, .. } if tool == "peek")));
    }
}
