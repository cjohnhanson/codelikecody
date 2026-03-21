# codelikecody project commands

# default: list recipes
default:
    @just --list

# --- build ---

# build all workspace crates
build:
    cargo build --workspace

# build in release mode
build-release:
    cargo build --workspace --release

# --- test ---

# run cargo tests
test:
    cargo test --workspace

# run missouri e2e tests
test-missouri:
    missouri run

# run all tests (cargo + missouri)
test-all: test test-missouri

# --- lint ---

# run clippy
clippy:
    cargo clippy --workspace -- -D warnings

# run cargo fmt check
fmt-check:
    cargo fmt --all -- --check

# format code
fmt:
    cargo fmt --all

# run all checks (fmt + clippy + test)
check: fmt-check clippy test

# --- docs site ---

# serve docs site locally
docs-serve:
    cd docs && mdbook serve --open

# build docs site
docs-build:
    cd docs && mdbook build

# --- utilities ---

# show clc status
status:
    clc status

# list open tiskets
issues:
    tisket issue list
