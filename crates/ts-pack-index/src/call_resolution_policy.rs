use crate::pathing;

use super::{CallRef, CallRefKind, CallResolution, CallResolutionContext, ExternalSymbolResolution};

fn rust_source_root(filepath: &str) -> Option<String> {
    let marker = "/src/";
    let idx = filepath.find(marker)?;
    Some(filepath[..idx + marker.len() - 1].to_string())
}

fn rust_qualified_root(text: &str) -> Option<&str> {
    let normalized = text.trim();
    if normalized.is_empty() {
        return None;
    }
    normalized.split("::").find(|segment| !segment.is_empty())
}

fn is_clearly_external_rust_scoped_call(ctx: &CallResolutionContext<'_>, call_ref: &CallRef) -> bool {
    if call_ref.language != "rust" || !matches!(call_ref.kind, CallRefKind::Scoped) {
        return false;
    }
    let Some(hint) = call_ref.qualified_hint.as_deref() else {
        return false;
    };
    let Some(root) = rust_qualified_root(hint) else {
        return false;
    };
    if matches!(root, "crate" | "self" | "super" | "Self") {
        return false;
    }
    if let Some(src_root) = rust_source_root(&call_ref.caller_filepath) {
        return match ctx.rust_local_module_roots_by_src_root.get(&src_root) {
            Some(local_roots) => !local_roots.contains(root),
            None => true,
        };
    }
    !ctx.rust_local_module_roots_by_src_root
        .values()
        .any(|local_roots| local_roots.contains(root))
}

fn import_module_alias(module: &str) -> Option<&str> {
    module.rsplit('/').find(|segment| !segment.is_empty())
}

fn is_clearly_external_go_scoped_call(ctx: &CallResolutionContext<'_>, call_ref: &CallRef) -> bool {
    if call_ref.language != "go" || !matches!(call_ref.kind, CallRefKind::Scoped) {
        return false;
    }
    let Some(receiver) = call_ref.receiver_hint.as_deref() else {
        return false;
    };
    ctx.import_symbol_requests
        .iter()
        .filter(|req| req.src_filepath == call_ref.caller_filepath)
        .filter(|req| req.items.is_empty())
        .filter(|req| import_module_alias(&req.module) == Some(receiver))
        .any(|req| pathing::resolve_module_path(&req.src_filepath, &req.module, ctx.files_set).is_none())
}

fn is_python_member_or_scoped(call_ref: &CallRef) -> bool {
    call_ref.language == "python" && matches!(call_ref.kind, CallRefKind::Member | CallRefKind::Scoped)
}

fn python_receiver(call_ref: &CallRef) -> Option<&str> {
    if !is_python_member_or_scoped(call_ref) {
        return None;
    }
    call_ref.receiver_hint.as_deref()
}

fn python_path_matches(call_ref: &CallRef, suffix: &str) -> bool {
    call_ref.language == "python" && call_ref.caller_filepath.ends_with(suffix)
}

fn is_python_script_path(path: &str) -> bool {
    path.starts_with("scripts/") || path.contains("/scripts/")
}

const PYTHON_SCRIPT_EXTERNAL_MODULE_PREFIXES: &[&str] = &[
    "json.",
    "yaml.",
    "re.",
    "hashlib.",
    "shutil.",
    "os.",
    "platform.",
    "asyncio.",
    "argparse.",
];

fn resolve_known_python_external_qualified(qualified: &str) -> Option<&'static str> {
    match qualified {
        "json.loads" => Some("json.loads"),
        "sys.exit" => Some("sys.exit"),
        "logging.getLogger" => Some("logging.getLogger"),
        "yaml.safe_load" => Some("yaml.safe_load"),
        "re.sub" => Some("re.sub"),
        "argparse.ArgumentParser" => Some("argparse.ArgumentParser"),
        "json.load" => Some("json.load"),
        "json.dumps" => Some("json.dumps"),
        "os.cpu_count" => Some("os.cpu_count"),
        "asyncio.Semaphore" => Some("asyncio.Semaphore"),
        "asyncio.gather" => Some("asyncio.gather"),
        "hashlib.sha256" => Some("hashlib.sha256"),
        "platform.system" => Some("platform.system"),
        "re.search" => Some("re.search"),
        "re.match" => Some("re.match"),
        "shutil.copy2" => Some("shutil.copy2"),
        "shutil.rmtree" => Some("shutil.rmtree"),
        _ => None,
    }
}

