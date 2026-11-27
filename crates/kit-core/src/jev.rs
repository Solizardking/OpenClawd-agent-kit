//! TypeSafe **Jev** (System One) client.
//!
//! Jev is TypeSafe's flagship System One model: it returns typed judgments
//! and probabilities, not prose. The kit asks it narrow questions and keeps
//! all policy in code (see [`crate::gate`]).
//!
//! HTTP contract (verified against <https://docs.typesafe.ai/api.md>):
//! `POST https://api.typesafe.ai/v1/systemone`, `Authorization: Bearer <key>`,
//! body `{ model, state, questions }` where each question is
//! `{ type, instructions, criteria? }`:
//! - `noul` — probability the condition holds (no criteria, no confidence)
//! - `choice` — one of a named set (criteria = map of name → description);
//!   answer carries `choice`, `confidence`, and per-option `probabilities`
//! - `score` — probability-weighted position on ordered levels
//!   (criteria = array of level descriptions)
//!
//! The API key is read from `TYPESAFE_API_KEY` **only** — never from files,
//! flags, or source. If the key is absent the client is `None` and the gate
//! fails closed.

use std::collections::BTreeMap;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const DEFAULT_MODEL: &str = "jev-latest";
pub const API_URL: &str = "https://api.typesafe.ai/v1/systemone";

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Question {
    Noul {
        instructions: String,
    },
    Choice {
        instructions: String,
        /// option name → what choosing it means
        criteria: BTreeMap<String, String>,
    },
    Score {
        instructions: String,
        /// ordered level descriptions, lowest first
        criteria: Vec<String>,
    },
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Answer {
    Noul {
        /// probability (0..=1) that the condition holds
        noul: f64,
    },
    Choice {
        choice: String,
        confidence: f64,
        #[serde(default)]
        probabilities: BTreeMap<String, f64>,
    },
    Score {
        score: f64,
        confidence: f64,
        #[serde(default)]
        probabilities: BTreeMap<String, f64>,
    },
}

impl Answer {
    pub fn choice(&self) -> Option<(&str, f64, &BTreeMap<String, f64>)> {
        match self {
            Answer::Choice {
                choice,
                confidence,
                probabilities,
            } => Some((choice, *confidence, probabilities)),
            _ => None,
        }
    }

    pub fn noul(&self) -> Option<f64> {
        match self {
            Answer::Noul { noul } => Some(*noul),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Usage {
    pub input_tokens: u64,
    pub output_tokens: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SystemOneResponse {
    pub model: String,
    pub answers: BTreeMap<String, Answer>,
    pub usage: Usage,
}

#[derive(Debug, thiserror::Error)]
pub enum JevError {
    #[error("TYPESAFE_API_KEY is not set — Jev judgment unavailable")]
    Unconfigured,
    #[error("Jev HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("Jev API returned status {status}: {body}")]
    Api { status: u16, body: String },
    #[error("Jev response missing answer for question `{0}`")]
    MissingAnswer(String),
}

/// Anything that can answer Jev-style questions. The reasoning loop's gate
/// depends on this trait, not on the HTTP client, so tests (and alternative
/// transports, e.g. a local judgment service) can substitute their own.
#[async_trait]
pub trait JudgmentProvider: Send + Sync {
    async fn ask(
        &self,
        state: &Value,
        questions: &BTreeMap<String, Question>,
    ) -> Result<SystemOneResponse, JevError>;

    /// Whether this provider can actually reach a judgment model.
    fn configured(&self) -> bool {
        true
    }
}

/// HTTP client for the TypeSafe System One API.
pub struct JevClient {
    http: reqwest::Client,
    api_key: String,
    model: String,
}

impl JevClient {
    /// Build from `TYPESAFE_API_KEY`. Returns `None` when the key is unset —
    /// callers must treat that as "no judgment available", not as approval.
    pub fn from_env() -> Option<Self> {
        let api_key = std::env::var("TYPESAFE_API_KEY").ok()?;
        if api_key.trim().is_empty() {
            return None;
        }
        Some(Self {
            http: reqwest::Client::new(),
            api_key,
            model: DEFAULT_MODEL.to_string(),
        })
    }

    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }
}

#[async_trait]
impl JudgmentProvider for JevClient {
    async fn ask(
        &self,
        state: &Value,
        questions: &BTreeMap<String, Question>,
    ) -> Result<SystemOneResponse, JevError> {
        let body = serde_json::json!({
            "model": self.model,
            "state": state,
            "questions": questions,
        });
        let resp = self
            .http
            .post(API_URL)
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await?;
        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(JevError::Api {
                status: status.as_u16(),
                body: text.chars().take(500).collect(),
            });
        }
        Ok(resp.json::<SystemOneResponse>().await?)
    }
}

/// A provider that never answers. Used when no Jev key is configured: the
/// gate sees `configured() == false` and fails closed (ask a human).
pub struct UnconfiguredProvider;

#[async_trait]
impl JudgmentProvider for UnconfiguredProvider {
    async fn ask(
        &self,
        _state: &Value,
        _questions: &BTreeMap<String, Question>,
    ) -> Result<SystemOneResponse, JevError> {
        Err(JevError::Unconfigured)
    }

    fn configured(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn question_serialization_matches_api_shape() {
        let mut criteria = BTreeMap::new();
        criteria.insert("proceed".to_string(), "Safe to execute".to_string());
        let q = Question::Choice {
            instructions: "Should this call run?".to_string(),
            criteria,
        };
        let v = serde_json::to_value(&q).unwrap();
        assert_eq!(v["type"], "choice");
        assert_eq!(v["criteria"]["proceed"], "Safe to execute");

        let n = serde_json::to_value(Question::Noul {
            instructions: "Is this consequential?".to_string(),
        })
        .unwrap();
        assert_eq!(n["type"], "noul");
        assert!(n.get("criteria").is_none());
    }

    #[test]
    fn answer_deserialization_matches_api_shape() {
        let raw = r#"{
            "model": "jev-latest",
            "answers": {
                "consequential": {"type": "noul", "noul": 0.93},
                "verdict": {"type": "choice", "choice": "proceed", "confidence": 0.81,
                            "probabilities": {"proceed": 0.81, "deny": 0.1, "ask_human": 0.09}}
            },
            "usage": {"input_tokens": 120, "output_tokens": 12}
        }"#;
        let resp: SystemOneResponse = serde_json::from_str(raw).unwrap();
        assert_eq!(resp.answers["consequential"].noul(), Some(0.93));
        let (choice, conf, _) = resp.answers["verdict"].choice().unwrap();
        assert_eq!(choice, "proceed");
        assert!((conf - 0.81).abs() < 1e-9);
    }
}
