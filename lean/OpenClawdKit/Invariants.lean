/-
  OpenClawdKit.Invariants — the kit's core safety invariants, stated in Lean.

  Each definition mirrors the executable Rust model in
  `crates/kit-lean/src/invariants.rs` (same names, same transition rules),
  and each theorem is the property the Rust property tests exercise.

  STATUS: scaffold. The `SpendState` theorems are proved below (they are
  small arithmetic facts). The `GateLog` and `SignerPolicy` statements are
  stated with `sorry` placeholders — they are the next proof targets, and
  the `sorry`s are deliberate and visible: nothing in this file should be
  cited as "formally verified" until they are discharged.

  When these models grow toward real Solana state, they will be restated
  over solanalib's runtime model (`Solanalib.Runtime`) — that is the path
  from kit invariants to Lean-client-verified state (see `docs/lean.md`).
-/

namespace OpenClawdKit

/-! ## Spend cap (mirrors Rust `SpendLedger`) -/

/-- A spend ledger: a hard cap and the cumulative authorized spend,
    in the smallest unit (micro-USDC, lamports, …). -/
structure SpendState where
  cap : Nat
  spent : Nat
  deriving DecidableEq, Repr

/-- Authorize a spend. Succeeds (returning the new state) iff the
    cumulative spend stays within the cap; otherwise the state is
    returned unchanged — authorization is atomic. -/
def SpendState.authorize (s : SpendState) (amount : Nat) : SpendState × Bool :=
  if s.spent + amount ≤ s.cap then
    ({ s with spent := s.spent + amount }, true)
  else
    (s, false)

/-- Spend is monotone: a successful authorization never decreases the
    cumulative spend, and a failed one changes nothing. -/
theorem SpendState.spend_monotone (s : SpendState) (amount : Nat) :
    s.spent ≤ (s.authorize amount).1.spent := by
  unfold SpendState.authorize
  split
  · exact Nat.le_add_right _ _
  · exact Nat.le_refl _

/-- The cap is respected: starting from a state within its cap, every
    authorization sequence stays within the cap. -/
theorem SpendState.cap_respected (s : SpendState) (amount : Nat)
    (h : s.spent ≤ s.cap) :
    (s.authorize amount).1.spent ≤ s.cap := by
  unfold SpendState.authorize
  split
  · rename_i hle
    exact hle
  · exact h

/-! ## The Jev gate (mirrors Rust `GateLog`) -/

/-- The gate's decision on a consequential call. -/
inductive Decision where
  | proceed
  | deny
  | askHuman
  deriving DecidableEq, Repr

/-- A consequential call's lifecycle. `executed` is reachable only from
    `judged .proceed` — that reachability fact IS the kit's central
    safety invariant: no consequential call executes without a prior
    Jev judgment. -/
inductive CallState where
  | proposed
  | judged (d : Decision)
  | executed
  | refused
  deriving DecidableEq, Repr

/-- One gate transition. -/
inductive GateStep : CallState → CallState → Prop where
  | judge (d : Decision) : GateStep .proposed (.judged d)
  | exec : GateStep (.judged .proceed) .executed
  | refuse : GateStep .proposed .refused
  | refuseAfter (d : Decision) : GateStep (.judged d) .refused

/-- **The gate invariant (statement).** If a call reached `executed`,
    then at some point it was in state `judged .proceed` — execution
    without a prior proceed judgment is impossible.

    Proof target #1: induction over the reflexive-transitive closure of
    `GateStep`. Scaffolded with `sorry` for now. -/
theorem GateLog.no_execution_without_judgment :
    ∀ (trace : List CallState),
      trace.head? = some .proposed →
      trace.getLast? = some .executed →
      (.judged .proceed) ∈ trace := by
  sorry

/-! ## Signer policy (mirrors Rust `SignerPolicy`) -/

/-- Which programs a signer may interact with, and the most value one
    transaction may move. An empty allowlist permits nothing. -/
structure SignerPolicy where
  allowedPrograms : List String
  maxLamportsPerTx : Nat
  deriving DecidableEq, Repr

def SignerPolicy.permits (p : SignerPolicy) (program : String) (lamports : Nat) : Bool :=
  lamports ≤ p.maxLamportsPerTx && p.allowedPrograms.contains program

/-- **Fail-closed (statement).** A policy with an empty allowlist permits
    no program at any amount. -/
theorem SignerPolicy.empty_permits_nothing (maxLamports : Nat)
    (program : String) (lamports : Nat) :
    (SignerPolicy.mk [] maxLamports).permits program lamports = false := by
  sorry

end OpenClawdKit
