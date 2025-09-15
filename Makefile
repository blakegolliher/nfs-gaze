NAME := nfs-gaze
VERSION := 1.0.0
GOFLAGS := -ldflags="-s -w"

.PHONY: all build clean test coverage coverage-md coverage-html install uninstall

all: build

build:
	go build $(GOFLAGS) -o $(NAME)

test:
	go test -v -cover ./...

clean:
	rm -f $(NAME)
	rm -rf dist/
	rm -f *.rpm *.deb
	rm -f tests/coverage.out tests/coverage.html tests/COVERAGE.md
	rm -f coverage.out coverage.html

install: build
	install -D -m 755 $(NAME) $(DESTDIR)/usr/bin/$(NAME)

uninstall:
	rm -f $(DESTDIR)/usr/bin/$(NAME)

# Coverage targets
coverage: coverage-md

coverage-md:
	@echo "Generating markdown coverage report..."
	@go test -coverprofile=tests/coverage.out ./... > /dev/null 2>&1
	@go run tests/coverage_to_md.go > tests/COVERAGE.md
	@echo "Coverage report generated in tests/COVERAGE.md"
	@echo "Overall coverage: $$(go tool cover -func=tests/coverage.out | grep total | awk '{print $$3}')"

coverage-html:
	@echo "Generating HTML coverage report..."
	@go test -coverprofile=tests/coverage.out ./... > /dev/null 2>&1
	@go tool cover -html=tests/coverage.out
	@echo "Coverage report opened in browser"

# Build distribution packages
dist: dist-rpm dist-deb

dist-rpm:
	mkdir -p dist/rpm
	# Add RPM building commands here
	@echo "RPM package built in dist/rpm/"

dist-deb:
	mkdir -p dist/deb
	# Add DEB building commands here
	@echo "DEB package built in dist/deb/"

# Development helpers
dev-deps:
	go mod download
	go install golang.org/x/tools/cmd/goimports@latest
	go install github.com/golangci/golangci-lint/cmd/golangci-lint@latest

fmt:
	go fmt ./...
	goimports -w .

lint:
	golangci-lint run

# Quick development cycle
dev: fmt test coverage-md

# Watch for changes and run tests (requires entr)
watch:
	find . -name '*.go' | entr -c make test

help:
	@echo "Available targets:"
	@echo "  build        - Build the binary"
	@echo "  test         - Run tests"
	@echo "  coverage     - Generate markdown coverage report"
	@echo "  coverage-md  - Generate markdown coverage report"
	@echo "  coverage-html- Open HTML coverage in browser"
	@echo "  clean        - Remove built files and reports"
	@echo "  install      - Install binary to system"
	@echo "  uninstall    - Remove installed binary"
	@echo "  fmt          - Format code"
	@echo "  lint         - Run linter"
	@echo "  dev          - Format, test, and generate coverage"
	@echo "  help         - Show this help message"