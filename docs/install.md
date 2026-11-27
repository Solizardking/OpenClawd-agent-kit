# Install

## The one-liner

```bash
curl -fsSL https://clawdcompute.us/install.sh | sh
```

That is the whole install story for this kit. The script is POSIX `sh`
(no bash required), needs no sudo, never prompts for a secret, writes
only inside your `HOME`, and is idempotent — re-running it converges to
the same state. Prefer to read before you run:
[install.sh](../install.sh) is short, and the bytes served at
`clawdcompute.us/install.sh` are exactly the file in this repository.

Running `./install.sh` from a clone of this repo does the same thing
using your checkout instead of downloading one.

## What it does

1. **Preflight.** Linux or macOS; finds `cargo` (or says exactly how to
   install Rust via rustup and stops); finds `curl` for verification.
2. **Source.** From a clone it uses the checkout; piped, it clones the
   kit into `~/.openclawd/agent-kit` (and fast-forwards it on re-runs).
3. **The `kit` CLI.** `cargo install --path crates/kit-cli --locked`
   into `~/.local/bin`, using the toolchain pinned by
   `rust-toolchain.toml`.
4. **Skills.** Copies the three bundled skills — `solana-dev`,
   `certora-solana-fv`, `harmonic-aristotle` — into both
   `~/.agents/skills/` and `~/.claude/skills/`, the two locations the
   agent ecosystem reads (Claude Code, Codex, Copilot, Gemini CLI,
   Cursor, Windsurf, Cline, OpenCode…).
