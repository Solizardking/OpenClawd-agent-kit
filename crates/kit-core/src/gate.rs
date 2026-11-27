//! The **Jev gate**: the policy checkpoint between "the agent wants to call
//! a tool" and "the tool runs".
//!
//! For every consequential call the gate asks Jev two narrow questions in a
//! single System One request, over a state object describing the call:
//!
//! 1. `consequential` (`noul`) — probability that executing this call moves
//!    funds, signs, trades, or otherwise changes external state in a way
//!    that is hard to reverse.
//! 2. `verdict` (`choice` over `proceed` / `deny` / `ask_human`) — Jev's
//!    recommendation given the kit's constitution and the stated rationale.
//!
//! Jev supplies judgments; **code owns the decision**:
//!
//! - Provider unconfigured or unreachable → `AskHuman` (fail closed).
//! - `consequential` probability < [`NON_CONSEQUENTIAL_BELOW`] → the call is
//!   not actually consequential → `Proceed`.
//! - Otherwise the `verdict` choice rules, but `proceed` is honoured only
//!   with both probability and confidence at or above
//!   [`PROCEED_MIN_PROBABILITY`] / [`PROCEED_MIN_CONFIDENCE`]; a weak or
//!   split judgment escalates to `AskHuman`. `deny` is honoured at
//!   [`DENY_MIN_CONFIDENCE`] and above; a weak `deny` also escalates —
//!   escalation is always the safe direction.
//!
//! Every evaluation is appended to an in-memory judgment log and emitted via
//! `tracing`, so an operator can audit exactly why each call ran or didn't.

use std::collections::BTreeMap;
use std::sync::Arc;

use serde_json::json;
use tokio::sync::Mutex;

use crate::jev::{JudgmentProvider, Question};
use crate::tool::ToolCallIntent;

/// Below this `noul` probability the call is treated as not consequential.
pub const NON_CONSEQUENTIAL_BELOW: f64 = 0.5;
/// Minimum Jev probability for the `proceed` option before code honours it.
pub const PROCEED_MIN_PROBABILITY: f64 = 0.6;
/// Minimum Jev confidence in a `proceed` verdict before code honours it.
pub const PROCEED_MIN_CONFIDENCE: f64 = 0.6;
/// Minimum Jev confidence in a `deny` verdict before code honours it.
pub const DENY_MIN_CONFIDENCE: f64 = 0.5;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GateDecision {
    Proceed,
    Deny,
    AskHuman,
}

/// One gate evaluation, kept for audit.
#[derive(Debug, Clone)]
pub struct JudgmentRecord {
    pub tool: String,
    /// Jev's `noul` probability that the call is consequential (if answered).
    pub consequential_probability: Option<f64>,
    /// Jev's raw `verdict` choice (if answered).
    pub verdict_choice: Option<String>,
    pub verdict_confidence: Option<f64>,
    /// The decision the *code* took from the judgment.
    pub decision: GateDecision,
    /// Why: which policy rule fired.
    pub reason: String,
}

pub struct JevGate {
    provider: Arc<dyn JudgmentProvider>,
    log: Mutex<Vec<JudgmentRecord>>,
}

impl JevGate {
    pub fn new(provider: Arc<dyn JudgmentProvider>) -> Self {
        Self {
            provider,
            log: Mutex::new(Vec::new()),
        }
    }

    pub fn configured(&self) -> bool {
        self.provider.configured()
    }

    pub async fn judgments(&self) -> Vec<JudgmentRecord> {
        self.log.lock().await.clone()
    }

