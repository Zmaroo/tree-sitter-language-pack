from __future__ import annotations

FOCUSED_DISPATCHER_ANCHOR_CONTRACT_VERSION = 1
FOCUSED_DISPATCHER_ANCHOR_CAPABILITY = (
    f"focused_dispatcher_anchor_v{FOCUSED_DISPATCHER_ANCHOR_CONTRACT_VERSION}"
)

REQUIRED_SEMANTIC_CHUNK_FIELDS: tuple[str, ...] = (
    "member_usages",
    "call_like_symbols",
    "declared_symbols",
    "declared_symbol_roles",
    "file_roles",
    "contains_definition",
    "contains_entrypoint",
    "chunk_role",
)
