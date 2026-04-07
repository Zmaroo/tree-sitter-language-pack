from __future__ import annotations

from tree_sitter_language_pack import _native


def extract_swift_semantic_facts(project_path: str):
    return _native.extract_swift_semantic_facts(project_path)
