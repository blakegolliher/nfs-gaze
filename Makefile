NAME := nfs-gaze
VERSION := 1.0.0

.PHONY: all build clean test coverage install uninstall help

all: build

build:
	cargo build --release

test:
	cargo test

clean:
	cargo clean
	rm -rf dist/
	rm -f *.rpm *.deb
	rm -rf coverage/

install: build
	install -D -m 755 target/release/$(NAME) $(DESTDIR)/usr/bin/$(NAME)

uninstall:
	rm -f $(DESTDIR)/usr/bin/$(NAME)

# Coverage targets
coverage:
	@echo "Generating test coverage report..."
	@mkdir -p coverage
	@export PATH="$$HOME/.cargo/bin:$$PATH" && cargo test > /dev/null 2>&1
	@export PATH="$$HOME/.cargo/bin:$$PATH" && ./scripts/coverage.sh
	@echo "Coverage report generated in coverage/README.md"

# Development helpers
dev-deps:
	rustup update
	cargo install cargo-edit

fmt:
	cargo fmt

lint:
	cargo clippy -- -D warnings

# Quick development cycle
dev: fmt test coverage

# Watch for changes and run tests (requires cargo-watch)
watch:
	cargo watch -x test

help:
	@echo "Available targets:"
	@echo "  build        - Build the release binary"
	@echo "  test         - Run tests"
	@echo "  coverage     - Generate test coverage report"
	@echo "  clean        - Remove built files"
	@echo "  install      - Install binary to system"
	@echo "  uninstall    - Remove installed binary"
	@echo "  fmt          - Format code"
	@echo "  lint         - Run clippy linter"
	@echo "  dev          - Format, test, and generate coverage"
	@echo "  watch        - Watch for changes and run tests"
	@echo "  help         - Show this help message"