fn is_python_builtin_noise(call_ref: &CallRef) -> bool {
    call_ref.language == "python"
        && matches!(call_ref.kind, CallRefKind::Plain)
        && matches!(
            call_ref.callee.as_str(),
            "len"
                | "isinstance"
                | "set"
                | "list"
                | "dict"
                | "tuple"
                | "str"
                | "int"
                | "float"
                | "bool"
                | "min"
                | "max"
                | "sum"
                | "sorted"
                | "any"
                | "all"
                | "enumerate"
                | "zip"
                | "range"
                | "abs"
                | "getattr"
                | "deepcopy"
        )
}

fn is_python_container_method_noise(ctx: &CallResolutionContext<'_>, call_ref: &CallRef) -> bool {
    if !is_python_member_or_scoped(call_ref) {
        return false;
    }
    let Some(receiver) = python_receiver(call_ref) else {
        return false;
    };
    if !receiver
        .chars()
        .next()
        .map(|ch| ch.is_ascii_lowercase())
        .unwrap_or(false)
    {
        return false;
    }
    if receiver.contains('.') || receiver.contains("::") {
        return false;
    }
    if ctx
        .python_module_aliases_by_file
        .get(&call_ref.caller_filepath)
        .and_then(|m| m.get(receiver))
        .is_some()
    {
        return false;
    }
    matches!(
        call_ref.callee.as_str(),
        "append" | "add" | "extend" | "update" | "discard" | "remove" | "pop" | "clear" | "items" | "splitlines"
    )
}

fn is_python_regex_method_noise(call_ref: &CallRef) -> bool {
    if !is_python_member_or_scoped(call_ref) {
        return false;
    }
    let Some(receiver) = python_receiver(call_ref) else {
        return false;
    };
    matches!(call_ref.callee.as_str(), "search" | "match" | "findall" | "sub")
        && (receiver.starts_with('_')
            || receiver
                .chars()
                .all(|ch| !ch.is_ascii_alphabetic() || ch.is_ascii_uppercase() || ch == '_'))
}

fn is_python_semantic_payload_support_plain_noise(call_ref: &CallRef) -> bool {
    call_ref.language == "python"
        && python_path_matches(call_ref, "python/tree_sitter_language_pack/_semantic_payload.py")
        && matches!(call_ref.kind, CallRefKind::Plain)
        && matches!(
            call_ref.callee.as_str(),
            "_native_build_semantic_sync_plan"
                | "_native_build_codebase_embedding_rows"
                | "round_plan_builder"
                | "progress_fn"
                | "__import__"
                | "embed_batch_fn"
                | "write_batch_fn"
                | "driver_plan_builder"
        )
}

fn is_python_semantic_payload_support_member_noise(call_ref: &CallRef) -> bool {
    if !python_path_matches(call_ref, "python/tree_sitter_language_pack/_semantic_payload.py")
        || !is_python_member_or_scoped(call_ref)
    {
        return false;
    }
    let Some(receiver) = python_receiver(call_ref) else {
        return false;
    };
    match call_ref.callee.as_str() {
        "encode" | "strip" => matches!(receiver, "source" | "text" | "content"),
        "parse" => receiver == "parser",
        "sha256" => receiver == "hashlib",
        "dumps" => receiver == "json",
        "execute" => matches!(receiver, "conn" | "prune_cursor"),
        "cursor" => receiver == "conn",
        "executemany" => receiver == "cursor",
        "fetchall" => receiver == "cur",
        _ => false,
    }
}

fn is_python_init_wrapper_plain_noise(call_ref: &CallRef) -> bool {
    call_ref.language == "python"
        && python_path_matches(call_ref, "python/tree_sitter_language_pack/__init__.py")
        && matches!(call_ref.kind, CallRefKind::Plain)
        && matches!(
            call_ref.callee.as_str(),
            "repr" | "PurePosixPath" | "NotImplementedError"
        )
}