    fn questions() -> BTreeMap<String, Question> {
        let mut verdict_criteria = BTreeMap::new();
        verdict_criteria.insert(
            "proceed".to_string(),
            "The call is within the agent's mandate, reversible or low-stakes, \
             and consistent with the constitution in `state.policy`."
                .to_string(),
        );
        verdict_criteria.insert(
            "deny".to_string(),
            "The call violates the constitution in `state.policy`, is deceptive \
             or harmful, or is clearly outside the agent's mandate."
                .to_string(),
        );
        verdict_criteria.insert(
            "ask_human".to_string(),
            "The stakes, irreversibility, or ambiguity are high enough that a \
             human operator should decide."
                .to_string(),
        );

        let mut questions = BTreeMap::new();
        questions.insert(
            "consequential".to_string(),
            Question::Noul {
                instructions: "Executing `state.call` would move funds, produce a \
                    signature, place a trade, send a message or payment, delete \
                    data, or otherwise change external state in a way that is \
                    hard to reverse."
                    .to_string(),
            },
        );
        questions.insert(
            "verdict".to_string(),
            Question::Choice {
                instructions: "Given the constitution and policy in `state.policy` \
                    and the rationale in `state.call`, should this tool call \
                    proceed, be denied, or be escalated to a human?"
                    .to_string(),
                criteria: verdict_criteria,
            },
        );
        questions
    }

    /// Evaluate one intended consequential call. Never panics; every failure
    /// mode resolves to `AskHuman`.
    pub async fn evaluate(&self, intent: &ToolCallIntent) -> JudgmentRecord {
        let record = self.evaluate_inner(intent).await;
        tracing::info!(
            tool = %intent.tool,
            decision = ?record.decision,
            reason = %record.reason,
            consequential = ?record.consequential_probability,
            verdict = ?record.verdict_choice,
            "jev gate"
        );
        self.log.lock().await.push(record.clone());
        record
    }

