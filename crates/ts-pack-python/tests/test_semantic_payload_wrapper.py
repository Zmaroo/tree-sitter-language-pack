import unittest
import asyncio
import tempfile
from pathlib import Path

import tree_sitter_language_pack as ts
from tree_sitter_language_pack import _semantic_payload as semantic_payload
from tree_sitter_language_pack._semantic_contract import (
    FOCUSED_DISPATCHER_ANCHOR_CAPABILITY,
    FOCUSED_DISPATCHER_ANCHOR_CONTRACT_VERSION,
)


class SemanticPayloadWrapperTests(unittest.TestCase):
    def test_trace_graph_provenance_export_exists(self):
        self.assertTrue(hasattr(ts, "trace_graph_provenance"))

    def test_cached_downloaded_languages_are_available_without_explicit_init(self):
        self.assertIn("python", ts.downloaded_languages())
        self.assertTrue(ts.has_language("python"))
        self.assertIn("python", ts.available_languages())
        self.assertGreater(ts.language_count(), 0)
        self.assertIsNotNone(ts.get_parser("python"))

    def test_build_semantic_payload_enriches_usage_metadata(self):
        if not ts.has_language("python"):
            self.skipTest("python parser unavailable in test environment")

        payload = ts.build_semantic_payload(
            """
parser.parse(source)
""",
            "python",
            "examples/python_smoke/main.py",
            "proj",
        )

        chunks = payload.get("chunks") or []
        self.assertTrue(chunks)
        usage_chunks = [chunk for chunk in chunks if chunk.get("metadata", {}).get("chunk_role") == "example_usage"]
        self.assertTrue(usage_chunks)
        metadata = usage_chunks[0]["metadata"]
        self.assertEqual(metadata["member_usages"], ["parser.parse"])
        self.assertIn("parse", metadata["call_like_symbols"])

    def test_build_line_window_chunks_adds_entrypoint_anchor(self):
        chunks = ts.build_line_window_chunks(
            """
use anyhow::Result;

fn helper() {}

fn main() -> Result<()> {
    helper();
    Ok(())
}
""",
            "packages/desktop/src-tauri/src/main.rs",
            "proj",
            language="rust",
        )

        self.assertTrue(chunks)
        entrypoint_chunks = [chunk for chunk in chunks if chunk.get("metadata", {}).get("contains_entrypoint")]
        self.assertEqual(len(entrypoint_chunks), 1)
        entrypoint = entrypoint_chunks[0]
        self.assertIn("fn main()", entrypoint["text"])
        self.assertIn("main", entrypoint["metadata"]["declared_symbols"])
        self.assertEqual(entrypoint["metadata"]["chunk_role"], "definition")
        self.assertIn("entrypoint_surface", entrypoint["metadata"]["file_roles"])

    def test_build_line_window_chunks_extracts_swift_declared_symbols(self):
        chunks = ts.build_line_window_chunks(
            """
@main
struct DrawThingsCLI: ParsableCommand {
    static var configuration: CommandConfiguration { .init(commandName: "drawthings") }
}
""",
            "Apps/DrawThingsCLI/DrawThingsCLI.swift",
            "proj",
            language="swift",
        )

        self.assertTrue(chunks)
        metadata = chunks[0]["metadata"]
        self.assertIn("DrawThingsCLI", metadata["declared_symbols"])
        self.assertTrue(metadata["contains_definition"])
        self.assertEqual(metadata["chunk_role"], "definition")
        self.assertIn("runtime_entrypoint_surface", metadata["file_roles"])

    def test_build_line_window_chunks_marks_api_and_facade_surfaces(self):
        api_chunks = ts.build_line_window_chunks(
            """
class OwnerController:
    pass
""",
            "src/main/java/org/example/owner/OwnerController.java",
            "proj",
            language="java",
        )
        self.assertIn("api_surface", api_chunks[0]["metadata"]["file_roles"])

        facade_chunks = ts.build_line_window_chunks(
            """
from .core import run

__all__ = ["run"]
""",
            "pkg/__init__.py",
            "proj",
            language="python",
        )
        self.assertIn("library_facade_surface", facade_chunks[0]["metadata"]["file_roles"])

    def test_build_line_window_chunks_marks_file_family_surfaces(self):
        controller_chunks = ts.build_line_window_chunks(
            """
class OwnerController:
    pass
""",
            "src/main/java/app/owner/OwnerController.java",
            "proj",
            language="java",
        )
        self.assertIn("controller_surface", controller_chunks[0]["metadata"]["file_roles"])

        view_chunks = ts.build_line_window_chunks(
            """
struct SidebarView: View {}
""",
            "FrameCreator/Views/SidebarView.swift",
            "proj",
            language="swift",
        )
        self.assertIn("view_surface", view_chunks[0]["metadata"]["file_roles"])

        service_chunks = ts.build_line_window_chunks(
            """
final class SyncService {}
""",
            "app/services/SyncService.swift",
            "proj",
            language="swift",
        )
        self.assertIn("service_surface", service_chunks[0]["metadata"]["file_roles"])

        client_chunks = ts.build_line_window_chunks(
            """
final class APIClient {}
""",
            "Networking/APIClient.swift",
            "proj",
            language="swift",
        )
        self.assertIn("client_surface", client_chunks[0]["metadata"]["file_roles"])

    def test_build_line_window_chunks_marks_request_handler_routing_roles(self):
        controller_chunks = ts.build_line_window_chunks(
            """
class OwnerController {
    String processFindForm() { return "owners/findOwners"; }
}
""",
            "src/main/java/app/owner/OwnerController.java",
            "proj",
            language="java",
        )
        metadata = controller_chunks[0]["metadata"]
        self.assertIn("controller_surface", metadata["file_roles"])
        self.assertIn("request_handler_surface", metadata["file_roles"])
        self.assertEqual(
            metadata["declared_symbol_roles"].get("processFindForm"),
            ["request_handler"],
        )

    def test_build_line_window_chunks_marks_route_definition_symbol_roles(self):
        server_chunks = ts.build_line_window_chunks(
            """
final class ImageGenerationServiceImpl {
    func routeImageRequest() {}
}
""",
            "Libraries/GRPC/Server/Sources/ImageGenerationServiceImpl.swift",
            "proj",
            language="swift",
        )
        metadata = server_chunks[0]["metadata"]
        self.assertIn("request_handler_surface", metadata["file_roles"])
        self.assertIn("route_definition_surface", metadata["file_roles"])
        self.assertEqual(
            metadata["declared_symbol_roles"].get("routeImageRequest"),
            ["request_handler", "route_definition"],
        )

    def test_build_line_window_chunks_does_not_treat_java_package_samples_as_examples(self):
        controller_chunks = ts.build_line_window_chunks(
            """
@Controller
class OwnerController {}
""",
            "src/main/java/org/springframework/samples/petclinic/owner/OwnerController.java",
            "proj",
            language="java",
        )
        self.assertIn("controller_surface", controller_chunks[0]["metadata"]["file_roles"])
        self.assertNotIn("example_surface", controller_chunks[0]["metadata"]["file_roles"])

    def test_build_line_window_chunks_marks_rust_command_surfaces(self):
        command_chunks = ts.build_line_window_chunks(
            """
#[derive(Subcommand)]
pub enum Commands {
    Auth(AuthNamespace),
}
""",
            "crates/uv-cli/src/lib.rs",
            "proj",
            language="rust",
        )
        metadata = command_chunks[0]["metadata"]
        self.assertIn("command_surface", metadata["file_roles"])
        self.assertEqual(
            metadata["declared_symbol_roles"].get("Commands"),
            ["command_enum"],
        )

    def test_build_line_window_chunks_marks_canonical_dispatcher_surface(self):
        chunks = ts.build_line_window_chunks(
            """
def infer_model(model_name: str):
    return model_name
""",
            "pkg/models/__init__.py",
            "proj",
            language="python",
        )

        self.assertTrue(chunks)
        metadata = chunks[0]["metadata"]
        self.assertEqual(metadata["chunk_role"], "canonical_dispatcher_definition")
        self.assertIn("dispatcher_surface", metadata["file_roles"])
        self.assertIn("model_dispatcher_surface", metadata["file_roles"])
        self.assertEqual(
            metadata["declared_symbol_roles"].get("infer_model"),
            ["canonical_dispatcher", "dispatcher", "model_selector"],
        )

    def test_finalize_semantic_chunks_keeps_focused_canonical_dispatcher_anchor(self):
        source = """
def helper():
    return None

def infer_model(model_name: str):
    return model_name
"""
        existing_chunks = [
            {
                "text": "// File: pkg/models/__init__.py\n" + source.strip(),
                "metadata": {
                    "declared_symbols": ["helper", "infer_model"],
                    "file_symbols": ["helper", "infer_model"],
                    "contains_definition": True,
                    "node_types": ["function_definition"],
                },
            }
        ]
        chunks = semantic_payload._finalize_semantic_chunks(
            source,
            "pkg/models/__init__.py",
            "proj",
            {"file_symbols": ["helper", "infer_model"], "language": "python"},
            existing_chunks,
            chunk_id_version="semantic:test",
        )
        focused = [
            chunk for chunk in chunks
            if chunk.get("metadata", {}).get("chunk_role") == "canonical_dispatcher_definition"
            and chunk.get("metadata", {}).get("declared_symbols") == ["infer_model"]
        ]
        self.assertEqual(len(focused), 1)
        self.assertIn("canonical model inference selection dispatcher", focused[0]["text"])
        self.assertIn("where model inference is selected", focused[0]["text"])
        self.assertIn("Dispatcher symbol: infer_model", focused[0]["text"])
        self.assertIn("selects the concrete model implementation and provider", focused[0]["text"])
        self.assertTrue(any("def infer_model" in line for line in focused[0]["text"].splitlines()))
        self.assertEqual(
            focused[0]["metadata"]["focused_dispatcher_anchor_contract_version"],
            FOCUSED_DISPATCHER_ANCHOR_CONTRACT_VERSION,
        )
        self.assertEqual(
            focused[0]["metadata"]["semantic_contract_capabilities"],
            [FOCUSED_DISPATCHER_ANCHOR_CAPABILITY],
        )

    def test_native_build_semantic_payload_keeps_focused_canonical_dispatcher_anchor(self):
        if not ts.has_language("python"):
            self.skipTest("python parser unavailable in test environment")

        payload = ts._native.build_semantic_payload(
            """
def helper():
    return None

def infer_model(model_name: str):
    return model_name
""",
            "python",
            "pkg/models/__init__.py",
            "proj",
            "semantic:test",
            4000,
            200,
        )

        focused = [
            chunk
            for chunk in (payload.get("chunks") or [])
            if chunk.get("metadata", {}).get("chunk_role") == "canonical_dispatcher_definition"
            and chunk.get("metadata", {}).get("declared_symbols") == ["infer_model"]
        ]
        self.assertEqual(len(focused), 1)
        self.assertIn("canonical model inference selection dispatcher", focused[0]["text"])
        self.assertIn("where model inference is selected", focused[0]["text"])
        self.assertIn("Dispatcher symbol: infer_model", focused[0]["text"])
        self.assertIn("selects the concrete model implementation and provider", focused[0]["text"])
        self.assertIn("dispatcher_surface", focused[0]["metadata"]["file_roles"])
        self.assertEqual(
            focused[0]["metadata"]["focused_dispatcher_anchor_contract_version"],
            FOCUSED_DISPATCHER_ANCHOR_CONTRACT_VERSION,
        )
        self.assertEqual(
            focused[0]["metadata"]["semantic_contract_capabilities"],
            [FOCUSED_DISPATCHER_ANCHOR_CAPABILITY],
        )

    def test_finalize_semantic_chunks_prioritizes_focused_provider_anchor_over_generic_anchor_cap(self):
        source = """
def helper_1():
    return None

def helper_2():
    return None

def helper_3():
    return None

def helper_4():
    return None

def helper_5():
    return None

def helper_6():
    return None

def infer_provider_class(provider_name: str):
    return provider_name
"""
        existing_chunks = [
            {
                "text": "// File: pkg/providers/__init__.py\n" + source.strip(),
                "metadata": {
                    "declared_symbols": [
                        "helper_1",
                        "helper_2",
                        "helper_3",
                        "helper_4",
                        "helper_5",
                        "helper_6",
                        "infer_provider_class",
                    ],
                    "file_symbols": [
                        "helper_1",
                        "helper_2",
                        "helper_3",
                        "helper_4",
                        "helper_5",
                        "helper_6",
                        "infer_provider_class",
                    ],
                    "contains_definition": True,
                    "node_types": ["function_definition"],
                },
            }
        ]
        chunks = semantic_payload._finalize_semantic_chunks(
            source,
            "pkg/providers/__init__.py",
            "proj",
            {
                "file_symbols": [
                    "helper_1",
                    "helper_2",
                    "helper_3",
                    "helper_4",
                    "helper_5",
                    "helper_6",
                    "infer_provider_class",
                ],
                "language": "python",
            },
            existing_chunks,
            chunk_id_version="semantic:test",
        )
        focused = [
            chunk
            for chunk in chunks
            if chunk.get("metadata", {}).get("chunk_role") == "canonical_dispatcher_definition"
            and chunk.get("metadata", {}).get("declared_symbols") == ["infer_provider_class"]
        ]
        self.assertEqual(len(focused), 1)
        self.assertIn("canonical provider inference selection dispatcher", focused[0]["text"])
        self.assertIn("where provider inference is selected", focused[0]["text"])
        self.assertEqual(
            focused[0]["metadata"]["focused_dispatcher_anchor_contract_version"],
            FOCUSED_DISPATCHER_ANCHOR_CONTRACT_VERSION,
        )
        self.assertEqual(
            focused[0]["metadata"]["semantic_contract_capabilities"],
            [FOCUSED_DISPATCHER_ANCHOR_CAPABILITY],
        )
        body_lines = focused[0]["text"].splitlines()
        self.assertTrue(any(line.startswith("def infer_provider_class(") for line in body_lines))
        self.assertNotIn("return f'{self.__class__.__name__}", focused[0]["text"])
        self.assertIn("Dispatcher symbol: infer_provider_class", focused[0]["text"])
        self.assertIn("provider wiring and selection entrypoint", focused[0]["text"])
        self.assertIn("selects or constructs the concrete provider implementation", focused[0]["text"])

    def test_native_build_semantic_payload_prioritizes_focused_provider_anchor_over_generic_anchor_cap(self):
        if not ts.has_language("python"):
            self.skipTest("python parser unavailable in test environment")

        payload = ts._native.build_semantic_payload(
            """
def helper_1():
    return None

def helper_2():
    return None

def helper_3():
    return None

def helper_4():
    return None

def helper_5():
    return None

def helper_6():
    return None

def infer_provider_class(provider_name: str):
    return provider_name
""",
            "python",
            "pkg/providers/__init__.py",
            "proj",
            "semantic:test",
            4000,
            200,
        )

        focused = [
            chunk
            for chunk in (payload.get("chunks") or [])
            if chunk.get("metadata", {}).get("chunk_role") == "canonical_dispatcher_definition"
            and chunk.get("metadata", {}).get("declared_symbols") == ["infer_provider_class"]
        ]
        self.assertEqual(len(focused), 1)
        self.assertIn("canonical provider inference selection dispatcher", focused[0]["text"])
        self.assertIn("provider_dispatcher_surface", focused[0]["metadata"]["file_roles"])
        self.assertEqual(
            focused[0]["metadata"]["focused_dispatcher_anchor_contract_version"],
            FOCUSED_DISPATCHER_ANCHOR_CONTRACT_VERSION,
        )
        self.assertEqual(
            focused[0]["metadata"]["semantic_contract_capabilities"],
            [FOCUSED_DISPATCHER_ANCHOR_CAPABILITY],
        )
        body_lines = focused[0]["text"].splitlines()
        self.assertTrue(any(line.startswith("def infer_provider_class(") for line in body_lines))
        self.assertNotIn("return f'{self.__class__.__name__}", focused[0]["text"])
        self.assertIn("Dispatcher symbol: infer_provider_class", focused[0]["text"])
        self.assertIn("provider wiring and selection entrypoint", focused[0]["text"])
        self.assertIn("selects or constructs the concrete provider implementation", focused[0]["text"])

    def test_focused_canonical_dispatcher_anchor_ref_id_changes_with_anchor_text(self):
        source_a = """
def infer_model(model_name: str):
    return model_name
"""
        source_b = """
def infer_model(model_name: str):
    return model_name.upper()
"""
        chunks_a = semantic_payload._finalize_semantic_chunks(
            source_a,
            "pkg/models/__init__.py",
            "proj",
            {"file_symbols": ["infer_model"], "language": "python"},
            [],
            chunk_id_version="semantic:test",
        )
        chunks_b = semantic_payload._finalize_semantic_chunks(
            source_b,
            "pkg/models/__init__.py",
            "proj",
            {"file_symbols": ["infer_model"], "language": "python"},
            [],
            chunk_id_version="semantic:test",
        )
        focused_a = next(
            chunk
            for chunk in chunks_a
            if chunk.get("metadata", {}).get("chunk_role") == "canonical_dispatcher_definition"
            and chunk.get("metadata", {}).get("declared_symbols") == ["infer_model"]
        )
        focused_b = next(
            chunk
            for chunk in chunks_b
            if chunk.get("metadata", {}).get("chunk_role") == "canonical_dispatcher_definition"
            and chunk.get("metadata", {}).get("declared_symbols") == ["infer_model"]
        )
        self.assertNotEqual(focused_a["ref_id"], focused_b["ref_id"])

    def test_finalize_semantic_chunks_does_not_promote_profile_helper_to_dispatcher_anchor(self):
        source = """
def infer_model_profile(model_name: str):
    return model_name
"""
        chunks = semantic_payload._finalize_semantic_chunks(
            source,
            "pkg/models/__init__.py",
            "proj",
            {"file_symbols": ["infer_model_profile"], "language": "python"},
            [],
            chunk_id_version="semantic:test",
        )
        focused = [
            chunk
            for chunk in chunks
            if chunk.get("metadata", {}).get("focused_dispatcher_anchor_contract_version")
            == FOCUSED_DISPATCHER_ANCHOR_CONTRACT_VERSION
        ]
        self.assertEqual(focused, [])

    def test_native_build_semantic_payload_does_not_promote_profile_helper_to_dispatcher_anchor(self):
        if not ts.has_language("python"):
            self.skipTest("python parser unavailable in test environment")

        payload = ts._native.build_semantic_payload(
            """
def infer_model_profile(model_name: str):
    return model_name
""",
            "python",
            "pkg/models/__init__.py",
            "proj",
            "semantic:test",
            4000,
            200,
        )
        focused = [
            chunk
            for chunk in (payload.get("chunks") or [])
            if chunk.get("metadata", {}).get("focused_dispatcher_anchor_contract_version")
            == FOCUSED_DISPATCHER_ANCHOR_CONTRACT_VERSION
        ]
        self.assertEqual(focused, [])

    def test_build_line_window_chunks_marks_profile_surface(self):
        chunks = ts.build_line_window_chunks(
            """
def deepseek_model_profile(model_name: str):
    return model_name
""",
            "pkg/profiles/deepseek.py",
            "proj",
            language="python",
        )

        self.assertTrue(chunks)
        metadata = chunks[0]["metadata"]
        self.assertEqual(metadata["chunk_role"], "profile_definition")
        self.assertIn("profile_surface", metadata["file_roles"])
        self.assertEqual(
            metadata["declared_symbol_roles"].get("deepseek_model_profile"),
            ["profile", "profile_surface"],
        )

    def test_build_line_window_chunks_marks_generated_example_and_binding_surfaces(self):
        generated_chunks = ts.build_line_window_chunks(
            """
public func call() {}
""",
            "Libraries/GRPC/Models/Sources/imageService/imageService.grpc.swift",
            "proj",
            language="swift",
        )
        self.assertIn("generated_surface", generated_chunks[0]["metadata"]["file_roles"])

        example_chunks = ts.build_line_window_chunks(
            """
parser.parse(source)
""",
            "examples/python_smoke/main.py",
            "proj",
            language="python",
        )
        self.assertIn("example_surface", example_chunks[0]["metadata"]["file_roles"])
        self.assertEqual(example_chunks[0]["metadata"]["chunk_role"], "example_usage")

        binding_chunks = ts.build_line_window_chunks(
            """
public class ProcessResult {}
""",
            "sdk/java/ProcessResult.java",
            "proj",
            language="java",
        )
        self.assertIn("binding_surface", binding_chunks[0]["metadata"]["file_roles"])

    def test_build_line_window_chunks_marks_test_and_benchmark_surfaces(self):
        test_chunks = ts.build_line_window_chunks(
            """
def test_parse():
    assert True
""",
            "e2e/python/tests/test_parsing.py",
            "proj",
            language="python",
        )
        self.assertIn("test_surface", test_chunks[0]["metadata"]["file_roles"])
        self.assertEqual(test_chunks[0]["metadata"]["chunk_role"], "test_usage")

        benchmark_chunks = ts.build_line_window_chunks(
            """
pub fn benchmark_parser() {}
""",
            "benchmarks/parser_bench.rs",
            "proj",
            language="rust",
        )
        self.assertIn("benchmark_surface", benchmark_chunks[0]["metadata"]["file_roles"])

    def test_build_line_window_chunks_marks_docs_and_config_surfaces(self):
        docs_chunks = ts.build_line_window_chunks(
            """
# Overview

This explains the architecture.
""",
            "docs/architecture.md",
            "proj",
            language=None,
        )
        self.assertIn("docs_surface", docs_chunks[0]["metadata"]["file_roles"])

        config_chunks = ts.build_line_window_chunks(
            """
site_name: Docs
nav:
  - Home: index.md
""",
            "mkdocs.yml",
            "proj",
            language=None,
        )
        self.assertIn("config_surface", config_chunks[0]["metadata"]["file_roles"])

    def test_process_semantic_manifest_entries_enriches_fallback_metadata(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            tmp_path = Path(tmpdir)
            abs_path = tmp_path / "mkdocs.yml"
            abs_path.write_text("site_name: Docs\nnav:\n  - Home: index.md\n")
            manifest = [
                {
                    "abs_path": str(abs_path),
                    "rel_path": "mkdocs.yml",
                    "ext": "yml",
                }
            ]

            payload = ts.process_semantic_manifest_entries(
                manifest,
                "proj",
                max_file_bytes=1_000_000,
                chunk_id_version="v6",
                chunk_max_size=4000,
                chunk_overlap=200,
                chunk_lines=60,
                overlap_lines=10,
                skip_diagnostic_files=False,
            )

            self.assertEqual(len(payload), 1)
            chunks = payload[0]["chunks"]
            self.assertTrue(chunks)
            metadata = chunks[0]["metadata"]
            self.assertIn("declared_symbol_roles", metadata)
            self.assertIn("file_roles", metadata)
            self.assertEqual(metadata["declared_symbol_roles"], {})
            self.assertEqual(metadata["file_roles"], ["config_surface"])
            expected_chunks = ts.build_line_window_chunks(
                abs_path.read_text(),
                "mkdocs.yml",
                "proj",
                language=None,
            )
            self.assertEqual(chunks[0]["ref_id"], expected_chunks[0]["ref_id"])

    def test_build_swift_chunks_emit_type_definition_chunk(self):
        if not ts.has_language("swift"):
            self.skipTest("swift parser unavailable in test environment")

        chunks = ts.build_swift_chunks(
            """
import SwiftUI

struct SidebarView: View {
    var body: some View {
        Text("Sidebar")
    }
}
""",
            "FrameCreator/Views/SidebarView.swift",
            "proj",
        )

        self.assertTrue(chunks)
        definition_chunks = [
            chunk
            for chunk in chunks
            if "SidebarView" in (chunk.get("metadata", {}).get("declared_symbols") or [])
        ]
        self.assertTrue(definition_chunks)
        definition = definition_chunks[0]
        self.assertIn("struct SidebarView: View", definition["text"])
        self.assertTrue(definition["metadata"]["contains_definition"])
        self.assertEqual(definition["metadata"]["chunk_role"], "definition")
        self.assertEqual(definition["metadata"]["context_path"], ["SidebarView"])

    def test_build_swift_chunks_split_oversized_single_line_members(self):
        if not ts.has_language("swift"):
            self.skipTest("swift parser unavailable in test environment")

        huge_literal = "a" * 47000
        chunks = ts.build_swift_chunks(
            f"""
struct StringNormalizationCases {{
    static let cases = ["{huge_literal}"]
}}
""",
            "validation-test/stdlib/StringNormalization.swift",
            "proj",
            chunk_max_size=4000,
            chunk_lines=60,
            overlap_lines=10,
        )

        self.assertTrue(chunks)
        max_body_bytes = max(
            len("\n".join(chunk["text"].splitlines()[1:]).encode("utf-8"))
            for chunk in chunks
        )
        self.assertLessEqual(max_body_bytes, 4000)
        self.assertTrue(
            any(
                "StringNormalizationCases" in (chunk.get("metadata", {}).get("declared_symbols") or [])
                for chunk in chunks
            )
        )

    def test_build_indexing_chunks_falls_back_for_pathological_swift_nesting(self):
        huge_parens = "(_:" * 3000 + "0" + ")" * 3000
        payload = ts.build_indexing_chunks(
            f"let x =\n{huge_parens}\n",
            "test/Parse/structure_overflow_paren_exprs.swift",
            "proj",
            language="swift",
            chunk_max_size=4000,
            chunk_overlap=200,
            chunk_lines=60,
            overlap_lines=10,
        )

        chunks = payload.get("chunks") or []
        self.assertTrue(chunks)
        self.assertEqual(payload.get("language"), "swift")
        self.assertTrue(all("metadata" in chunk for chunk in chunks))

    def test_build_indexing_chunks_falls_back_for_objcpp_and_keeps_objc_symbols(self):
        source = """
#import <Foundation/Foundation.h>

@interface Counter : NSObject
- (int)add:(int)a to:(int)b;
@end

@implementation Counter
- (int)add:(int)a to:(int)b {
    std::vector<int> values = {a, b};
    return values[0] + values[1];
}
@end
"""
        payload = ts.build_indexing_chunks(
            source,
            "src/Bridge.mm",
            "proj",
            language="objc",
            chunk_max_size=4000,
            chunk_overlap=200,
            chunk_lines=60,
            overlap_lines=10,
        )

        chunks = payload.get("chunks") or []
        self.assertTrue(chunks)
        all_declared = {
            symbol
            for chunk in chunks
            for symbol in ((chunk.get("metadata") or {}).get("declared_symbols") or [])
        }
        self.assertIn("Counter", all_declared)

    def test_build_indexing_chunks_strips_nul_bytes_from_chunk_text(self):
        payload = ts.build_indexing_chunks(
            "let weird = \"a\\x00b\\x00c\"\n",
            "test/Parse/strange-characters.swift",
            "proj",
            language="swift",
            chunk_max_size=4000,
            chunk_overlap=200,
            chunk_lines=60,
            overlap_lines=10,
        )

        chunks = payload.get("chunks") or []
        self.assertTrue(chunks)
        self.assertTrue(all("\x00" not in chunk["text"] for chunk in chunks))

    def test_execute_semantic_index_driver_commits_before_rounds(self):
        class _Conn:
            def __init__(self):
                self.commits = 0

            async def execute(self, *_args, **_kwargs):
                class _Cursor:
                    async def fetchall(self):
                        return []

                    def __aiter__(self):
                        async def _iter():
                            if False:
                                yield None
                        return _iter()

                return _Cursor()

            def cursor(self):
                class _PruneCursor:
                    rowcount = 0

                    async def __aenter__(self):
                        return self

                    async def __aexit__(self, exc_type, exc, tb):
                        return False

                    async def execute(self, *_args, **_kwargs):
                        return None

                return _PruneCursor()

            async def commit(self):
                self.commits += 1

        conn = _Conn()
        observed = {"commit_seen": False}

        async def _embed(batch):
            return batch

        async def _write(batch):
            observed["commit_seen"] = conn.commits > 0
            return len(batch)

        async def _run():
            return await ts.execute_semantic_index_driver(
                conn,
                "proj",
                ["src/sample.py"],
                [[{"ref_id": "chunk-1", "text": "hello", "metadata": {"file": "src/sample.py"}}]],
                rebuild=False,
                batch_size=1,
                concurrency=1,
                embed_batch_fn=_embed,
                write_batch_fn=_write,
            )

        result = asyncio.run(_run())

        self.assertEqual(conn.commits, 1)
        self.assertTrue(observed["commit_seen"])
        self.assertEqual(result["written"], 1)

    def test_execute_semantic_index_rounds_coalesces_writes_per_round(self):
        embedded_calls = []
        written_batches = []

        async def _embed(batch):
            embedded_calls.append(len(batch))
            return [
                {
                    "ref_id": item["ref_id"],
                    "text": item["text"],
                    "vector": [0.1, 0.2],
                    "metadata": item["metadata"],
                }
                for item in batch
            ]

        async def _write(batch):
            written_batches.append(len(batch))
            return len(batch)

        new_chunks = [
            {"ref_id": f"chunk-{i}", "text": f"text {i}", "metadata": {"file": f"src/{i}.py"}}
            for i in range(4)
        ]

        result = asyncio.run(
            semantic_payload.execute_semantic_index_rounds(
                new_chunks,
                batch_size=2,
                concurrency=2,
                embed_batch_fn=_embed,
                write_batch_fn=_write,
            )
        )

        self.assertEqual(result["written"], 4)
        self.assertEqual(embedded_calls, [2, 2])
        self.assertEqual(written_batches, [4])

    def test_execute_semantic_index_prepare_scopes_queries_to_manifest(self):
        class _CursorResult:
            def __init__(self, rows=None, rowcount=0):
                self._rows = rows or []
                self.rowcount = rowcount

            async def fetchall(self):
                return self._rows

        class _Conn:
            def __init__(self):
                self.calls = []

            async def execute(self, query, params):
                self.calls.append((" ".join(str(query).split()), params))
                if "SELECT chunk_id" in query:
                    return _CursorResult(rows=[("chunk-1",)])
                return _CursorResult(rowcount=3)

            def cursor(self):
                class _PruneCursor:
                    rowcount = 0

                    async def __aenter__(self):
                        return self

                    async def __aexit__(self, exc_type, exc, tb):
                        return False

                    async def execute(self, *_args, **_kwargs):
                        return None

                return _PruneCursor()

        conn = _Conn()
        result = asyncio.run(
            semantic_payload.execute_semantic_index_prepare(
                conn,
                "proj",
                ["src/a.py", "src/b.py"],
                [[{"ref_id": "chunk-2", "text": "hello", "metadata": {"file": "src/a.py"}}]],
                rebuild=False,
            )
        )

        self.assertEqual(result["orphan_pruned"], 3)
        self.assertEqual(result["existing_ids"], {"chunk-1"})
        select_calls = [call for call in conn.calls if "SELECT chunk_id" in call[0]]
        self.assertEqual(len(select_calls), 1)
        self.assertIn("file_path = ANY(%s)", select_calls[0][0])
        self.assertEqual(select_calls[0][1], ("proj", ["src/a.py", "src/b.py"]))
        delete_calls = [call for call in conn.calls if "DELETE FROM codebase_embeddings" in call[0]]
        self.assertEqual(len(delete_calls), 1)
        self.assertIn("NOT (file_path = ANY(%s))", delete_calls[0][0])

    def test_execute_semantic_index_prepare_rebuild_ignores_existing_ids(self):
        class _CursorResult:
            def __init__(self, rows=None, rowcount=0):
                self._rows = rows or []
                self.rowcount = rowcount

            async def fetchall(self):
                return self._rows

        class _Conn:
            def __init__(self):
                self.calls = []

            async def execute(self, query, params):
                self.calls.append((" ".join(str(query).split()), params))
                if "SELECT chunk_id" in query:
                    return _CursorResult(rows=[("chunk-1",)])
                if "DELETE FROM codebase_embeddings" in query:
                    return _CursorResult(rowcount=5)
                return _CursorResult()

            def cursor(self):
                class _PruneCursor:
                    rowcount = 0

                    async def __aenter__(self):
                        return self

                    async def __aexit__(self, exc_type, exc, tb):
                        return False

                    async def execute(self, *_args, **_kwargs):
                        return None

                return _PruneCursor()

        conn = _Conn()
        result = asyncio.run(
            semantic_payload.execute_semantic_index_prepare(
                conn,
                "proj",
                ["src/a.py"],
                [[{"ref_id": "chunk-1", "text": "hello", "metadata": {"file": "src/a.py"}}]],
                rebuild=True,
            )
        )

        self.assertEqual(result["existing_ids"], set())
        self.assertEqual(len(result["new_chunks"]), 1)
        select_calls = [call for call in conn.calls if "SELECT chunk_id" in call[0]]
        self.assertEqual(select_calls, [])
        delete_calls = [call for call in conn.calls if "DELETE FROM codebase_embeddings" in call[0]]
        self.assertEqual(len(delete_calls), 1)

    def test_execute_semantic_index_driver_emits_prepare_done_progress(self):
        class _Cursor:
            async def fetchall(self):
                return []

        class _Conn:
            async def execute(self, *_args, **_kwargs):
                return _Cursor()

            def cursor(self):
                class _PruneCursor:
                    rowcount = 0

                    async def __aenter__(self):
                        return self

                    async def __aexit__(self, exc_type, exc, tb):
                        return False

                    async def execute(self, *_args, **_kwargs):
                        return None

                return _PruneCursor()

            async def commit(self):
                return None

        events = []

        async def _progress(event):
            events.append(event)

        async def _embed(batch):
            return batch

        async def _write(batch):
            return len(batch)

        result = asyncio.run(
            semantic_payload.execute_semantic_index_driver(
                _Conn(),
                "proj",
                ["src/sample.py"],
                [[{"ref_id": "chunk-1", "text": "hello", "metadata": {"file": "src/sample.py"}}]],
                rebuild=False,
                batch_size=1,
                concurrency=1,
                embed_batch_fn=_embed,
                write_batch_fn=_write,
                progress_fn=_progress,
            )
        )

        self.assertEqual(result["written"], 1)
        prepare_events = [event for event in events if event.get("phase") == "prepare_done"]
        self.assertEqual(len(prepare_events), 1)
        self.assertIn("prepare_seconds", prepare_events[0])


if __name__ == "__main__":
    unittest.main()