fn is_python_init_wrapper_member_noise(call_ref: &CallRef) -> bool {
    if !python_path_matches(call_ref, "python/tree_sitter_language_pack/__init__.py")
        || !is_python_member_or_scoped(call_ref)
    {
        return false;
    }
    let Some(receiver) = python_receiver(call_ref) else {
        return false;
    };
    match call_ref.callee.as_str() {
        "split" => matches!(receiver, "value" | "location"),
        "decode" => receiver == "source",
        "replace" => matches!(
            receiver,
            "file_path" | "project_file" | "container" | "group_path" | "rel_path"
        ),
        "startswith" | "endswith" => matches!(receiver, "location" | "normalized_path"),
        "findall" => receiver == "root",
        "get_targets" => receiver == "objects",
        "get_id" => matches!(receiver, "target" | "build_file" | "file_ref" | "phase"),
        "get_objects_in_section" => receiver == "objects",
        _ => false,
    }
}

fn is_python_script_support_member_noise(call_ref: &CallRef) -> bool {
    if !is_python_member_or_scoped(call_ref) {
        return false;
    }
    if !is_python_script_path(&call_ref.caller_filepath) {
        return false;
    }
    let Some(receiver) = python_receiver(call_ref) else {
        return false;
    };
    let callee = call_ref.callee.as_str();
    let lower_receiver = receiver.to_ascii_lowercase();
    let pathish_receiver = lower_receiver.ends_with("_path")
        || lower_receiver.ends_with("_dir")
        || lower_receiver.ends_with("_file")
        || lower_receiver.contains("path")
        || lower_receiver.contains("dir")
        || lower_receiver.contains("file")
        || matches!(
            receiver,
            "CACHE_MANIFEST_FILE"
                | "DEFINITIONS_PATH"
                | "_cache_path"
                | "ffi_lib"
                | "header"
                | "clone_target"
                | "target_src"
                | "target_common"
                | "target_queries"
                | "vendor_directory"
                | "stale_dir"
                | "parsers_directory"
                | "parser_dir"
                | "target_source_dir"
                | "replacement_path"
                | "config_path"
                | "output_path"
                | "definitions_path"
                | "cargo_toml"
                | "file_path"
                | "path"
                | "dest_lib"
                | "core_vendor"
                | "vendor_cargo"
                | "core_toml"
                | "vendor_base"
                | "src"
                | "artifact"
                | "php_toml"
                | "f"
                | "ruby_toml"
        );
    let stringish_receiver = matches!(
        receiver,
        "text"
            | "line"
            | "stripped"
            | "output"
            | "lang"
            | "lang_id"
            | "repo_url"
            | "key"
            | "version"
            | "content"
            | "word"
            | "field"
            | "current"
            | "url"
            | "val"
            | "base_spec"
    );
    let containerish_receiver = lower_receiver.contains("language")
        || matches!(receiver, "languages" | "language_def" | "language_definitions");
    match callee {
        "add_argument" | "parse_args" => receiver == "parser",
        "debug" | "info" | "warning" | "error" | "exception" | "setLevel" => receiver == "logger",
        "exists" | "open" | "write_text" | "mkdir" | "glob" | "rglob" | "iterdir" | "relative_to" | "stat"
        | "unlink" => pathish_receiver,
        "replace" => pathish_receiver || matches!(receiver, "lang_id" | "version" | "content"),
        "split" | "strip" | "rstrip" | "removesuffix" | "startswith" | "capitalize" => stringish_receiver,
        "keys" | "copy" => containerish_receiver,
        "render" => receiver == "template",
        "group" => matches!(receiver, "match" | "m"),
        _ => false,
    }
}

fn resolve_python_init_external_receiver(call_ref: &CallRef) -> Option<ExternalSymbolResolution> {
    if !python_path_matches(call_ref, "python/tree_sitter_language_pack/__init__.py")
        || !is_python_member_or_scoped(call_ref)
    {
        return None;
    }
    let receiver = python_receiver(call_ref)?;
    match (receiver, call_ref.callee.as_str()) {
        ("ElementTree", "fromstring") => Some(ExternalSymbolResolution {
            name: call_ref.callee.clone(),
            qualified_name: "xml.etree.ElementTree.fromstring".to_string(),
            language: call_ref.language.clone(),
        }),
        ("XcodeProject", "load") => Some(ExternalSymbolResolution {
            name: call_ref.callee.clone(),
            qualified_name: "pbxproj.XcodeProject.load".to_string(),
            language: call_ref.language.clone(),
        }),
        _ => None,
    }
}

