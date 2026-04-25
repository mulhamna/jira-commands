.PHONY: help build release check fmt fmt-check lint test test-core test-cli test-mcp \
        audit smoke run run-tui mcp install uninstall clean doc doc-open \
        plugin-version winget-check changelog deps-update tree

CARGO ?= cargo
BIN   ?= jirac
PROFILE ?= release
PREFIX ?= /usr/local

help:
	@echo "jirac — common workspace targets"
	@echo ""
	@echo "Build:"
	@echo "  make build           Debug build (workspace)"
	@echo "  make release         Release build (workspace)"
	@echo "  make install         Install jirac to \$$PREFIX/bin (default /usr/local/bin)"
	@echo "  make uninstall       Remove installed jirac binary"
	@echo ""
	@echo "Quality:"
	@echo "  make fmt             Format all crates"
	@echo "  make fmt-check       Check formatting (no write)"
	@echo "  make lint            Clippy with -D warnings"
	@echo "  make test            Run all tests"
	@echo "  make test-core       Tests for jira-core"
	@echo "  make test-cli        Tests for jira (CLI)"
	@echo "  make test-mcp        Tests for jira-mcp"
	@echo "  make audit           cargo audit (security)"
	@echo "  make smoke           fmt-check + lint + test + build (matches CI gate)"
	@echo ""
	@echo "Run:"
	@echo "  make run ARGS=...    Run jirac with ARGS (e.g. make run ARGS='issue list')"
	@echo "  make run-tui P=PROJ  Launch TUI for project key"
	@echo "  make mcp             Run jirac-mcp server (stdio)"
	@echo ""
	@echo "Docs:"
	@echo "  make doc             Build rustdoc"
	@echo "  make doc-open        Build + open rustdoc in browser"
	@echo ""
	@echo "Misc:"
	@echo "  make clean           cargo clean"
	@echo "  make tree            Workspace dep tree"
	@echo "  make deps-update     Update Cargo.lock"
	@echo "  make changelog       Show recent CHANGELOG entries"

build:
	$(CARGO) build --workspace --all-targets

release:
	$(CARGO) build --workspace --release

check:
	$(CARGO) check --workspace --all-targets --all-features

fmt:
	$(CARGO) fmt --all

fmt-check:
	$(CARGO) fmt --all -- --check

lint:
	$(CARGO) clippy --workspace --all-targets --all-features -- -D warnings

test: test-core test-cli test-mcp

test-core:
	$(CARGO) test -p jira-core

test-cli:
	$(CARGO) test -p jira-commands

test-mcp:
	$(CARGO) test -p jira-mcp

audit:
	$(CARGO) audit

smoke: fmt-check lint test build
	@echo "Smoke OK."

run:
	$(CARGO) run -p jira-commands -- $(ARGS)

run-tui:
	@if [ -z "$(P)" ]; then \
		$(CARGO) run -p jira-commands -- tui; \
	else \
		$(CARGO) run -p jira-commands -- tui -p $(P); \
	fi

mcp:
	$(CARGO) run -p jira-mcp

install: release
	@install -d $(PREFIX)/bin
	@install -m 0755 target/release/$(BIN) $(PREFIX)/bin/$(BIN)
	@echo "Installed $(BIN) → $(PREFIX)/bin/$(BIN)"

uninstall:
	@rm -f $(PREFIX)/bin/$(BIN)
	@echo "Removed $(PREFIX)/bin/$(BIN)"

doc:
	$(CARGO) doc --workspace --no-deps

doc-open:
	$(CARGO) doc --workspace --no-deps --open

clean:
	$(CARGO) clean

tree:
	$(CARGO) tree --workspace

deps-update:
	$(CARGO) update --workspace

changelog:
	@head -60 CHANGELOG.md 2>/dev/null || echo "No CHANGELOG.md at repo root."
