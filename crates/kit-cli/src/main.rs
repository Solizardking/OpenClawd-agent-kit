//! kit — the OpenClawd agent kit CLI.
//!
//! Subcommands:
//!   kit constitution                     print the Clawd constitution preamble
//!   kit mcp list [--server NAME]         initialize + list tools on the MCP servers
//!   kit mcp call <server__tool> <json>   call an MCP tool (spend policy applies)
//!   kit solana balance <pubkey>          SOL balance via RPC (SOLANA_RPC_URL)
//!   kit solana transfer-build <from> <to> <lamports>
//!                                        build an UNSIGNED transfer, print base64
//!   kit gate check <tool> <json>         run the Jev gate on a hypothetical call
//!   kit demo                             scripted reasoning-loop demo (dry-run)
//!
//! Everything consequential is fail-closed by default: the gate needs
//! TYPESAFE_API_KEY, the MCP spend policy is read-only, and `demo` runs in
//! dry-run, so this binary cannot move funds out of the box.

use std::sync::Arc;

use anyhow::{Context, Result};
use serde_json::{json, Value};

use kit_core::{
    GateDecision, ReasoningLoop, ReasonerStep, ScriptedReasoner, Tool, ToolCallIntent,
    ToolError, ToolRegistry,
};
use kit_mcp::{McpClient, SpendPolicy};

fn rpc_url() -> String {
    std::env::var("SOLANA_RPC_URL").unwrap_or_else(|_| "https://api.mainnet-beta.solana.com".to_string())
}

fn gate_json(record: &kit_core::JudgmentRecord) -> Value {
    json!({
        "tool": record.tool,
        "decision": format!("{:?}", record.decision),
        "reason": record.reason,
        "consequential_probability": record.consequential_probability,
        "verdict_choice": record.verdict_choice,
        "verdict_confidence": record.verdict_confidence,
    })
}

async fn cmd_mcp_list(server_filter: Option<String>) -> Result<()> {
    let mut registry = ToolRegistry::new();
    let clients = McpClient::fleet();
    for client in &clients {
        if let Some(f) = &server_filter
            && &client.namespace != f {
                continue;
            }
        match client.initialize().await {
            Ok(info) => println!(
                "== {} — {} v{} — protocol {}",
                client.namespace, info.name, info.version, info.protocol_version
            ),
            Err(e) => println!("== {} — initialize failed: {e}", client.namespace),
        }
    }
    let counts = kit_mcp::register_mcp_servers(&mut registry, &clients, SpendPolicy::default()).await;
    for (ns, n) in &counts {
        println!("{ns}: {n} tools");
    }
    println!("total registered: {}", registry.len());
    Ok(())
}

async fn cmd_mcp_call(namespaced: &str, args_json: &str) -> Result<()> {
    let arguments: Value = serde_json::from_str(args_json).context("arguments must be JSON")?;
    let mut registry = ToolRegistry::new();
    let clients = McpClient::fleet();
    for client in &clients {
        let _ = client.initialize().await;
    }
    kit_mcp::register_mcp_servers(&mut registry, &clients, SpendPolicy::default()).await;
    let intent = ToolCallIntent {
        tool: namespaced.to_string(),
        arguments,
        rationale: "manual CLI call".to_string(),
    };
    // Route through the same rule as the loop: consequential ⇒ gate first.
    if registry.is_consequential(namespaced) {
        let gate = kit_core::gate_from_env();
        let record = gate.evaluate(&intent).await;
        println!("gate: {}", serde_json::to_string_pretty(&gate_json(&record))?);
        if record.decision != GateDecision::Proceed {
            println!("not executed (gate decision: {:?})", record.decision);
            return Ok(());
        }
    }
    match registry.call(&intent).await {
        Ok(v) => println!("{}", serde_json::to_string_pretty(&v)?),
        Err(e) => println!("error: {e}"),
    }
    Ok(())
}

async fn cmd_gate_check(tool: &str, args_json: &str, rationale: &str) -> Result<()> {
    let arguments: Value = serde_json::from_str(args_json).context("arguments must be JSON")?;
    let gate = kit_core::gate_from_env();
    println!("jev configured: {}", gate.configured());
    let record = gate
        .evaluate(&ToolCallIntent {
            tool: tool.to_string(),
            arguments,
            rationale: rationale.to_string(),
        })
        .await;
    println!("{}", serde_json::to_string_pretty(&gate_json(&record))?);
    Ok(())
}

