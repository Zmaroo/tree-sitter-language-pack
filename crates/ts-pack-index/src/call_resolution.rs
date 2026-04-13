use std::collections::{HashMap, HashSet};

use crate::pathing;
use crate::{CallRef, CallRefKind, ImportSymbolRequest, SymbolCallRow};

pub(crate) struct CallResolutionContext<'a> {
    pub(crate) callable_symbols_by_name: &'a HashMap<String, Vec<(String, String)>>,
    pub(crate) qualified_callable_symbols: &'a [(String, String, String)],
    pub(crate) symbols_by_file: &'a HashMap<String, HashMap<String, String>>,
    pub(crate) imported_target_files_by_src: &'a HashMap<String, HashSet<String>>,
    pub(crate) import_symbol_requests: &'a [ImportSymbolRequest],
    pub(crate) exported_symbols_by_file: &'a HashMap<String, Vec<String>>,
    pub(crate) files_set: &'a HashSet<String>,
    pub(crate) rust_local_module_roots_by_src_root: &'a HashMap<String, HashSet<String>>,
}

pub(crate) enum CallResolution {
    Resolved(String, &'static str),
    Unresolved,
}

pub(crate) struct CallResolutionOutputs {
    pub(crate) symbol_call_rows: Vec<SymbolCallRow>,
    pub(crate) resolved_call_rows: usize,
    pub(crate) resolution_stage_counts: HashMap<&'static str, usize>,
    pub(crate) unresolved_name_counts: HashMap<String, usize>,
    pub(crate) unresolved_bucket_counts: HashMap<String, usize>,
    pub(crate) unresolved_bucket_samples: HashMap<String, Vec<String>>,
    pub(crate) unresolved_rust_plain_attribution: HashMap<(String, String), usize>,
    pub(crate) skipped_external_call_rows: usize,
}

fn strip_generic_segments(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut depth = 0usize;
    for ch in text.chars() {
        match ch {
            '<' => depth += 1,
            '>' => depth = depth.saturating_sub(1),
            _ if depth == 0 => out.push(ch),
            _ => {}
        }
    }
    out
}

fn rust_path_variants(text: &str) -> Vec<String> {
    let mut variants = Vec::new();
    variants.push(text.to_string());

    let mut current = text;
    while let Some(rest) = current
        .strip_prefix("crate::")
        .or_else(|| current.strip_prefix("self::"))
        .or_else(|| current.strip_prefix("super::"))
    {
        variants.push(rest.to_string());
        current = rest;
    }

    variants
}

pub(crate) fn normalize_qualified_variants(text: &str, language: &str) -> Vec<String> {
    let compact = text.trim().replace(":: <", "::<").replace(' ', "");
    let compact = compact.trim_start_matches('&').trim_start_matches("mut ");
    let no_generics = strip_generic_segments(compact);
    let base = if no_generics.is_empty() {
        compact.to_string()
    } else {
        no_generics
    };

    let mut variants = match language {
        "rust" => rust_path_variants(&base),
        _ => vec![base],
    };
    variants.sort();
    variants.dedup();
    variants
}

pub(crate) fn normalize_qualified_hint(text: &str) -> String {
    normalize_qualified_variants(text, "")
        .into_iter()
        .next()
        .unwrap_or_default()
}

fn receiver_qualified_candidates(call_ref: &CallRef) -> Vec<String> {
    let Some(receiver) = call_ref.receiver_hint.as_ref() else {
        return Vec::new();
    };
    let receiver = receiver.trim();
    if receiver.is_empty() {
        return Vec::new();
    }

    let mut candidates = Vec::new();
    match call_ref.language.as_str() {
        "rust" => {
            candidates.push(format!("{receiver}::{}", call_ref.callee));
            candidates.push(format!("{receiver}.{}", call_ref.callee));
        }
        "go" | "javascript" | "typescript" | "tsx" | "jsx" | "python" => {
            candidates.push(format!("{receiver}.{}", call_ref.callee));
        }
        _ => {
            candidates.push(format!("{receiver}.{}", call_ref.callee));
            candidates.push(format!("{receiver}::{}", call_ref.callee));
        }
    }
    candidates
        .into_iter()
        .flat_map(|text| normalize_qualified_variants(&text, &call_ref.language))
        .collect()
}

fn resolve_by_global_unique(ctx: &CallResolutionContext<'_>, call_ref: &CallRef) -> Option<String> {
    let candidates = ctx.callable_symbols_by_name.get(&call_ref.callee)?;
    let mut matches = candidates
        .iter()
        .filter(|(_, filepath)| call_ref.allow_same_file || filepath != &call_ref.caller_filepath)
        .map(|(id, _)| id.clone());
    let first = matches.next();
    let second = matches.next();
    if second.is_none() { first } else { None }
}

fn resolve_by_same_file(ctx: &CallResolutionContext<'_>, call_ref: &CallRef) -> Option<String> {
    if !call_ref.allow_same_file {
        return None;
    }
    ctx.symbols_by_file
        .get(&call_ref.caller_filepath)
        .and_then(|sym_map| sym_map.get(&call_ref.callee).cloned())
}

fn resolve_by_import_symbol_request(ctx: &CallResolutionContext<'_>, call_ref: &CallRef) -> Option<String> {
    for req in ctx
        .import_symbol_requests
        .iter()
        .filter(|req| req.src_filepath == call_ref.caller_filepath)
    {
        let target_fp = pathing::resolve_module_path(&req.src_filepath, &req.module, ctx.files_set);
        let sym_map = target_fp.as_ref().and_then(|fp| ctx.symbols_by_file.get(fp));
        if req.items.is_empty() {
            if call_ref.language == "rust"
                && let Some((module_path, imported_name)) = req.module.rsplit_once("::")
                && imported_name == call_ref.callee
            {
                let imported_target = pathing::resolve_module_path(&req.src_filepath, module_path, ctx.files_set);
                let imported_sym_map = imported_target
                    .as_ref()
                    .and_then(|fp| ctx.symbols_by_file.get(fp));
                if let Some(imported_sym_map) = imported_sym_map
                    && let Some(sym_id) = imported_sym_map.get(imported_name)
                {
                    return Some(sym_id.clone());
                }
            }
            if let Some(fp) = target_fp.as_ref() {
                if let Some(sym_map) = sym_map {
                    if let Some(sym_id) = sym_map.get(&call_ref.callee) {
                        return Some(sym_id.clone());
                    }
                } else if let Some(exported) = ctx.exported_symbols_by_file.get(fp) {
                    if let Some(sym_id) = exported.first() {
                        return Some(sym_id.clone());
                    }
                }
            }
            continue;
        }
        if !req
            .items
            .iter()
            .any(|item| pathing::clean_import_name(item) == call_ref.callee)
        {
            continue;
        }
        if let Some(sym_map) = sym_map {
            if let Some(sym_id) = sym_map.get(&call_ref.callee) {
                return Some(sym_id.clone());
            }
        }
    }
    None
}

fn resolve_by_imported_target_unique(ctx: &CallResolutionContext<'_>, call_ref: &CallRef) -> Option<String> {
    let candidates = ctx.callable_symbols_by_name.get(&call_ref.callee)?;
    let imported_files = ctx.imported_target_files_by_src.get(&call_ref.caller_filepath)?;
    let mut matches = candidates
        .iter()
        .filter(|(_, filepath)| imported_files.contains(filepath))
        .map(|(id, _)| id.clone());
    let first = matches.next();
    let second = matches.next();
    if second.is_none() { first } else { None }
}

fn resolve_by_local_directory_unique(ctx: &CallResolutionContext<'_>, call_ref: &CallRef) -> Option<String> {
    let candidates = ctx.callable_symbols_by_name.get(&call_ref.callee)?;
    let caller_dir = std::path::Path::new(&call_ref.caller_filepath)
        .parent()
        .and_then(|p| p.to_str())
        .map(|s| s.trim_end_matches('/').to_string())?;
    let dir_prefix = if caller_dir.is_empty() {
        String::new()
    } else {
        format!("{caller_dir}/")
    };
    let mut matches = candidates
        .iter()
        .filter(|(_, filepath)| call_ref.allow_same_file || *filepath != call_ref.caller_filepath)
        .filter(|(_, filepath)| dir_prefix.is_empty() || filepath.starts_with(&dir_prefix))
        .map(|(id, _)| id.clone());
    let first = matches.next();
    let second = matches.next();
    if second.is_none() { first } else { None }
}

fn resolve_by_qualified_hint(ctx: &CallResolutionContext<'_>, call_ref: &CallRef) -> Option<String> {
    let hint = call_ref.qualified_hint.as_ref()?;
    let normalized = normalize_qualified_variants(hint, &call_ref.language);
    let mut matches = ctx
        .qualified_callable_symbols
        .iter()
        .filter(|(qualified_name, _, filepath)| {
            normalized.iter().any(|candidate| {
                (qualified_name == candidate
                    || qualified_name.ends_with(candidate)
                    || candidate.ends_with(qualified_name.as_str()))
                    && (call_ref.allow_same_file || filepath != &call_ref.caller_filepath)
            })
        })
        .map(|(_, id, _)| id.clone());
    let first = matches.next();
    let second = matches.next();
    if second.is_none() { first } else { None }
}

fn resolve_by_receiver_qualified(ctx: &CallResolutionContext<'_>, call_ref: &CallRef) -> Option<String> {
    let candidates = receiver_qualified_candidates(call_ref);
    if candidates.is_empty() {
        return None;
    }

    let mut matches = ctx
        .qualified_callable_symbols
        .iter()
        .filter(|(qualified_name, _, filepath)| {
            candidates.iter().any(|candidate| {
                (qualified_name == candidate
                    || qualified_name.ends_with(candidate)
                    || candidate.ends_with(qualified_name.as_str()))
                    && (call_ref.allow_same_file || filepath != &call_ref.caller_filepath)
            })
        })
        .map(|(_, id, _)| id.clone());
    let first = matches.next();
    let second = matches.next();
    if second.is_none() { first } else { None }
}

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
    let Some(src_root) = rust_source_root(&call_ref.caller_filepath) else {
        return false;
    };
    match ctx.rust_local_module_roots_by_src_root.get(&src_root) {
        Some(local_roots) => !local_roots.contains(root),
        None => true,
    }
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

pub(crate) fn resolve_call_ref(ctx: &CallResolutionContext<'_>, call_ref: &CallRef) -> CallResolution {
    let stages: &[(&str, fn(&CallResolutionContext<'_>, &CallRef) -> Option<String>)] = match call_ref.kind {
        CallRefKind::Scoped => &[
            ("qualified", resolve_by_qualified_hint),
            ("receiver_qualified", resolve_by_receiver_qualified),
            ("import_symbol", resolve_by_import_symbol_request),
            ("imported_target", resolve_by_imported_target_unique),
            ("local_directory", resolve_by_local_directory_unique),
            ("global_unique", resolve_by_global_unique),
        ],
        CallRefKind::Member => &[
            ("receiver_qualified", resolve_by_receiver_qualified),
            ("import_symbol", resolve_by_import_symbol_request),
            ("imported_target", resolve_by_imported_target_unique),
            ("local_directory", resolve_by_local_directory_unique),
            ("global_unique", resolve_by_global_unique),
        ],
        CallRefKind::Plain => &[
            ("global_unique", resolve_by_global_unique),
            ("same_file", resolve_by_same_file),
            ("import_symbol", resolve_by_import_symbol_request),
            ("imported_target", resolve_by_imported_target_unique),
            ("local_directory", resolve_by_local_directory_unique),
        ],
    };

    for (stage, resolver) in stages {
        if let Some(id) = resolver(ctx, call_ref) {
            return CallResolution::Resolved(id, stage);
        }
    }
    CallResolution::Unresolved
}

fn is_rust_constructor_noise(call_ref: &CallRef) -> bool {
    call_ref.language == "rust"
        && matches!(call_ref.kind, CallRefKind::Plain)
        && matches!(call_ref.callee.as_str(), "Ok" | "Err" | "Some" | "None")
}

pub(crate) fn build_symbol_call_rows(
    call_refs: Vec<CallRef>,
    resolution_ctx: &CallResolutionContext<'_>,
    project_id: &std::sync::Arc<str>,
    debug_call_resolution: bool,
) -> CallResolutionOutputs {
    let mut symbol_call_rows = Vec::with_capacity(call_refs.len());
    let mut resolved_call_rows = 0usize;
    let mut resolution_stage_counts: HashMap<&'static str, usize> = HashMap::new();
    let mut unresolved_name_counts: HashMap<String, usize> = HashMap::new();
    let mut unresolved_bucket_counts: HashMap<String, usize> = HashMap::new();
    let mut unresolved_bucket_samples: HashMap<String, Vec<String>> = HashMap::new();
    let mut unresolved_rust_plain_attribution: HashMap<(String, String), usize> = HashMap::new();
    let mut skipped_external_call_rows = 0usize;

    for call_ref in call_refs {
        let callee_id = match resolve_call_ref(resolution_ctx, &call_ref) {
            CallResolution::Resolved(id, stage) => {
                resolved_call_rows += 1;
                *resolution_stage_counts.entry(stage).or_insert(0) += 1;
                Some(id)
            }
            CallResolution::Unresolved => {
                // Product policy: keep unresolved internal utility/helper calls unless they are
                // clearly external or obvious constructor noise. Several targeted suppression
                // experiments reduced rows but did not improve CALLS write time, and they risked
                // silently degrading graph quality. Use debug attribution to study hotspots before
                // introducing any new skip rule here.
                if is_rust_constructor_noise(&call_ref)
                    || is_clearly_external_rust_scoped_call(resolution_ctx, &call_ref)
                    || is_clearly_external_go_scoped_call(resolution_ctx, &call_ref)
                {
                    skipped_external_call_rows += 1;
                    continue;
                }
                if debug_call_resolution {
                    *unresolved_name_counts.entry(call_ref.callee.clone()).or_insert(0) += 1;
                    let kind = match call_ref.kind {
                        CallRefKind::Plain => "plain",
                        CallRefKind::Member => "member",
                        CallRefKind::Scoped => "scoped",
                    };
                    let bucket = format!(
                        "{}:{}:{}",
                        call_ref.language,
                        kind,
                        if call_ref.receiver_hint.is_some() {
                            "recv"
                        } else {
                            "norecv"
                        }
                    );
                    *unresolved_bucket_counts.entry(bucket.clone()).or_insert(0) += 1;
                    if bucket == "rust:plain:norecv" {
                        *unresolved_rust_plain_attribution
                            .entry((call_ref.callee.clone(), call_ref.caller_filepath.clone()))
                            .or_insert(0) += 1;
                    }
                    let samples = unresolved_bucket_samples.entry(bucket).or_default();
                    if samples.len() < 5 {
                        let qualified = call_ref.qualified_hint.as_deref().unwrap_or("-");
                        let receiver = call_ref.receiver_hint.as_deref().unwrap_or("-");
                        samples.push(format!(
                            "{} @ {} (qualified={}, recv={})",
                            call_ref.callee, call_ref.caller_filepath, qualified, receiver
                        ));
                    }
                }
                None
            }
        };
        symbol_call_rows.push(SymbolCallRow {
            caller_id: call_ref.caller_id,
            callee: call_ref.callee,
            callee_id,
            project_id: std::sync::Arc::clone(project_id),
            caller_filepath: call_ref.caller_filepath,
            allow_same_file: call_ref.allow_same_file,
        });
    }

    CallResolutionOutputs {
        symbol_call_rows,
        resolved_call_rows,
        resolution_stage_counts,
        unresolved_name_counts,
        unresolved_bucket_counts,
        unresolved_bucket_samples,
        unresolved_rust_plain_attribution,
        skipped_external_call_rows,
    }
}
