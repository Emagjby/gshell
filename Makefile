SHELL := /bin/sh

CARGO ?= cargo
CRATE ?= gshell

.PHONY: help
help:
	@printf "\n"
	@printf "Targets:\n"
	@printf "  make check        - cargo check\n"
	@printf "  make test         - cargo test\n"
	@printf "  make fmt          - cargo fmt\n"
	@printf "  make fmt-check    - cargo fmt --check\n"
	@printf "  make clippy       - cargo clippy with warnings denied\n"
	@printf "  make lint         - fmt-check + clippy\n"
	@printf "  make validate     - check + test + lint\n"
	@printf "  make run          - cargo run\n"
	@printf "  make clean        - cargo clean\n"
	@printf "\n"

.PHONY: check
check:
	$(CARGO) check --all-targets --all-features

.PHONY: test
test:
	$(CARGO) test --all-targets --all-features

.PHONY: fmt
fmt:
	$(CARGO) fmt --all

.PHONY: fmt-check
fmt-check:
	$(CARGO) fmt --all --check

.PHONY: clippy
clippy:
	$(CARGO) clippy --all-targets --all-features -- -D warnings

.PHONY: lint
lint: fmt-check clippy

.PHONY: validate
validate: check test lint

.PHONY: run
run:
	$(CARGO) run --bin $(CRATE)

.PHONY: clean
clean:
	$(CARGO) clean