fn resolve_explicit_python_external_receiver(call_ref: &CallRef) -> Option<ExternalSymbolResolution> {
    if !is_python_member_or_scoped(call_ref) {
        return None;
    }
    let qualified = call_ref.qualified_hint.as_deref().unwrap_or("");
    if is_python_script_path(&call_ref.caller_filepath)
        && PYTHON_SCRIPT_EXTERNAL_MODULE_PREFIXES
            .iter()
            .any(|prefix| qualified.starts_with(prefix))
    {
        return Some(ExternalSymbolResolution {
            name: call_ref.callee.clone(),
            qualified_name: qualified.to_string(),
            language: call_ref.language.clone(),
        });
    }
    let resolved = resolve_known_python_external_qualified(qualified)?;
    Some(ExternalSymbolResolution {
        name: call_ref.callee.clone(),
        qualified_name: resolved.to_string(),
        language: call_ref.language.clone(),
    })
}

fn resolve_external_python_module_receiver(
    ctx: &CallResolutionContext<'_>,
    call_ref: &CallRef,
) -> Option<ExternalSymbolResolution> {
    if !is_python_member_or_scoped(call_ref) {
        return None;
    }
    let receiver = python_receiver(call_ref)?;
    let module = ctx
        .python_module_aliases_by_file
        .get(&call_ref.caller_filepath)
        .and_then(|m| m.get(receiver))?;
    if pathing::resolve_module_path(&call_ref.caller_filepath, module, ctx.files_set).is_some() {
        return None;
    }
    Some(ExternalSymbolResolution {
        name: call_ref.callee.clone(),
        qualified_name: format!("{module}.{}", call_ref.callee),
        language: call_ref.language.clone(),
    })
}

fn is_rust_constructor_noise(call_ref: &CallRef) -> bool {
    call_ref.language == "rust"
        && matches!(call_ref.kind, CallRefKind::Plain)
        && matches!(call_ref.callee.as_str(), "Ok" | "Err" | "Some" | "None")
}

fn is_filtered_same_file_policy(ctx: &CallResolutionContext<'_>, call_ref: &CallRef) -> bool {
    !call_ref.allow_same_file
        && ctx
            .symbols_by_file
            .get(&call_ref.caller_filepath)
            .and_then(|sym_map| sym_map.get(&call_ref.callee))
            .is_some()
}

fn is_go_test_receiver_noise(call_ref: &CallRef) -> bool {
    if call_ref.language != "go" || !matches!(call_ref.kind, CallRefKind::Scoped) {
        return false;
    }
    let Some(receiver) = call_ref.receiver_hint.as_deref() else {
        return false;
    };
    if !matches!(receiver, "t" | "b" | "m") {
        return false;
    }
    let path = call_ref.caller_filepath.as_str();
    if !(path.contains("/test") || path.ends_with("_test.go")) {
        return false;
    }
    match receiver {
        "t" => matches!(
            call_ref.callee.as_str(),
            "Helper" | "Fatal" | "Fatalf" | "Error" | "Errorf" | "Log" | "Logf" | "Run" | "Cleanup" | "Skipf"
        ),
        "b" => matches!(call_ref.callee.as_str(), "Run"),
        "m" => matches!(call_ref.callee.as_str(), "Run"),
        _ => false,
    }
}

fn is_python_test_receiver_noise(call_ref: &CallRef) -> bool {
    if call_ref.language != "python" || !matches!(call_ref.kind, CallRefKind::Member | CallRefKind::Scoped) {
        return false;
    }
    let path = call_ref.caller_filepath.as_str();
    if !(path.contains("/tests/") || path.starts_with("tests/") || path.contains("test_")) {
        return false;
    }
    let qualified = call_ref.qualified_hint.as_deref().unwrap_or("");
    if matches!(
        qualified,
        "unittest.main" | "pytest.fixture" | "pytest.mark" | "pytest.fail" | "pytest.raises"
    ) {
        return true;
    }
    let Some(receiver) = call_ref.receiver_hint.as_deref() else {
        return false;
    };
    match receiver {
        "self" => matches!(
            call_ref.callee.as_str(),
            "skipTest"
                | "assertIn"
                | "assertEqual"
                | "assertTrue"
                | "assertFalse"
                | "assertRaises"
                | "assertIsNone"
                | "assertIsNotNone"
        ),
        "unittest" => matches!(call_ref.callee.as_str(), "main"),
        "pytest" => matches!(call_ref.callee.as_str(), "fixture" | "mark" | "fail" | "raises"),
        _ => false,
    }
}

