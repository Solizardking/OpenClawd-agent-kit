# Quickstart

## 1. Environment

```bash
cp .env.example .env
```

Set at minimum:

- `TYPESAFE_API_KEY` — without it the Jev gate escalates every
  consequential call to a human (and the default approver denies). That
  is a safe way to explore, but the demo below is more interesting with
  the gate live.
- `SOLANA_RPC_URL` — a private endpoint (Helius, QuikNode) if you have
  one; the public mainnet endpoint is the fallback.

## 2. The demo (no funds, no keys)

```bash
cargo run -p kit-cli -- demo
```

A scripted reasoner proposes two calls: a balance read (runs immediately)
and a SOL transfer. The transfer is consequential, so it goes through the
Jev gate; the loop is in dry-run, so even a `Proceed` verdict results in a
*simulated* execution. Watch the events: `ToolExecuted` for the read,
`ToolSimulated` (or an escalation) for the transfer.

## 3. Browse the fleet's MCP tools

```bash
cargo run -p kit-cli -- mcp list
```

Initializes against the Clawd Compute gateway and PumpAI and registers
their catalogs: **272 + 213 tools** (live-verified 2026-11-27). Call a
read-only one:

```bash
cargo run -p kit-cli -- mcp call pumpai__get_health '{}'
```

## 4. Solana reads and unsigned builds

```bash
cargo run -p kit-cli -- solana balance <pubkey>
cargo run -p kit-cli -- solana transfer-build <from> <to> <lamports>
```

`transfer-build` prints a base64 wire-format transaction. Signing it is
your wallet's job — the kit never holds keys by default.

## 5. Check the gate directly

```bash
cargo run -p kit-cli -- gate check solana_transfer \
  '{"to":"ComputeProvider","lamports":5000}' "pay for compute"
```

Prints Jev's judgment and the decision the *code* derived from it,
including the policy rule that fired.

## 6. Build everything

```bash
cargo check            # default members
cargo test             # default members
cargo check -p kit-evm -p kit-http   # opt-in members
```
