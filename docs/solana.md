# Solana

`kit-solana` is built on the modern split-crate SDK
(`solana-sdk` / `solana-rpc-client` / `solana-rpc-client-api` 3.x) —
no monolith, no web3.js-era types.

## Reads

`SolanaClient` (from `SOLANA_RPC_URL`, or any endpoint you pass):

- `balance(pubkey)` → lamports
- `token_accounts(owner)` → SPL token accounts (mint, raw + UI amounts),
  parsed from the RPC's jsonParsed encoding
- `signature_status(signature)` → slot, confirmations, error,
  confirmation status
- `latest_blockhash()`, `slot()`, `get_account(pubkey)`

## Building transfers

```rust
let tx = kit_solana::transfer::build_transfer(&payer, &to, lamports, blockhash);
let base64 = kit_solana::transfer::encode_tx(&tx)?;   // canonical wire format
```

Transactions are built **unsigned**. That is a design rule, not a missing
feature: the kit constructs, something the operator controls signs.

## Signers

`SolanaSigner` is the abstraction; `SignerContext` (task-local, carried
from the original kit) makes the current signer available to tools
without threading it through every call.

| Signer | Custody | Notes |
|---|---|---|
| `UnsignedSigner` | none (pubkey only) | Default for read/build flows. Cannot sign; a transaction it "signs" fails cluster verification — fail-closed. |
| `RemoteSigner` | operator's endpoint | POSTs base64 transactions/messages to an operator-run signer (optional bearer via `KIT_REMOTE_SIGNER_TOKEN`). |
| `LocalKeypairSigner` | local keypair | The original kit's flow, behind the `local-signer` feature. Never constructed implicitly. |

The one broadcast path is `SolanaClient::send_transaction`, reached by
the `solana_transfer` tool only after the Jev gate proceeds, dry-run is
off, and a real signer produced a signature.

## State proofs and the Lean seam

`kit_solana::verify` defines:

- `StateProof` — produce/check an `AccountProof` (account digest at a slot).
- `LightVerify` — the interface a Solana **Lean client** backend
  implements to verify state against validator-voted roots.
- `RpcStateProof` — today's implementation: RPC reads + digest compare.
  It detects drift; it is **not** cryptographic inclusion verification
  and is never described as such.

See [Lean](./lean.md) and the [ohr note](./ohr.md).
