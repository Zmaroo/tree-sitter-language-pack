# Local Fork Boundary

This repository is a local fork of `kreuzberg-dev/tree-sitter-language-pack`.

The fork exists because our GraphRAG and MCP stack depends on code paths that do not exist on upstream `main`, especially around indexing, graph enrichment, and retrieval-oriented analysis.

## Upstream-safe area

Changes are good upstream candidates when they are:

- fully contained in `crates/ts-pack-core`
- general parser/query/intelligence correctness fixes
- general performance improvements that do not depend on our fork-specific indexer
- small symbol/intelligence coverage improvements that fit the public `ts-pack-core` contract

Recent examples:

- parser reuse in `parse_string()`
- compiled query reuse in `run_query()`
- Go type declarations surfaced as symbols

## Fork-only area

Changes should stay local when they depend on surfaces that upstream `main` does not currently have, or when they mainly exist to support our retrieval/indexing stack.

That currently includes:

- `crates/ts-pack-index`
- graph finalization and file-level GDS metrics in `crates/ts-pack-python`
- retrieval-adjacent duplicate analysis and reranking support
- JS/TS tag and file-fact behavior that is tuned for our GraphRAG use case

## Why this split exists

Upstream `main` has `ts-pack-core` and language bindings, but it does not include the full fork-specific indexing/graph pipeline we use locally. Because of that, many local fixes and optimizations are valuable for us but do not map cleanly onto upstream code.

## Local operating rule

When deciding whether to upstream a change:

1. If it is self-contained in `ts-pack-core`, consider upstreaming it.
2. If it depends on `ts-pack-index`, graph finalization, or retrieval semantics, keep it local unless upstream grows equivalent surfaces.
3. Prefer small upstream PRs over large fork-shaped changesets.

