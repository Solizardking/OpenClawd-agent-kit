//! Executable models of the kit's core safety invariants.
//!
//! Each model here has a statement-for-statement counterpart in the Lean
//! package under `lean/` (`OpenClawdKit/Invariants.lean`), which is pinned
//! against the Solana Foundation's **solanalib**. The Rust models are the
//! ones that run today (unit + property tests in CI); the Lean statements
//! are the ones a prover — or the emerging Solana Lean client stack —
//! checks tomorrow. Names are kept identical on both sides on purpose:
//!
//! | Rust (this crate)              | Lean (`lean/`)                    |
//! |--------------------------------|-----------------------------------|
//! | `SpendLedger::authorize`       | `SpendState.authorize`            |
//! | `spend_monotone` (test)        | `SpendState.spend_monotone`       |
//! | `cap_never_exceeded` (test)    | `SpendState.cap_respected`        |
//! | `GateLog::execute`             | `GateLog.execute`                 |
//! | `no_execution_without_judgment`| `GateLog.no_execution_without_judgment` |
//! | `SignerPolicy::permits`        | `SignerPolicy.permits`            |

/// A spend ledger with a hard cap — the model behind every paid tool call
/// (x402 MCP calls, API spend, transfer budgets).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpendLedger {
    /// Total budget in the smallest unit (e.g. micro-USDC, lamports).
    pub cap: u64,
    /// Cumulative authorized spend. Monotone non-decreasing by construction.
    pub spent: u64,
}

impl SpendLedger {
    pub fn new(cap: u64) -> Self {
        Self { cap, spent: 0 }
    }

    /// Authorize spending `amount`. Succeeds only if the cumulative spend
    /// stays within the cap; on failure the ledger is unchanged.
    pub fn authorize(&mut self, amount: u64) -> bool {
        match self.spent.checked_add(amount) {
            Some(next) if next <= self.cap => {
                self.spent = next;
                true
            }
            _ => false,
        }
    }

    pub fn remaining(&self) -> u64 {
        self.cap - self.spent
    }
}

/// The Jev-gate invariant, as a state machine: a consequential call may
/// transition to `Executed` **only** from a state carrying a prior
/// `Judged` record for that same call. This is the exact property the
/// reasoning loop relies on, stated so both Rust tests and Lean can check it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CallState {
    Proposed,
    Judged { decision: Decision },
    Executed,
    Refused,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    Proceed,
    Deny,
    AskHuman,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GateLog {
    pub state: CallState,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum GateError {
    #[error("execution attempted without a prior judgment — the gate invariant forbids this")]
    NoJudgment,
    #[error("execution attempted after a non-proceed judgment")]
    NotApproved,
    #[error("call is already in a terminal state")]
    Terminal,
}

impl GateLog {
    pub fn new() -> Self {
        Self {
            state: CallState::Proposed,
        }
    }

    pub fn record_judgment(&mut self, decision: Decision) -> Result<(), GateError> {
        match self.state {
            CallState::Proposed => {
                self.state = CallState::Judged { decision };
                Ok(())
            }
            CallState::Executed | CallState::Refused => Err(GateError::Terminal),
            CallState::Judged { .. } => Err(GateError::Terminal),
        }
    }

    /// The load-bearing transition. From `Proposed` (no judgment) this is
    /// an error — there is no code path from proposal to execution that
    /// skips the gate.
    pub fn execute(&mut self) -> Result<(), GateError> {
        match self.state {
            CallState::Proposed => Err(GateError::NoJudgment),
            CallState::Judged {
                decision: Decision::Proceed,
            } => {
                self.state = CallState::Executed;
                Ok(())
            }
            CallState::Judged { .. } => Err(GateError::NotApproved),
            CallState::Executed | CallState::Refused => Err(GateError::Terminal),
        }
    }

    pub fn refuse(&mut self) -> Result<(), GateError> {
        match self.state {
            CallState::Proposed | CallState::Judged { .. } => {
                self.state = CallState::Refused;
                Ok(())
            }
            CallState::Executed | CallState::Refused => Err(GateError::Terminal),
        }
    }
}

impl Default for GateLog {
    fn default() -> Self {
        Self::new()
    }
}

/// Signer policy: which programs a signer may interact with and the maximum
/// value a single transaction may move. Enforcement is a pure predicate —
/// the same shape the Lean spec states.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignerPolicy {
    /// Base58 program ids the signer may build transactions against.
    /// An empty list means *no* programs (fail closed), not all programs.
    pub allowed_programs: Vec<String>,
    /// Maximum lamports a single transaction may transfer.
    pub max_lamports_per_tx: u64,
}

impl SignerPolicy {
    pub fn permits(&self, program: &str, lamports: u64) -> bool {
        lamports <= self.max_lamports_per_tx && self.allowed_programs.iter().any(|p| p == program)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Property: spend is monotone non-decreasing and never exceeds the cap,
    /// over arbitrary authorization sequences (deterministic pseudo-random
    /// walk — the same statement as Lean `SpendState.spend_monotone` +
    /// `SpendState.cap_respected`).
    #[test]
    fn spend_monotone_and_cap_respected() {
        let mut ledger = SpendLedger::new(1_000);
        let mut rng: u64 = 0x853c49e6748fea9b;
        let mut last = 0;
        for _ in 0..10_000 {
            // xorshift64*
            rng ^= rng >> 12;
            rng ^= rng << 25;
            rng ^= rng >> 27;
            let amount = (rng.wrapping_mul(0x2545F4914F6CDD1D) >> 56) % 50;
            ledger.authorize(amount);
            assert!(ledger.spent >= last, "spend must be monotone");
            assert!(ledger.spent <= ledger.cap, "cap must never be exceeded");
            last = ledger.spent;
        }
    }

    #[test]
    fn authorize_fails_atomically_at_cap() {
        let mut ledger = SpendLedger::new(100);
        assert!(ledger.authorize(60));
        assert!(!ledger.authorize(41));
        assert_eq!(ledger.spent, 60, "failed authorization must not move the ledger");
        assert!(ledger.authorize(40));
        assert_eq!(ledger.remaining(), 0);
    }

    /// The gate invariant: no execution without a judgment.
    #[test]
    fn no_execution_without_judgment() {
        let mut log = GateLog::new();
        assert_eq!(log.execute(), Err(GateError::NoJudgment));

        let mut log = GateLog::new();
        log.record_judgment(Decision::Proceed).unwrap();
        assert!(log.execute().is_ok());

        let mut log = GateLog::new();
        log.record_judgment(Decision::Deny).unwrap();
        assert_eq!(log.execute(), Err(GateError::NotApproved));

        let mut log = GateLog::new();
        log.record_judgment(Decision::AskHuman).unwrap();
        assert_eq!(log.execute(), Err(GateError::NotApproved));
    }

    #[test]
    fn signer_policy_fails_closed() {
        let policy = SignerPolicy {
            allowed_programs: vec![],
            max_lamports_per_tx: 1_000_000,
        };
        assert!(!policy.permits("11111111111111111111111111111111", 1));

        let policy = SignerPolicy {
            allowed_programs: vec!["11111111111111111111111111111111".to_string()],
            max_lamports_per_tx: 1_000_000,
        };
        assert!(policy.permits("11111111111111111111111111111111", 999_999));
        assert!(!policy.permits("11111111111111111111111111111111", 1_000_001));
        assert!(!policy.permits("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA", 1));
    }
}
