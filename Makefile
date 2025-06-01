.PHONY: build test docs release

# Build all workspace members
build:
	cargo build --workspace

# Run all tests in the workspace
test:
	cargo test --workspace

# Generate documentation for the entire workspace
docs:
	cargo doc --workspace --no-deps --open

# Release a new version (requires VERSION to be set)
release:
ifndef VERSION
	$(error VERSION is not set. Usage: make release VERSION=<x.y.z>)
endif
	cargo release $(VERSION) --workspace --execute
