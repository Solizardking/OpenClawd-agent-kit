//! kit-lean — the OpenClawd agent kit's formal-methods bridge.
//!
//! This crate holds *executable models* of the kit's core safety
//! invariants (spend caps, the Jev-gate judgment requirement, signer
//! policy). The same invariants are stated in Lean in the repo's `lean/`
//! package, pinned against the Solana Foundation's solanalib, so proofs
//! can replace tests as the Lean toolchain matures. See `docs/lean.md`.

pub mod invariants;

pub use invariants::{CallState, Decision, GateError, GateLog, SignerPolicy, SpendLedger};
