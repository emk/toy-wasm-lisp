# Build our Rust-based WASL compiler.
build: grammar
    cargo build

# Regenerate the tree-sitter grammar.
grammar:
    pushd crates/tree-sitter-wasl/ && tree-sitter generate