5. **Clawd Connectors.** Clones and builds
   [clawd-connectors-2](https://github.com/Solizardking/clawd-connectors-2)
   under `~/.openclawd/clawd-connectors` (its MCP server,
   `clawd-connectors-mcp`, is not on npm yet, so the clone path is the
   install path). Skipped with a loud warning if Node.js is absent —
   the hosted gateway below still works.
6. **MCP connection.** Merges four servers into `~/.claude.json`
   (`mcpServers`), preserving every entry you already have:
   `clawdcompute` (hosted gateway, https://mcp.clawdcompute.us/mcp),
   `pumpai` (https://pumpai.ai/mcp), `solana-dev`
   (https://mcp.solana.com/mcp), and `clawd-connectors` (the local
   stdio server from step 5). On macOS the Claude Desktop config is
   merged too. A standalone copy lands in `~/.openclawd/mcp.json` for
   any other client.
7. **Verification — before it declares success.** `kit constitution`
   runs; a raw MCP `initialize` handshake to the gateway must answer
   with the `clawd-compute-mcp-portal` identity; `kit mcp list` must
   list the fleet with a non-zero Clawd Compute tool count (272 at the
   time of writing); every skill's `SKILL.md` must exist in both
   locations; the config must contain the gateway entry. Any failure
   exits non-zero with a plain-language message naming what failed.

After it finishes: `kit mcp list` shows the fleet (490 tools across
the three hosted servers at the time of writing), `kit demo` runs the
gated reasoning loop in dry-run.

## Why not the upstream installer?

The kit vendors the Solana Foundation's `solana-dev` skill
byte-faithful (see [Solana.ai resources](./solana-ai.md)). Before
building this installer we tested the upstream distribution paths end
to end in a sandbox `$HOME` (2026-11-27, upstream
`solana-foundation/solana-dev-skill` @ `c9da2624`). The skill itself is
fine — its validator passes — but the install paths break in exactly
the situations an installer is for:

| Path | Result | Evidence |
| --- | --- | --- |
| `./install.sh` from a clone, under `bash` | **Works** (exit 0) | Installs into `~/.agents/skills` + `~/.claude/skills`; a non-interactive re-run overwrites cleanly; `--project` and `--path` also work |
| `sh install.sh` (dash / POSIX `sh`) | **Broken** (exit 2) | `./install.sh: 15: Bad substitution`, `./install.sh: 24: [[: not found`, `./install.sh: 72: Syntax error: "(" unexpected` — the script is bash-only (arrays, `[[ ]]`, `${BASH_SOURCE}`) despite the `.sh` name |
| `curl … \| sh` / piped one-liner | **Impossible** (exit 1) | The script resolves its payload relative to its own path (`SCRIPT_DIR/skills/solana-dev`); piped, there is no clone around it: `Error: SKILL.md not found in '<cwd>/skills/solana-dev'`. There is no remote one-liner at all |
| `npx skills add solana-foundation/solana-dev-skill` (the documented quick install) | **Silently installs nothing** in a non-interactive shell | The CLI clones the repo, finds the skill, then stops at an interactive *Installation scope* prompt; with stdin closed it exits **0** having installed nothing (no project directory created, `$HOME` untouched) |
| `npx skills add … -y -g` (the CLI's own flags) | **Works** | Installs to `~/.agents/skills/solana-dev` — but those flags are not in the documented command |
| `python3 scripts/validate-skill.py skills/solana-dev` | **Works** (exit 0) | `Skill is valid!` |
| `bash scripts/validate-skills.sh` | **Works** (exit 0) | `ok skills/solana-dev` |
| `sh scripts/validate-skills.sh` | **Broken** (exit 2) | `scripts/validate-skills.sh: 17: set: Illegal option -o pipefail` |

Two smaller notes: the shebang is `#!/bin/bash` (not
`#!/usr/bin/env bash`), so the script also fails on systems without
`/bin/bash`; and even when it works, the upstream installer installs
one skill and nothing else — no CLI, no MCP configuration, no
connection to anything.

Our installer differs on each point: POSIX `sh` throughout; works
piped because it fetches its own payload; installs the CLI, all three
skills, and the connectors server; writes the MCP config (merged, not
clobbered); and verifies the live gateway connection before declaring
success — a failed check is a non-zero exit with the failing step
named, never a silent no-op.

## Verification transcript (this repo, 2026-11-27)

Tested end to end on the development VM in a throwaway sandbox
`$HOME` (the real `$HOME` was never touched), twice in a row — the
second run is the idempotency check — plus a third run in piped mode
(`sh < install.sh` from an unrelated directory, kit repo pointed at
the local checkout via `OPENCLAWD_KIT_REPO`):

<!-- VERIFY-SUMMARY -->
Verified end-to-end on 2026-11-27 in throwaway sandbox HOMEs on the
build VM (never the real HOME). Environment notes first, because two
findings are about the test VM, not about machines in general:

- This VM's egress is a MITM proxy plus recurring stall waves. cargo's
  own HTTP fetches starved (0 bytes for minutes at a time) while plain
  `curl` worked, so the crate cache for the test was seeded by
  downloading every locked `.crate` with `curl` (sha256 verified
  against `Cargo.lock`) and exposing it to cargo as a directory source
  via the sandbox `CARGO_HOME` config. cargo verified every checksum
  itself; `install.sh` is unchanged by this — on a normal network it
  fetches from crates.io directly.
- Node/npm do not trust the VM proxy's CA chain
  (`SELF_SIGNED_CERT_IN_CHAIN` on every tarball), so the test env set
  `NODE_EXTRA_CA_CERTS` to the system CA bundle. A user machine
  without a MITM proxy needs neither workaround.

Runs (local-checkout mode unless noted):

1. **Cold run** — CLI built and installed (`Finished release profile
   in 9m 51s`, `kit` at `~/.local/bin/kit`), all three skills copied
   to both locations. Then npm crashed on the proxy CA issue above
   and the installer stopped with `install: error: could not build
   Clawd Connectors` and exit 1 — the failure was named, nothing was
   half-registered.
2. **Wedged-state runs** — the crashed npm left a half-populated
   `node_modules` that the next runs could not repair (`ENOTEMPTY` on
   rename). This was a real installer bug, fixed in commit `1ae7773`:
   on connectors-install failure the installer now clears
   `node_modules` and retries once.
3. **Full success run** — the retry fired live (`added 100 packages`,
   `tsc` build OK), MCP config merged into `~/.claude.json`
   (clawdcompute, pumpai, solana-dev over HTTP + clawd-connectors over
   stdio), and every verification passed: gateway handshake against
   `clawd-compute-mcp-portal v1.0.0`, `kit mcp list` showing
   **clawdcompute 272 tools, pumpai 213, solana 5 — 490 total**, all
   skills on disk, exit 0.
4. **Idempotency run** — same HOME again: build converged in ~1s,
   npm reported `up to date`, the merged config was identical, all
   verifications passed again, exit 0.
5. **Piped one-liner run** — fresh HOME, unrelated working directory,
   `sh < install.sh` (the `curl … | sh` shape): the installer cloned
   the kit into `~/.openclawd/agent-kit`, built it, and completed
   with the same 490-tool verification, exit 0.

The hosted copy is kept byte-identical to this repo's `install.sh`:
after the `1ae7773` fix, the gateway's embedded copy was refreshed
and `curl https://clawdcompute.us/install.sh | cmp - install.sh`
passes against the file at the top of this repo.
<!-- /VERIFY-SUMMARY -->
