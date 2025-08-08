SHELL := /usr/bin/bash
CARGO ?= cargo
BIN   ?= sergw

# Optional runtime variables
SERIAL ?=
BAUD   ?= 115200
HOST   ?= 127.0.0.1:5656
ALL    ?=
VERBOSE?=

.PHONY: help build release test clippy fmt run ports listen install clean

help:
	@echo "Available targets:"
	@echo "  build    - Build debug"
	@echo "  release  - Build release"
	@echo "  test     - Run tests"
	@echo "  clippy   - Lint with clippy (deny warnings)"
	@echo "  fmt      - Format code"
	@echo "  ports    - Run 'ports' subcommand (ALL=1 VERBOSE=1)"
	@echo "  listen   - Run 'listen' (SERIAL=/dev/ttyUSB0 BAUD=115200 HOST=127.0.0.1:5656)"
	@echo "  install  - Cargo install this crate"
	@echo "  clean    - Cargo clean"

build:
	$(CARGO) build

release:
	$(CARGO) build --release

test:
	$(CARGO) test

clippy:
	$(CARGO) clippy --all-targets -- -D warnings

fmt:
	$(CARGO) fmt --all

ports:
	@set -e; \
	ARGS="ports"; \
	if [ -n "$(ALL)" ]; then ARGS="$$ARGS --all"; fi; \
	if [ -n "$(VERBOSE)" ]; then ARGS="$$ARGS --verbose"; fi; \
	$(CARGO) run -- $$ARGS

listen:
	@set -e; \
	ARGS="listen --baud $(BAUD) --host $(HOST)"; \
	if [ -n "$(SERIAL)" ]; then ARGS="$$ARGS --serial $(SERIAL)"; fi; \
	$(CARGO) run -- $$ARGS

install:
	$(CARGO) install --path . --locked

clean:
	$(CARGO) clean