    async fn evaluate_inner(&self, intent: &ToolCallIntent) -> JudgmentRecord {
        let base = |decision, reason: &str| JudgmentRecord {
            tool: intent.tool.clone(),
            consequential_probability: None,
            verdict_choice: None,
            verdict_confidence: None,
            decision,
            reason: reason.to_string(),
        };

        if !self.provider.configured() {
            return base(
                GateDecision::AskHuman,
                "Jev is not configured (TYPESAFE_API_KEY unset); consequential calls fail closed to a human",
            );
        }

        let state = json!({
            "policy": crate::constitution::CONSTITUTION_FALLBACK,
            "call": {
                "tool": intent.tool,
                "arguments": intent.arguments,
                "rationale": intent.rationale,
            },
        });

        let response = match self.provider.ask(&state, &Self::questions()).await {
            Ok(r) => r,
            Err(e) => {
                return base(
                    GateDecision::AskHuman,
                    &format!("Jev judgment unavailable ({e}); failing closed to a human"),
                );
            }
        };

        let consequential = response.answers.get("consequential").and_then(|a| a.noul());
        let verdict = response.answers.get("verdict").and_then(|a| a.choice());

        let mut record = base(GateDecision::AskHuman, "");
        record.consequential_probability = consequential;
        if let Some((choice, confidence, _)) = verdict {
            record.verdict_choice = Some(choice.to_string());
            record.verdict_confidence = Some(confidence);
        }

        // A call Jev judges not-consequential may proceed without escalation.
        if let Some(p) = consequential {
            if p < NON_CONSEQUENTIAL_BELOW {
                record.decision = GateDecision::Proceed;
                record.reason =
                    format!("Jev judged the call not consequential (p={p:.2} < {NON_CONSEQUENTIAL_BELOW})");
                return record;
            }
        }

        match verdict {
            Some(("proceed", confidence, probabilities)) => {
                let p_proceed = probabilities.get("proceed").copied().unwrap_or(0.0);
                if p_proceed >= PROCEED_MIN_PROBABILITY && confidence >= PROCEED_MIN_CONFIDENCE {
                    record.decision = GateDecision::Proceed;
                    record.reason = format!(
                        "Jev verdict proceed with p={p_proceed:.2}, confidence={confidence:.2}"
                    );
                } else {
                    record.decision = GateDecision::AskHuman;
                    record.reason = format!(
                        "Jev leaned proceed but too weakly (p={p_proceed:.2}, confidence={confidence:.2}); escalating"
                    );
                }
            }
            Some(("deny", confidence, _)) => {
                if confidence >= DENY_MIN_CONFIDENCE {
                    record.decision = GateDecision::Deny;
                    record.reason = format!("Jev verdict deny, confidence={confidence:.2}");
                } else {
                    record.decision = GateDecision::AskHuman;
                    record.reason =
                        format!("Jev leaned deny but weakly (confidence={confidence:.2}); escalating");
                }
            }
            Some(("ask_human", _, _)) => {
                record.decision = GateDecision::AskHuman;
                record.reason = "Jev verdict ask_human".to_string();
            }
            _ => {
                record.decision = GateDecision::AskHuman;
                record.reason = "Jev returned no usable verdict; failing closed to a human".to_string();
            }
        }
        record
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jev::{Answer, SystemOneResponse, Usage};
    use async_trait::async_trait;

    struct StubProvider {
        configured: bool,
        answers: BTreeMap<String, Answer>,
    }

    #[async_trait]
    impl JudgmentProvider for StubProvider {
        async fn ask(
            &self,
            _state: &Value,
            _questions: &BTreeMap<String, Question>,
        ) -> Result<SystemOneResponse, crate::jev::JevError> {
            Ok(SystemOneResponse {
                model: "stub".to_string(),
                answers: self.answers.clone(),
                usage: Usage {
                    input_tokens: 1,
                    output_tokens: 1,
                },
            })
        }
        fn configured(&self) -> bool {
            self.configured
        }
    }

    fn intent() -> ToolCallIntent {
        ToolCallIntent {
            tool: "solana_transfer".to_string(),
            arguments: json!({"to": "SomePubkey", "lamports": 1000}),
            rationale: "pay for compute".to_string(),
        }
    }

    fn choice_answer(choice: &str, confidence: f64, p: f64) -> Answer {
        let mut probabilities = BTreeMap::new();
        probabilities.insert(choice.to_string(), p);
        Answer::Choice {
            choice: choice.to_string(),
            confidence,
            probabilities,
        }
    }

    #[tokio::test]
    async fn unconfigured_provider_fails_closed() {
        let gate = JevGate::new(Arc::new(StubProvider {
            configured: false,
            answers: BTreeMap::new(),
        }));
        let record = gate.evaluate(&intent()).await;
        assert_eq!(record.decision, GateDecision::AskHuman);
    }

    #[tokio::test]
    async fn strong_proceed_proceeds() {
        let mut answers = BTreeMap::new();
        answers.insert("consequential".to_string(), Answer::Noul { noul: 0.95 });
        answers.insert(
            "verdict".to_string(),
            choice_answer("proceed", 0.9, 0.9),
        );
        let gate = JevGate::new(Arc::new(StubProvider {
            configured: true,
            answers,
        }));
        let record = gate.evaluate(&intent()).await;
        assert_eq!(record.decision, GateDecision::Proceed);
        assert_eq!(gate.judgments().await.len(), 1);
    }

    #[tokio::test]
    async fn weak_proceed_escalates() {
        let mut answers = BTreeMap::new();
        answers.insert("consequential".to_string(), Answer::Noul { noul: 0.9 });
        answers.insert(
            "verdict".to_string(),
            choice_answer("proceed", 0.4, 0.45),
        );
        let gate = JevGate::new(Arc::new(StubProvider {
            configured: true,
            answers,
        }));
        assert_eq!(gate.evaluate(&intent()).await.decision, GateDecision::AskHuman);
    }

    #[tokio::test]
    async fn confident_deny_denies() {
        let mut answers = BTreeMap::new();
        answers.insert("consequential".to_string(), Answer::Noul { noul: 0.9 });
        answers.insert("verdict".to_string(), choice_answer("deny", 0.8, 0.8));
        let gate = JevGate::new(Arc::new(StubProvider {
            configured: true,
            answers,
        }));
        assert_eq!(gate.evaluate(&intent()).await.decision, GateDecision::Deny);
    }

    #[tokio::test]
    async fn not_consequential_proceeds_without_verdict() {
        let mut answers = BTreeMap::new();
        answers.insert("consequential".to_string(), Answer::Noul { noul: 0.1 });
        answers.insert(
            "verdict".to_string(),
            choice_answer("ask_human", 0.9, 0.9),
        );
        let gate = JevGate::new(Arc::new(StubProvider {
            configured: true,
            answers,
        }));
        assert_eq!(gate.evaluate(&intent()).await.decision, GateDecision::Proceed);
    }
}
