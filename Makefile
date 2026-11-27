# OpenClawd Agent Kit — iteration shortcuts
#
#   make check     # cargo check, default members
#   make test      # cargo test, default members
#   make all       # check the opt-in members too (kit-evm, kit-http)
#   make demo      # scripted reasoning-loop demo (dry-run, no keys needed)
#   make mcp       # list the fleet MCP catalogs (Clawd Compute + PumpAI)
#   make clippy    # clippy over default members
#   make fmt       # rustfmt the workspace

SHELL := /bin/bash
CARGO ?= cargo

.PHONY: help check test all demo mcp clippy fmt
help: ## Show this help
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-14s\033[0m %s\n", $$1, $$2}'

check: ## cargo check, default members
	$(CARGO) check

test: ## cargo test, default members
	$(CARGO) test

all: ## check opt-in members too
	$(CARGO) check -p kit-evm -p kit-http

demo: ## scripted reasoning-loop demo (dry-run)
	$(CARGO) run -p kit-cli -- demo

mcp: ## list MCP tools on the fleet servers
	$(CARGO) run -p kit-cli -- mcp list

clippy: ## clippy over default members
	$(CARGO) clippy -- -D warnings

fmt: ## format the workspace
	$(CARGO) fmt --all