/// A consequential demo tool: stands in for a transfer. Its `call` would
/// move funds in a real deployment; the demo never gets there (dry-run +
/// fail-closed gate), which is exactly what the demo demonstrates.
struct DemoTransfer;

#[async_trait::async_trait]
impl Tool for DemoTransfer {
    fn name(&self) -> &str {
        "solana_transfer"
    }
    fn description(&self) -> &str {
        "Transfer SOL to another wallet (consequential)"
    }
    fn input_schema(&self) -> Value {
        json!({"type": "object", "properties": {"to": {"type": "string"}, "lamports": {"type": "integer"}}})
    }
    fn consequential(&self) -> bool {
        true
    }
    async fn call(&self, _arguments: Value) -> Result<Value, ToolError> {
        Ok(json!({"signature": "demo-signature"}))
    }
}

struct DemoBalance;

#[async_trait::async_trait]
impl Tool for DemoBalance {
    fn name(&self) -> &str {
        "solana_get_balance"
    }
    fn description(&self) -> &str {
        "Read a wallet balance (read-only)"
    }
    fn input_schema(&self) -> Value {
        json!({"type": "object", "properties": {"pubkey": {"type": "string"}}})
    }
    async fn call(&self, _arguments: Value) -> Result<Value, ToolError> {
        Ok(json!({"lamports": 42000}))
    }
}

async fn cmd_demo() -> Result<()> {
    let mut registry = ToolRegistry::new();
    registry.register(Arc::new(DemoBalance));
    registry.register(Arc::new(DemoTransfer));
    let reasoner = Arc::new(ScriptedReasoner::new(vec![
        ReasonerStep::CallTool(ToolCallIntent {
            tool: "solana_get_balance".to_string(),
            arguments: json!({"pubkey": "DemoWallet"}),
            rationale: "check funds before paying".to_string(),
        }),
        ReasonerStep::CallTool(ToolCallIntent {
            tool: "solana_transfer".to_string(),
            arguments: json!({"to": "ComputeProvider", "lamports": 5000}),
            rationale: "pay for compute per the constitution (Law II)".to_string(),
        }),
    ]));
    let loopy = ReasoningLoop::new(reasoner, Arc::new(registry), kit_core::gate_from_env());
    println!("dry_run: {} (default)", loopy.dry_run);
    let events = loopy.run("pay for compute").await?;
    for event in &events {
        println!("{event:?}");
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let cmd: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    match cmd.as_slice() {
        ["constitution"] => {
            println!("{}", kit_core::constitution::clawd_system_preamble());
        }
        ["mcp", "list"] => cmd_mcp_list(None).await?,
        ["mcp", "list", "--server", name] => cmd_mcp_list(Some(name.to_string())).await?,
        ["mcp", "call", name, json_args] => cmd_mcp_call(name, json_args).await?,
        ["solana", "balance", pubkey] => {
            let client = kit_solana::SolanaClient::new(rpc_url());
            let pk: kit_solana::Pubkey = pubkey.parse().context("invalid pubkey")?;
            let lamports = client.balance(&pk).await?;
            println!("{lamports} lamports ({} SOL)", lamports as f64 / 1e9);
        }
        ["solana", "transfer-build", from, to, lamports] => {
            let client = kit_solana::SolanaClient::new(rpc_url());
            let from_pk: kit_solana::Pubkey = from.parse().context("invalid from pubkey")?;
            let to_pk: kit_solana::Pubkey = to.parse().context("invalid to pubkey")?;
            let lamports: u64 = lamports.parse().context("lamports must be an integer")?;
            let blockhash = client.latest_blockhash().await?;
            let tx = kit_solana::transfer::build_transfer(&from_pk, &to_pk, lamports, blockhash);
            println!("unsigned transaction (base64): {}", kit_solana::transfer::encode_tx(&tx)?);
            println!("sign this with your wallet / remote signer — the kit never holds keys by default");
        }
        ["gate", "check", tool, json_args] => cmd_gate_check(tool, json_args, "manual CLI check").await?,
        ["gate", "check", tool, json_args, rationale] => cmd_gate_check(tool, json_args, rationale).await?,
        ["demo"] => cmd_demo().await?,
        _ => {
            eprintln!(
                "usage:\n  kit constitution\n  kit mcp list [--server NAME]\n  kit mcp call <server__tool> <json-args>\n  kit solana balance <pubkey>\n  kit solana transfer-build <from> <to> <lamports>\n  kit gate check <tool> <json-args> [rationale]\n  kit demo"
            );
            std::process::exit(2);
        }
    }
    Ok(())
}
