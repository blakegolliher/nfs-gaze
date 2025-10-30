NAME := nfs-gaze
VERSION := 0.1.0
RPM_TOPDIR := $(HOME)/rpmbuild
BUILD_NUMBER := $(shell date +%Y%m%d%H%M%S)

.PHONY: all build clean test coverage install uninstall rpm rpm-clean help

all: build

build:
	cargo build --release

test:
	cargo test

clean: rpm-clean
	cargo clean
	rm -rf dist/
	rm -f *.rpm *.deb
	rm -rf coverage/

install: build
	install -D -m 755 target/release/$(NAME) $(DESTDIR)/usr/bin/$(NAME)

uninstall:
	rm -f $(DESTDIR)/usr/bin/$(NAME)

# RPM building targets
rpm-prep:
	@echo "Setting up RPM build environment..."
	@mkdir -p $(RPM_TOPDIR)/{BUILD,RPMS,SOURCES,SPECS,SRPMS}
	@echo "RPM build directories created in $(RPM_TOPDIR)"

rpm-tarball: rpm-prep
	@echo "Creating source tarball..."
	@cd .. && tar czf $(RPM_TOPDIR)/SOURCES/$(NAME)-$(VERSION).tar.gz \
		--transform "s|^nfs-gaze|$(NAME)-$(VERSION)|" \
		--exclude='.git' \
		--exclude='target' \
		--exclude='*.rpm' \
		--exclude='*.deb' \
		--exclude='coverage' \
		nfs-gaze/
	@echo "Source tarball created: $(RPM_TOPDIR)/SOURCES/$(NAME)-$(VERSION).tar.gz"

rpm: rpm-tarball
	@echo "Copying spec file..."
	@cp nfs-gaze.spec $(RPM_TOPDIR)/SPECS/
	@echo "Building RPM package with build number: $(BUILD_NUMBER)"
	@cd $(RPM_TOPDIR) && rpmbuild -ba --define "build_number $(BUILD_NUMBER)" SPECS/nfs-gaze.spec
	@echo ""
	@echo "RPM build complete!"
	@echo "RPMs available at:"
	@find $(RPM_TOPDIR)/RPMS -name "$(NAME)-$(VERSION)-*.rpm" -type f
	@echo ""
	@echo "Source RPM available at:"
	@find $(RPM_TOPDIR)/SRPMS -name "$(NAME)-$(VERSION)-*.src.rpm" -type f

rpm-clean:
	@echo "Cleaning RPM build artifacts..."
	@rm -rf $(RPM_TOPDIR)/BUILD/$(NAME)-$(VERSION)
	@rm -f $(RPM_TOPDIR)/SOURCES/$(NAME)-$(VERSION).tar.gz
	@rm -f $(RPM_TOPDIR)/SPECS/nfs-gaze.spec
	@rm -f $(RPM_TOPDIR)/RPMS/*/$(NAME)-$(VERSION)-*.rpm
	@rm -f $(RPM_TOPDIR)/SRPMS/$(NAME)-$(VERSION)-*.src.rpm

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
	@echo "  clean        - Remove built files and RPM artifacts"
	@echo "  install      - Install binary to system"
	@echo "  uninstall    - Remove installed binary"
	@echo "  rpm          - Build RPM package (CentOS/RHEL/Rocky)"
	@echo "  rpm-clean    - Clean RPM build artifacts"
	@echo "  fmt          - Format code"
	@echo "  lint         - Run clippy linter"
	@echo "  dev          - Format, test, and generate coverage"
	@echo "  watch        - Watch for changes and run tests"
	@echo "  help         - Show this help message"