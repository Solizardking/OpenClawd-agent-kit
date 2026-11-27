# Tools and the reasoning loop

## The Tool trait

Every capability is a `kit_core::Tool`:

```rust
#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn input_schema(&self) -> Value;      // JSON Schema
    fn consequential(&self) -> bool { false }
    async fn call(&self, arguments: Value) -> Result<Value, ToolError>;
}
```

`consequential` is the load-bearing flag: a tool that moves funds, signs,
trades, sends, deletes, or otherwise changes external state in a way that
is hard to reverse. When in doubt, mark it consequential — an unknown tool
name is also treated as consequential by the registry.

## The reasoning loop

`kit_core::ReasoningLoop` drives any `Reasoner` (an LLM backend, a
scripted planner, a test double) through propose → gate → execute cycles:

1. **Read-only tools** run immediately.
2. **Consequential tools** go to the Jev gate first (below).
3. In **dry-run** (the default), a gate-cleared consequential call is
   *simulated*: the loop records what would have run and feeds a synthetic
   result back. The tool's `call` is never invoked.
4. The loop emits an event transcript (`ToolExecuted`, `ToolSimulated`,
   `ToolDenied`, `Escalated`, `Final`) for audit.

## The Jev gate

For each consequential call, the gate asks TypeSafe Jev two questions in
one System One request, over a state object containing the constitution,
the tool name, its arguments, and the agent's rationale:

- `consequential` — a `noul`: the probability that executing this call
  changes external state irreversibly.
- `verdict` — a `choice` over `proceed` / `deny` / `ask_human`.

Code then decides (thresholds in `crates/kit-core/src/gate.rs`):

| Situation | Decision |
|---|---|
| Jev unconfigured or unreachable | `AskHuman` |
| `consequential` probability < 0.5 | `Proceed` (not actually consequential) |
| `proceed` with probability ≥ 0.6 **and** confidence ≥ 0.6 | `Proceed` |
| `proceed` but weaker | `AskHuman` |
| `deny` with confidence ≥ 0.5 | `Deny` |
| `deny` but weaker | `AskHuman` |
| `ask_human`, or no usable verdict | `AskHuman` |

Escalations go to a `HumanApprover`; the default implementation denies,
so an unattended kit never executes an escalated call. Every evaluation
is appended to an in-memory judgment log and emitted via `tracing`.

## Where tools come from

- `kit-solana` registers `solana_get_balance`,
  `solana_get_token_accounts`, `solana_get_tx_status` (read-only) and
  `solana_transfer` (consequential, only when a signer is provided).
- `kit-mcp` registers the Clawd Compute + PumpAI catalogs, namespaced
  (`clawdcompute__…`, `pumpai__…`), classified from MCP annotations.
- Your own tools: implement the trait, register them, done. The gate and
  the loop treat built-in and custom tools identically.