pub(super) fn resolve_call_ref_filters(ctx: &CallResolutionContext<'_>, call_ref: &CallRef) -> Option<CallResolution> {
    if is_rust_constructor_noise(call_ref) {
        return Some(CallResolution::Filtered("constructor_noise", None));
    }
    if is_python_builtin_noise(call_ref) {
        return Some(CallResolution::Filtered("python_builtin", None));
    }
    if is_python_semantic_payload_support_plain_noise(call_ref) {
        return Some(CallResolution::Filtered("python_payload_support_plain", None));
    }
    if is_python_init_wrapper_plain_noise(call_ref) {
        return Some(CallResolution::Filtered("python_init_wrapper_plain", None));
    }
    if is_python_container_method_noise(ctx, call_ref) {
        return Some(CallResolution::Filtered("python_container_method", None));
    }
    if is_python_semantic_payload_support_member_noise(call_ref) {
        return Some(CallResolution::Filtered("python_payload_support_member", None));
    }
    if is_python_init_wrapper_member_noise(call_ref) {
        return Some(CallResolution::Filtered("python_init_wrapper_member", None));
    }
    if is_python_script_support_member_noise(call_ref) {
        return Some(CallResolution::Filtered("python_script_support_member", None));
    }
    if is_python_regex_method_noise(call_ref) {
        return Some(CallResolution::Filtered("python_regex_method", None));
    }
    if is_filtered_same_file_policy(ctx, call_ref) {
        return Some(CallResolution::Filtered("policy_same_file", None));
    }
    if is_go_test_receiver_noise(call_ref) {
        return Some(CallResolution::Filtered("go_test_receiver", None));
    }
    if is_python_test_receiver_noise(call_ref) {
        return Some(CallResolution::Filtered("python_test_receiver", None));
    }
    if is_clearly_external_rust_scoped_call(ctx, call_ref) {
        return Some(CallResolution::Filtered(
            "external_rust_scoped",
            call_ref
                .qualified_hint
                .as_ref()
                .map(|qualified_name| ExternalSymbolResolution {
                    name: call_ref.callee.clone(),
                    qualified_name: qualified_name.clone(),
                    language: call_ref.language.clone(),
                }),
        ));
    }
    if is_clearly_external_go_scoped_call(ctx, call_ref) {
        return Some(CallResolution::Filtered(
            "external_go_scoped",
            call_ref
                .qualified_hint
                .as_ref()
                .map(|qualified_name| ExternalSymbolResolution {
                    name: call_ref.callee.clone(),
                    qualified_name: qualified_name.clone(),
                    language: call_ref.language.clone(),
                }),
        ));
    }
    if let Some(external_symbol) = resolve_external_python_module_receiver(ctx, call_ref) {
        return Some(CallResolution::Filtered(
            "external_python_module",
            Some(external_symbol),
        ));
    }
    if let Some(external_symbol) = resolve_python_init_external_receiver(call_ref) {
        return Some(CallResolution::Filtered(
            "external_python_module",
            Some(external_symbol),
        ));
    }
    if let Some(external_symbol) = resolve_explicit_python_external_receiver(call_ref) {
        return Some(CallResolution::Filtered(
            "external_python_module",
            Some(external_symbol),
        ));
    }
    None
}

pub(super) fn python_scoped_trace_flags(
    ctx: &CallResolutionContext<'_>,
    call_ref: &CallRef,
) -> Option<(bool, bool, bool, bool, bool, bool)> {
    if !(call_ref.language == "python"
        && matches!(call_ref.kind, CallRefKind::Scoped)
        && (call_ref
            .caller_filepath
            .ends_with("python/tree_sitter_language_pack/_semantic_payload.py")
            || call_ref
                .caller_filepath
                .ends_with("python/tree_sitter_language_pack/__init__.py")
            || call_ref.caller_filepath.contains("/tests/")
            || call_ref.caller_filepath.starts_with("tests/")))
    {
        return None;
    }
    Some((
        is_python_semantic_payload_support_member_noise(call_ref),
        is_python_init_wrapper_member_noise(call_ref),
        is_python_container_method_noise(ctx, call_ref),
        is_python_regex_method_noise(call_ref),
        is_python_test_receiver_noise(call_ref),
        resolve_external_python_module_receiver(ctx, call_ref).is_some()
            || resolve_python_init_external_receiver(call_ref).is_some()
            || resolve_explicit_python_external_receiver(call_ref).is_some(),
    ))
}
