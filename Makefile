# Daystrom TUI - Rust Edition Makefile

# Variables
BINARY_NAME = daystrom-tui
CARGO = cargo
RUSTC = rustc
TARGET_DIR = target
RELEASE_DIR = $(TARGET_DIR)/release
DEBUG_DIR = $(TARGET_DIR)/debug

# Default target
.PHONY: all
all: build

# Build targets
.PHONY: build
build:
	$(CARGO) build

.PHONY: release
release:
	$(CARGO) build --release

.PHONY: clean
clean:
	$(CARGO) clean

# Run targets
.PHONY: run
run: build
	$(CARGO) run

.PHONY: run-release
run-release: release
	./$(RELEASE_DIR)/$(BINARY_NAME)

.PHONY: run-config
run-config: build
	$(CARGO) run -- --config config.yaml

# Development targets
.PHONY: dev
dev:
	$(CARGO) run -- --log-level debug

.PHONY: watch
watch:
	$(CARGO) watch -x run

.PHONY: watch-test
watch-test:
	$(CARGO) watch -x test

# Testing targets
.PHONY: test
test:
	$(CARGO) test

.PHONY: test-release
test-release:
	$(CARGO) test --release

.PHONY: test-verbose
test-verbose:
	$(CARGO) test -- --nocapture

# Code quality targets
.PHONY: check
check:
	$(CARGO) check

.PHONY: clippy
clippy:
	$(CARGO) clippy

.PHONY: clippy-release
clippy-release:
	$(CARGO) clippy --release

.PHONY: fmt
fmt:
	$(CARGO) fmt

.PHONY: fmt-check
fmt-check:
	$(CARGO) fmt -- --check

# Documentation targets
.PHONY: doc
doc:
	$(CARGO) doc

.PHONY: doc-open
doc-open:
	$(CARGO) doc --open

# Installation targets
.PHONY: install
install: release
	cp $(RELEASE_DIR)/$(BINARY_NAME) /usr/local/bin/

.PHONY: uninstall
uninstall:
	rm -f /usr/local/bin/$(BINARY_NAME)

# Docker targets
.PHONY: docker-build
docker-build:
	docker build -t daystrom-tui .

.PHONY: docker-run
docker-run:
	docker run -it --rm \
		-v $(PWD)/config.yaml:/app/config.yaml:ro \
		daystrom-tui

.PHONY: docker-shell
docker-shell:
	docker run -it --rm \
		-v $(PWD):/app \
		-w /app \
		daystrom-tui /bin/bash

# Utility targets
.PHONY: help
help:
	@echo "Daystrom TUI - Rust Edition"
	@echo ""
	@echo "Available targets:"
	@echo "  build          - Build in debug mode"
	@echo "  release        - Build in release mode"
	@echo "  clean          - Clean build artifacts"
	@echo "  run            - Run in debug mode"
	@echo "  run-release    - Run release binary"
	@echo "  run-config     - Run with config file"
	@echo "  dev            - Run with debug logging"
	@echo "  watch          - Run with file watching"
	@echo "  test           - Run tests"
	@echo "  test-verbose   - Run tests with output"
	@echo "  check          - Check code without building"
	@echo "  clippy         - Run clippy linter"
	@echo "  fmt            - Format code"
	@echo "  fmt-check      - Check code formatting"
	@echo "  doc            - Generate documentation"
	@echo "  install        - Install to /usr/local/bin"
	@echo "  uninstall      - Remove from /usr/local/bin"
	@echo "  docker-build   - Build Docker image"
	@echo "  docker-run     - Run Docker container"
	@echo "  help           - Show this help"

.PHONY: size
size: release
	@echo "Binary size:"
	@ls -lh $(RELEASE_DIR)/$(BINARY_NAME)

.PHONY: deps
deps:
	@echo "Installing development dependencies..."
	$(CARGO) install cargo-watch
	$(CARGO) install cargo-audit

.PHONY: audit
audit:
	$(CARGO) audit

.PHONY: update
update:
	$(CARGO) update

.PHONY: outdated
outdated:
	$(CARGO) outdated

# Default target
.DEFAULT_GOAL := help 