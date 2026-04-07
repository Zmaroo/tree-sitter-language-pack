/// Language-specific tree-sitter tags queries.
///
/// Captures:
///   @vis    — visibility modifier (e.g. `pub` in Rust)
///   @name   — function/method/class definition name
///   @callee — identifier being called at a call site
///   @exported — JS/TS export statement wrapper (anonymous capture)
///
/// Each language query is compiled once at call time by `ts_pack::run_query`.
/// Queries that fail to compile (wrong node type for a grammar) are silently
/// skipped — safe to add patterns for new languages without breaking existing ones.
use std::collections::{HashMap, HashSet};
use tree_sitter_language_pack as ts_pack;

// ---------------------------------------------------------------------------
// Per-language S-expression query strings
// ---------------------------------------------------------------------------

/// Rust: detect `pub`/`pub(...)` functions and call expressions.
const RUST_TAGS: &str = r#"
(function_item
  (visibility_modifier) @vis
  name: (identifier) @name)

(call_expression
  function: (identifier) @callee)

(call_expression
  function: (field_expression
    field: (identifier) @callee))

(call_expression
  function: (scoped_identifier
    name: (identifier) @callee))
"#;

/// Python: all defs (visibility is by _ convention). Call expressions.
const PYTHON_TAGS: &str = r#"
(function_definition
  name: (identifier) @name)

(call
  function: (identifier) @callee)

(call
  function: (attribute
    object: (identifier) @recv
    attribute: (identifier) @callee))

(call
  function: (attribute
    object: (identifier) @launch_module
    attribute: (identifier) @launch_callee)
  arguments: (argument_list
    (list
      (string) @launch_arg)))
"#;

const PYTHON_LAUNCH_ASSIGN_TAGS: &str = r#"
(assignment
  left: (identifier) @launch_assign_name
  right: (list (string) @launch_assign_str)) @launch_assign_stmt

(assignment
  left: (identifier) @launch_assign_name
  right: (tuple (string) @launch_assign_str)) @launch_assign_stmt

(assignment
  left: (identifier) @launch_assign_name
  right: (list
    (call
      function: (attribute) @launch_join_fn
      arguments: (argument_list (string) @launch_join_arg)) @launch_join_call)) @launch_assign_stmt

(assignment
  left: (identifier) @launch_assign_name
  right: (tuple
    (call
      function: (attribute) @launch_join_fn
      arguments: (argument_list (string) @launch_join_arg)) @launch_join_call)) @launch_assign_stmt
"#;

const PYTHON_LAUNCH_IDENT_CALL_TAGS: &str = r#"
(call
  function: (attribute
    object: (identifier) @launch_module
    attribute: (identifier) @launch_callee)
  arguments: (argument_list (identifier) @launch_arg_ident)) @launch_call

(call
  function: (attribute
    object: (identifier) @launch_module
    attribute: (identifier) @launch_callee)
  arguments: (argument_list
    (keyword_argument value: (identifier) @launch_arg_ident))) @launch_call
"#;

/// JavaScript: exported functions (explicit `export` keyword). Call expressions.
const JS_TAGS: &str = r#"
(export_statement
  (function_declaration
    name: (identifier) @name)) @exported

(export_statement
  (lexical_declaration
    (variable_declarator
      name: (identifier) @name
      value: (arrow_function)))) @exported

(export_statement
  (lexical_declaration
    (variable_declarator
      name: (identifier) @name
      value: (function_expression)))) @exported

(call_expression
  function: (identifier) @callee)

(call_expression
  function: (member_expression
    property: (property_identifier) @callee))

(member_expression
  object: (identifier) @dbobj
  property: (property_identifier) @db)
(#eq? @dbobj "prisma")

(member_expression
  object: (identifier) @dbobj
  property: (identifier) @db)
(#eq? @dbobj "prisma")

(member_expression
  object: (identifier) @dbobj
  property: (property_identifier) @db)
(#eq? @dbobj "tx")

(member_expression
  object: (identifier) @dbobj
  property: (identifier) @db)
(#eq? @dbobj "tx")

(member_expression
  object: (identifier) @dbobj
  property: (property_identifier) @db)
(#eq? @dbobj "prismaClient")

(member_expression
  object: (identifier) @dbobj
  property: (identifier) @db)
(#eq? @dbobj "prismaClient")

(member_expression
  object: (identifier) @dbobj
  property: (property_identifier) @db)
(#eq? @dbobj "db")

(member_expression
  object: (identifier) @dbobj
  property: (identifier) @db)
(#eq? @dbobj "db")

(member_expression
  object: (member_expression
    object: (this)
    property: (property_identifier) @dbobj)
  property: (property_identifier) @db)
(#eq? @dbobj "prisma")

(member_expression
  object: (member_expression
    object: (this)
    property: (identifier) @dbobj)
  property: (identifier) @db)
(#eq? @dbobj "prisma")

(member_expression
  object: (member_expression
    property: (property_identifier) @dbobj)
  property: (property_identifier) @db)
(#eq? @dbobj "prisma")

(member_expression
  object: (member_expression
    property: (identifier) @dbobj)
  property: (identifier) @db)
(#eq? @dbobj "prisma")

(member_expression
  object: (member_expression
    object: (identifier) @ctx
    property: (property_identifier) @dbobj)
  property: (property_identifier) @db)
(#eq? @dbobj "prisma")

(member_expression
  object: (member_expression
    object: (identifier) @ctx
    property: (identifier) @dbobj)
  property: (identifier) @db)
(#eq? @dbobj "prisma")

(member_expression
  object: (member_expression
    object: (identifier) @ctx
    property: (property_identifier) @dbobj)
  property: (property_identifier) @db)
(#match? @dbobj ".*Prisma$")

(member_expression
  object: (member_expression
    object: (identifier) @ctx
    property: (identifier) @dbobj)
  property: (identifier) @db)
(#match? @dbobj ".*Prisma$")

(call_expression
  function: (identifier) @external_callee
  arguments: (arguments (string) @external_arg))

(call_expression
  function: (identifier) @external_callee
  arguments: (arguments (identifier) @external_arg))

(call_expression
  function: (identifier) @external_callee
  arguments: (arguments (template_string) @external_arg))

(call_expression
  function: (member_expression
    object: (identifier) @external_callee)
  arguments: (arguments (string) @external_arg))

(call_expression
  function: (member_expression
    object: (identifier) @external_callee)
  arguments: (arguments (identifier) @external_arg))

(call_expression
  function: (member_expression
    object: (identifier) @external_callee)
  arguments: (arguments (template_string) @external_arg))

(call_expression
  function: (identifier) @external_callee
  arguments: (arguments
    (binary_expression
      left: (identifier) @external_arg_left
      operator: "+"
      right: (string) @external_arg_right)))

(call_expression
  function: (identifier) @external_callee
  arguments: (arguments
    (binary_expression
      left: (string) @external_arg_left
      operator: "+"
      right: (identifier) @external_arg_right)))

(call_expression
  function: (member_expression
    object: (identifier) @external_callee)
  arguments: (arguments
    (binary_expression
      left: (identifier) @external_arg_left
      operator: "+"
      right: (string) @external_arg_right)))

(call_expression
  function: (member_expression
    object: (identifier) @external_callee)
  arguments: (arguments
    (binary_expression
      left: (string) @external_arg_left
      operator: "+"
      right: (identifier) @external_arg_right)))

(call_expression
  function: (identifier) @external_callee
  arguments: (arguments
    (new_expression
      constructor: (identifier) @external_url_ctor
      arguments: (arguments (string) @external_url_path (string) @external_url_base))))

(call_expression
  function: (identifier) @external_callee
  arguments: (arguments
    (new_expression
      constructor: (identifier) @external_url_ctor
      arguments: (arguments (string) @external_url_path (identifier) @external_url_base_ident))))

(call_expression
  function: (member_expression
    object: (identifier) @external_callee)
  arguments: (arguments
    (new_expression
      constructor: (identifier) @external_url_ctor
      arguments: (arguments (string) @external_url_path (string) @external_url_base))))

(call_expression
  function: (member_expression
    object: (identifier) @external_callee)
  arguments: (arguments
    (new_expression
      constructor: (identifier) @external_url_ctor
      arguments: (arguments (string) @external_url_path (identifier) @external_url_base_ident))))

(lexical_declaration
  (variable_declarator
    name: (identifier) @const_name
    value: (string) @const_value))

(lexical_declaration
  (variable_declarator
    name: (identifier) @const_name
    value: (template_string) @const_value))

(lexical_declaration
  (variable_declarator
    name: (identifier) @const_name
    value: (binary_expression
      left: (string) @const_left
      operator: "+"
      right: (string) @const_right)))

(lexical_declaration
  (variable_declarator
    name: (identifier) @const_name
    value: (member_expression
      object: (member_expression
        object: (identifier) @env_root
        property: (property_identifier) @env_prop)
      property: (property_identifier) @env_key)))

(lexical_declaration
  (variable_declarator
    name: (identifier) @const_name
    value: (member_expression
      object: (member_expression
        object: (member_expression
          object: (identifier) @env_import
          property: (property_identifier) @env_meta)
        property: (property_identifier) @env_env)
      property: (property_identifier) @env_key)))

(import_clause
  name: (identifier) @import_name)

(import_clause
  (named_imports
    (import_specifier
      name: (identifier) @import_named)))

(import_clause
  (named_imports
    (import_specifier
      name: (property_identifier) @import_named)))

(import_clause
  (namespace_import
    (identifier) @import_star))
"#;

/// Go: all top-level functions (all exported if name starts with uppercase,
/// but we capture all and let the resolver use naming convention).
const GO_TAGS: &str = r#"
(function_declaration
  name: (identifier) @name)

(method_declaration
  name: (field_identifier) @name)

(call_expression
  function: (identifier) @callee)

(call_expression
  function: (selector_expression
    field: (field_identifier) @callee))
"#;

/// Swift: exported declarations and call expressions.
const SWIFT_TAGS: &str = r#"
(function_declaration
  (modifiers) @vis
  name: (simple_identifier) @name) @exported

(function_declaration
  (modifiers) @vis
  name: (identifier) @name) @exported

(class_declaration
  (modifiers) @vis
  name: (type_identifier) @name) @exported

(struct_declaration
  (modifiers) @vis
  name: (type_identifier) @name) @exported

(enum_declaration
  (modifiers) @vis
  name: (type_identifier) @name) @exported

(protocol_declaration
  (modifiers) @vis
  name: (type_identifier) @name) @exported

(typealias_declaration
  (modifiers) @vis
  name: (type_identifier) @name) @exported

(function_declaration
  name: (simple_identifier) @name)

(function_declaration
  name: (identifier) @name)

(call_expression
  (simple_identifier) @callee
  (call_suffix))

(call_expression
  (identifier) @callee
  (call_suffix))

(call_expression
  (navigation_expression
    target: (self_expression) @recv
    (navigation_suffix
      (simple_identifier) @callee))
  (call_suffix))

(call_expression
  (navigation_expression
    target: (simple_identifier) @recv
    (navigation_suffix
      (simple_identifier) @callee))
  (call_suffix))

(call_expression
  (navigation_expression
    target: (identifier) @recv
    (navigation_suffix
      (simple_identifier) @callee))
  (call_suffix))
"#;

/// TypeScript/TSX: same as JS plus type-annotated export forms.
const TS_TAGS: &str = r#"
(export_statement
  (function_declaration
    name: (identifier) @name)) @exported

(export_statement
  (lexical_declaration
    (variable_declarator
      name: (identifier) @name
      value: [(arrow_function)(function_expression)]))) @exported

(call_expression
  function: (identifier) @callee)

(call_expression
  function: (member_expression
    property: (property_identifier) @callee))

(member_expression
  object: (identifier) @dbobj
  property: (property_identifier) @db)
(#eq? @dbobj "prisma")

(member_expression
  object: (identifier) @dbobj
  property: (identifier) @db)
(#eq? @dbobj "prisma")

(member_expression
  object: (identifier) @dbobj
  property: (property_identifier) @db)
(#eq? @dbobj "tx")

(member_expression
  object: (identifier) @dbobj
  property: (identifier) @db)
(#eq? @dbobj "tx")

(member_expression
  object: (identifier) @dbobj
  property: (property_identifier) @db)
(#eq? @dbobj "prismaClient")

(member_expression
  object: (identifier) @dbobj
  property: (identifier) @db)
(#eq? @dbobj "prismaClient")

(member_expression
  object: (identifier) @dbobj
  property: (property_identifier) @db)
(#eq? @dbobj "db")

(member_expression
  object: (identifier) @dbobj
  property: (identifier) @db)
(#eq? @dbobj "db")

(member_expression
  object: (member_expression
    object: (this)
    property: (property_identifier) @dbobj)
  property: (property_identifier) @db)
(#eq? @dbobj "prisma")

(member_expression
  object: (member_expression
    object: (this)
    property: (identifier) @dbobj)
  property: (identifier) @db)
(#eq? @dbobj "prisma")

(member_expression
  object: (member_expression
    property: (property_identifier) @dbobj)
  property: (property_identifier) @db)
(#eq? @dbobj "prisma")

(member_expression
  object: (member_expression
    property: (identifier) @dbobj)
  property: (identifier) @db)
(#eq? @dbobj "prisma")

(member_expression
  object: (member_expression
    object: (identifier) @ctx
    property: (property_identifier) @dbobj)
  property: (property_identifier) @db)
(#eq? @dbobj "prisma")

(member_expression
  object: (member_expression
    object: (identifier) @ctx
    property: (identifier) @dbobj)
  property: (identifier) @db)
(#eq? @dbobj "prisma")

(member_expression
  object: (member_expression
    object: (identifier) @ctx
    property: (property_identifier) @dbobj)
  property: (property_identifier) @db)
(#match? @dbobj ".*Prisma$")

(member_expression
  object: (member_expression
    object: (identifier) @ctx
    property: (identifier) @dbobj)
  property: (identifier) @db)
(#match? @dbobj ".*Prisma$")

(call_expression
  function: (identifier) @external_callee
  arguments: (arguments (string) @external_arg))

(call_expression
  function: (identifier) @external_callee
  arguments: (arguments (identifier) @external_arg))

(call_expression
  function: (identifier) @external_callee
  arguments: (arguments (template_string) @external_arg))

(call_expression
  function: (member_expression
    object: (identifier) @external_callee)
  arguments: (arguments (string) @external_arg))

(call_expression
  function: (member_expression
    object: (identifier) @external_callee)
  arguments: (arguments (identifier) @external_arg))

(call_expression
  function: (member_expression
    object: (identifier) @external_callee)
  arguments: (arguments (template_string) @external_arg))

(call_expression
  function: (identifier) @external_callee
  arguments: (arguments
    (binary_expression
      left: (identifier) @external_arg_left
      operator: "+"
      right: (string) @external_arg_right)))

(call_expression
  function: (identifier) @external_callee
  arguments: (arguments
    (binary_expression
      left: (string) @external_arg_left
      operator: "+"
      right: (identifier) @external_arg_right)))

(call_expression
  function: (member_expression
    object: (identifier) @external_callee)
  arguments: (arguments
    (binary_expression
      left: (identifier) @external_arg_left
      operator: "+"
      right: (string) @external_arg_right)))

(call_expression
  function: (member_expression
    object: (identifier) @external_callee)
  arguments: (arguments
    (binary_expression
      left: (string) @external_arg_left
      operator: "+"
      right: (identifier) @external_arg_right)))

(call_expression
  function: (identifier) @external_callee
  arguments: (arguments
    (new_expression
      constructor: (identifier) @external_url_ctor
      arguments: (arguments (string) @external_url_path (string) @external_url_base))))

(call_expression
  function: (identifier) @external_callee
  arguments: (arguments
    (new_expression
      constructor: (identifier) @external_url_ctor
      arguments: (arguments (string) @external_url_path (identifier) @external_url_base_ident))))

(call_expression
  function: (member_expression
    object: (identifier) @external_callee)
  arguments: (arguments
    (new_expression
      constructor: (identifier) @external_url_ctor
      arguments: (arguments (string) @external_url_path (string) @external_url_base))))

(call_expression
  function: (member_expression
    object: (identifier) @external_callee)
  arguments: (arguments
    (new_expression
      constructor: (identifier) @external_url_ctor
      arguments: (arguments (string) @external_url_path (identifier) @external_url_base_ident))))

(lexical_declaration
  (variable_declarator
    name: (identifier) @const_name
    value: (string) @const_value))

(lexical_declaration
  (variable_declarator
    name: (identifier) @const_name
    value: (template_string) @const_value))

(lexical_declaration
  (variable_declarator
    name: (identifier) @const_name
    value: (binary_expression
      left: (string) @const_left
      operator: "+"
      right: (string) @const_right)))

(lexical_declaration
  (variable_declarator
    name: (identifier) @const_name
    value: (member_expression
      object: (member_expression
        object: (identifier) @env_root
        property: (property_identifier) @env_prop)
      property: (property_identifier) @env_key)))

(lexical_declaration
  (variable_declarator
    name: (identifier) @const_name
    value: (member_expression
      object: (member_expression
        object: (member_expression
          object: (identifier) @env_import
          property: (property_identifier) @env_meta)
        property: (property_identifier) @env_env)
      property: (property_identifier) @env_key)))

(import_clause
  name: (identifier) @import_name)

(import_clause
  (named_imports
    (import_specifier
      name: (identifier) @import_named)))

(import_clause
  (named_imports
    (import_specifier
      name: (property_identifier) @import_named)))

(import_clause
  (namespace_import
    (identifier) @import_star))
"#;

fn strip_string_literal(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.len() < 2 {
        return None;
    }
    let first = trimmed.chars().next()?;
    let last = trimmed.chars().last()?;
    let is_quote = (first == '"' && last == '"') || (first == '\'' && last == '\'') || (first == '`' && last == '`');
    if !is_quote {
        return None;
    }
    let inner = &trimmed[1..trimmed.len() - 1];
    if inner.contains("${") {
        return None;
    }
    Some(inner.to_string())
}

fn is_launch_callee(module: &str, callee: &str) -> bool {
    module == "subprocess" && matches!(callee, "run" | "Popen" | "call")
}

fn join_path_parts(parts: &[String]) -> Option<String> {
    if parts.is_empty() {
        return None;
    }
    let mut out = String::new();
    for part in parts {
        let trimmed = part.trim_matches('/');
        if trimmed.is_empty() {
            continue;
        }
        if !out.is_empty() {
            out.push('/');
        }
        out.push_str(trimmed);
    }
    if out.is_empty() { None } else { Some(out) }
}

fn is_join_fn(text: &str) -> bool {
    text.ends_with(".path.join") || text == "path.join" || text == "os.path.join"
}

fn debug_launch_enabled() -> bool {
    std::env::var("TS_PACK_DEBUG_LAUNCH")
        .ok()
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

#[derive(Clone, Copy)]
struct ScopeRange {
    start: usize,
    end: usize,
}

fn scope_for(node_start: usize, node_end: usize, scopes: &[ScopeRange]) -> usize {
    let mut best_idx = 0usize;
    let mut best_span = usize::MAX;
    for (idx, scope) in scopes.iter().enumerate() {
        if node_start >= scope.start && node_end <= scope.end {
            let span = scope.end.saturating_sub(scope.start);
            if span < best_span {
                best_idx = idx;
                best_span = span;
            }
        }
    }
    best_idx
}

fn resolve_python_launch_idents(tree: &ts_pack::Tree, source: &[u8]) -> Vec<String> {
    let mut scopes: Vec<ScopeRange> = Vec::new();
    if let Some(query) = ts_pack::get_locals_query("python") {
        match ts_pack::run_query(tree, "python", query, source) {
            Ok(matches) => {
                for m in matches {
                    for (cap, info) in m.captures {
                        if cap.as_ref() == "local.scope" {
                            scopes.push(ScopeRange {
                                start: info.start_byte,
                                end: info.end_byte,
                            });
                        }
                    }
                }
            }
            Err(err) => {
                if debug_launch_enabled() {
                    eprintln!("[ts-pack-index] launch locals query failed: {err}");
                }
            }
        }
    }
    if scopes.is_empty() {
        scopes.push(ScopeRange {
            start: 0,
            end: source.len(),
        });
    }

    let mut assignments: HashMap<(String, usize), Vec<(usize, Vec<String>)>> = HashMap::new();
    let mut join_calls: HashMap<(String, usize, usize), (Option<String>, Vec<String>, usize)> = HashMap::new();
    match ts_pack::run_query(tree, "python", PYTHON_LAUNCH_ASSIGN_TAGS, source) {
        Ok(matches) => {
            for m in matches {
                let mut name: Option<String> = None;
                let mut stmt: Option<ts_pack::NodeInfo> = None;
                let mut assign_strings: Vec<String> = Vec::new();
                let mut join_fn: Option<String> = None;
                let mut join_arg: Option<String> = None;
                let mut join_call: Option<ts_pack::NodeInfo> = None;

                for (cap, info) in m.captures {
                    match cap.as_ref() {
                        "launch_assign_name" => {
                            if let Ok(text) = ts_pack::extract_text(source, &info) {
                                name = Some(text.to_string());
                            }
                        }
                        "launch_assign_stmt" => {
                            stmt = Some(info);
                        }
                        "launch_assign_str" => {
                            if let Ok(text) = ts_pack::extract_text(source, &info) {
                                if let Some(literal) = strip_string_literal(text) {
                                    assign_strings.push(literal);
                                }
                            }
                        }
                        "launch_join_fn" => {
                            if let Ok(text) = ts_pack::extract_text(source, &info) {
                                join_fn = Some(text.to_string());
                            }
                        }
                        "launch_join_arg" => {
                            if let Ok(text) = ts_pack::extract_text(source, &info) {
                                if let Some(literal) = strip_string_literal(text) {
                                    join_arg = Some(literal);
                                }
                            }
                        }
                        "launch_join_call" => {
                            join_call = Some(info);
                        }
                        _ => {}
                    }
                }

                let Some(name) = name else {
                    continue;
                };
                let Some(stmt) = stmt else {
                    continue;
                };
                let stmt_start = stmt.start_byte;
                let scope_id = scope_for(stmt.start_byte, stmt.end_byte, &scopes);

                if let Some(literal) = join_arg {
                    let call = join_call.unwrap_or(stmt.clone());
                    let key = (name.clone(), scope_id, call.start_byte);
                    let entry = join_calls.entry(key).or_insert((None, Vec::new(), stmt_start));
                    if entry.0.is_none() {
                        entry.0 = join_fn.clone();
                    }
                    entry.1.push(literal);
                }

                if !assign_strings.is_empty() {
                    let mut paths: Vec<String> = Vec::new();
                    for literal in assign_strings {
                        if literal.ends_with(".py") {
                            paths.push(literal);
                        }
                    }
                    if !paths.is_empty() {
                        assignments
                            .entry((name, scope_id))
                            .or_default()
                            .push((stmt.start_byte, paths));
                    }
                }
            }
        }
        Err(err) => {
            if debug_launch_enabled() {
                eprintln!("[ts-pack-index] launch assign query failed: {err}");
            }
        }
    }

    for ((name, scope_id, _), (fn_text, args, stmt_start)) in join_calls {
        let Some(fn_text) = fn_text else {
            continue;
        };
        if !is_join_fn(&fn_text) {
            continue;
        }
        let Some(path) = join_path_parts(&args) else {
            continue;
        };
        if !path.ends_with(".py") {
            continue;
        }
        assignments
            .entry((name, scope_id))
            .or_default()
            .push((stmt_start, vec![path]));
    }

    for list in assignments.values_mut() {
        list.sort_by_key(|(start, _)| *start);
    }

    let mut calls: Vec<(String, usize, usize)> = Vec::new();
    match ts_pack::run_query(tree, "python", PYTHON_LAUNCH_IDENT_CALL_TAGS, source) {
        Ok(matches) => {
            for m in matches {
                let mut module: Option<String> = None;
                let mut callee: Option<String> = None;
                let mut arg_ident: Option<String> = None;
                let mut call_node: Option<ts_pack::NodeInfo> = None;

                for (cap, info) in m.captures {
                    match cap.as_ref() {
                        "launch_module" => {
                            if let Ok(text) = ts_pack::extract_text(source, &info) {
                                module = Some(text.to_string());
                            }
                        }
                        "launch_callee" => {
                            if let Ok(text) = ts_pack::extract_text(source, &info) {
                                callee = Some(text.to_string());
                            }
                        }
                        "launch_arg_ident" => {
                            if let Ok(text) = ts_pack::extract_text(source, &info) {
                                arg_ident = Some(text.to_string());
                            }
                        }
                        "launch_call" => {
                            call_node = Some(info);
                        }
                        _ => {}
                    }
                }

                let (Some(module), Some(callee), Some(arg_ident), Some(call_node)) =
                    (module, callee, arg_ident, call_node)
                else {
                    continue;
                };
                if !is_launch_callee(&module, &callee) {
                    continue;
                }
                let scope_id = scope_for(call_node.start_byte, call_node.end_byte, &scopes);
                calls.push((arg_ident, scope_id, call_node.start_byte));
            }
        }
        Err(err) => {
            if debug_launch_enabled() {
                eprintln!("[ts-pack-index] launch call query failed: {err}");
            }
        }
    }

    let mut out: Vec<String> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    let call_count = calls.len();
    for (ident, scope_id, call_start) in calls {
        let mut resolved: Vec<String> = Vec::new();
        if let Some(list) = assignments.get(&(ident.clone(), scope_id)) {
            for (start, paths) in list.iter().rev() {
                if *start <= call_start {
                    resolved = paths.clone();
                    break;
                }
            }
        }
        if resolved.is_empty() && scope_id != 0 {
            if let Some(list) = assignments.get(&(ident.clone(), 0)) {
                for (start, paths) in list.iter().rev() {
                    if *start <= call_start {
                        resolved = paths.clone();
                        break;
                    }
                }
            }
        }

        for path in resolved {
            if seen.insert(path.clone()) {
                out.push(path);
            }
        }
    }

    if debug_launch_enabled() {
        eprintln!(
            "[ts-pack-index] launch resolve: scopes={} assigns={} calls={} resolved={}",
            scopes.len(),
            assignments.len(),
            call_count,
            out.len()
        );
    }

    out
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// A single resolved call expression with its byte offset in the source.
#[derive(Clone)]
pub struct CallSite {
    /// Byte offset of the call expression start (for enclosing-scope lookup).
    pub start_byte: usize,
    /// Name of the function/method being called.
    pub callee: String,
    /// Receiver identifier for member calls.
    pub receiver: Option<String>,
}

#[derive(Clone)]
pub enum ExternalCallArg {
    Literal(String),
    Identifier(String),
    ConcatIdentLiteral { ident: String, literal: String },
    ConcatLiteralIdent { literal: String, ident: String },
    UrlLiteral { path: String, base: String },
    UrlWithBaseIdent { path: String, base_ident: String },
}

#[derive(Clone)]
pub struct ExternalCallSite {
    pub arg: ExternalCallArg,
}

/// Result of running the tags query on a single file.
pub struct TagsResult {
    /// Names of functions/classes that are exported (public API surface).
    pub exported_names: std::collections::HashSet<String>,
    /// All call sites found in this file (with byte position for scope lookup).
    pub call_sites: Vec<CallSite>,
    /// DB model references (currently Prisma delegates for ts/js).
    pub db_models: std::collections::HashSet<String>,
    /// External API call sites (js/ts only).
    pub external_calls: Vec<ExternalCallSite>,
    /// Constant string assignments (js/ts only).
    pub const_strings: std::collections::HashMap<String, String>,
    /// Subprocess launch script paths (python only).
    pub launch_calls: Vec<String>,
}

/// Run the tags query for `lang_name` against the already-parsed `tree`.
///
/// Returns `None` if there is no query configured for this language, or if
/// query compilation fails (e.g. the grammar uses different node type names).
pub fn run_tags(lang_name: &str, tree: &ts_pack::Tree, source: &[u8]) -> Option<TagsResult> {
    let query_str = tags_query(lang_name)?;

    // Split the multi-pattern query into individual patterns and try each one.
    // This way a single bad pattern doesn't kill the whole query for a language.
    let patterns = split_query_patterns(query_str);
    if patterns.is_empty() {
        return None;
    }

    let mut exported_names = std::collections::HashSet::new();
    let mut call_sites = Vec::new();
    let mut db_models = std::collections::HashSet::new();
    let mut external_calls = Vec::new();
    let mut const_strings = std::collections::HashMap::new();
    let mut launch_calls = Vec::new();

    let is_external_callee = |name: &str| matches!(name, "fetch" | "axios" | "ky" | "ofetch" | "$fetch");

    for pattern in &patterns {
        let matches = match ts_pack::run_query(tree, lang_name, pattern, source) {
            Ok(m) => m,
            Err(_) => continue, // invalid node type for this grammar — skip
        };

        for m in &matches {
            let is_export_pattern = m.captures.iter().any(|(cap, _)| cap == "exported");
            let mut has_vis = false;
            let mut def_name: Option<String> = None;
            // (start_byte, callee_name)
            let mut callee_site: Option<(usize, String)> = None;
            let mut receiver_name: Option<String> = None;
            let mut external_callee: Option<String> = None;
            let mut external_arg: Option<ExternalCallArg> = None;
            let mut external_arg_left: Option<String> = None;
            let mut external_arg_right: Option<String> = None;
            let mut external_arg_left_is_literal = false;
            let mut external_arg_right_is_literal = false;
            let mut const_name: Option<String> = None;
            let mut const_value: Option<String> = None;
            let mut const_left: Option<String> = None;
            let mut const_right: Option<String> = None;
            let mut external_url_ctor: Option<String> = None;
            let mut external_url_path: Option<String> = None;
            let mut external_url_base: Option<String> = None;
            let mut external_url_base_ident: Option<String> = None;
            let mut env_root: Option<String> = None;
            let mut env_prop: Option<String> = None;
            let mut env_import: Option<String> = None;
            let mut env_meta: Option<String> = None;
            let mut env_env: Option<String> = None;
            let mut env_key: Option<String> = None;
            let mut launch_module: Option<String> = None;
            let mut launch_callee: Option<String> = None;
            let mut launch_args: Vec<String> = Vec::new();

            for (cap_name, node_info) in &m.captures {
                let text = match ts_pack::extract_text(source, node_info) {
                    Ok(t) => t.to_string(),
                    Err(_) => continue,
                };

                match cap_name.as_ref() {
                    "vis" => {
                        if lang_name == "swift" {
                            let lowered = text.to_lowercase();
                            if lowered.contains("public") || lowered.contains("open") {
                                has_vis = true;
                            }
                        } else {
                            has_vis = true;
                        }
                    }
                    "name" => {
                        def_name = Some(text);
                    }
                    "callee" => {
                        callee_site = Some((node_info.start_byte, text));
                    }
                    "recv" => {
                        receiver_name = Some(text);
                    }
                    "db" => {
                        db_models.insert(text);
                    }
                    "external_callee" => {
                        external_callee = Some(text);
                    }
                    "external_arg" => {
                        if let Some(literal) = strip_string_literal(&text) {
                            external_arg = Some(ExternalCallArg::Literal(literal));
                        } else if text.starts_with('`') && text.contains("${") {
                            continue;
                        } else {
                            external_arg = Some(ExternalCallArg::Identifier(text));
                        }
                    }
                    "external_arg_left" => {
                        if let Some(literal) = strip_string_literal(&text) {
                            external_arg_left = Some(literal);
                            external_arg_left_is_literal = true;
                        } else {
                            external_arg_left = Some(text);
                            external_arg_left_is_literal = false;
                        }
                    }
                    "external_arg_right" => {
                        if let Some(literal) = strip_string_literal(&text) {
                            external_arg_right = Some(literal);
                            external_arg_right_is_literal = true;
                        } else {
                            external_arg_right = Some(text);
                            external_arg_right_is_literal = false;
                        }
                    }
                    "const_name" => {
                        const_name = Some(text);
                    }
                    "const_value" => {
                        const_value = strip_string_literal(&text);
                    }
                    "const_left" => {
                        const_left = strip_string_literal(&text);
                    }
                    "const_right" => {
                        const_right = strip_string_literal(&text);
                    }
                    "external_url_ctor" => {
                        external_url_ctor = Some(text);
                    }
                    "external_url_path" => {
                        external_url_path = strip_string_literal(&text);
                    }
                    "external_url_base" => {
                        external_url_base = strip_string_literal(&text);
                    }
                    "external_url_base_ident" => {
                        external_url_base_ident = Some(text);
                    }
                    "env_root" => {
                        env_root = Some(text);
                    }
                    "env_prop" => {
                        env_prop = Some(text);
                    }
                    "env_import" => {
                        env_import = Some(text);
                    }
                    "env_meta" => {
                        env_meta = Some(text);
                    }
                    "env_env" => {
                        env_env = Some(text);
                    }
                    "env_key" => {
                        env_key = Some(text);
                    }
                    "launch_module" => {
                        launch_module = Some(text);
                    }
                    "launch_callee" => {
                        launch_callee = Some(text);
                    }
                    "launch_arg" => {
                        if let Some(literal) = strip_string_literal(&text) {
                            launch_args.push(literal);
                        }
                    }
                    _ => {}
                }
            }

            if let Some(name) = def_name {
                if has_vis || is_export_pattern {
                    exported_names.insert(name);
                }
            }

            if let Some((start_byte, callee)) = callee_site {
                call_sites.push(CallSite {
                    start_byte,
                    callee,
                    receiver: receiver_name,
                });
            }

            if let Some(name) = const_name {
                if let Some(value) = const_value {
                    const_strings.insert(name, value);
                } else if let (Some(left), Some(right)) = (const_left, const_right) {
                    const_strings.insert(name, format!("{left}{right}"));
                } else if let (Some(key), Some(root), Some(prop)) = (env_key.clone(), env_root, env_prop) {
                    if root == "process" && prop == "env" {
                        const_strings.insert(name, format!("env://{key}"));
                    }
                } else if let (Some(key), Some(import), Some(meta), Some(env)) =
                    (env_key.clone(), env_import, env_meta, env_env)
                {
                    if import == "import" && meta == "meta" && env == "env" {
                        const_strings.insert(name, format!("env://{key}"));
                    }
                }
            }

            if let (Some(callee), Some(arg)) = (external_callee.as_ref(), external_arg) {
                if is_external_callee(callee.as_str()) {
                    external_calls.push(ExternalCallSite { arg });
                }
            } else if let (Some(callee), Some(left), Some(right)) =
                (external_callee.as_ref(), external_arg_left, external_arg_right)
            {
                if is_external_callee(callee.as_str()) {
                    if external_arg_left_is_literal && !external_arg_right_is_literal {
                        external_calls.push(ExternalCallSite {
                            arg: ExternalCallArg::ConcatLiteralIdent {
                                literal: left,
                                ident: right,
                            },
                        });
                    } else if !external_arg_left_is_literal && external_arg_right_is_literal {
                        external_calls.push(ExternalCallSite {
                            arg: ExternalCallArg::ConcatIdentLiteral {
                                ident: left,
                                literal: right,
                            },
                        });
                    } else if external_arg_left_is_literal && external_arg_right_is_literal {
                        external_calls.push(ExternalCallSite {
                            arg: ExternalCallArg::Literal(format!("{left}{right}")),
                        });
                    }
                }
            } else if let (Some(callee), Some(ctor), Some(path)) =
                (external_callee, external_url_ctor, external_url_path)
            {
                if is_external_callee(callee.as_str()) && ctor == "URL" {
                    if let Some(base) = external_url_base {
                        external_calls.push(ExternalCallSite {
                            arg: ExternalCallArg::UrlLiteral { path, base },
                        });
                    } else if let Some(base_ident) = external_url_base_ident {
                        external_calls.push(ExternalCallSite {
                            arg: ExternalCallArg::UrlWithBaseIdent { path, base_ident },
                        });
                    }
                }
            }

            if let (Some(module), Some(callee)) = (launch_module.as_ref(), launch_callee.as_ref()) {
                if is_launch_callee(module, callee.as_str()) {
                    for arg in &launch_args {
                        if arg.ends_with(".py") {
                            launch_calls.push(arg.clone());
                        }
                    }
                }
            }
        }
    }

    if lang_name == "python" {
        let extra = resolve_python_launch_idents(tree, source);
        if !extra.is_empty() {
            let mut seen: HashSet<String> = launch_calls.iter().cloned().collect();
            for item in extra {
                if seen.insert(item.clone()) {
                    launch_calls.push(item);
                }
            }
        }
    }

    Some(TagsResult {
        exported_names,
        call_sites,
        db_models,
        external_calls,
        const_strings,
        launch_calls,
    })
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

fn tags_query(lang: &str) -> Option<&'static str> {
    match lang {
        "rust" => Some(RUST_TAGS),
        "python" => Some(PYTHON_TAGS),
        "javascript" => Some(JS_TAGS),
        "typescript" | "tsx" => Some(TS_TAGS),
        "go" => Some(GO_TAGS),
        "swift" => Some(SWIFT_TAGS),
        _ => None,
    }
}

/// Naively split a multi-pattern query string into individual S-expression
/// patterns. Splits on blank lines between top-level patterns so that each
/// `run_query` call compiles exactly one pattern — allowing graceful failure.
fn split_query_patterns(query: &str) -> Vec<String> {
    let mut patterns = Vec::new();
    let mut current = String::new();
    let mut depth = 0i32;

    for line in query.lines() {
        let trimmed = line.trim();
        for ch in trimmed.chars() {
            if ch == '(' {
                depth += 1;
            } else if ch == ')' {
                depth -= 1;
            }
        }
        current.push_str(line);
        current.push('\n');

        // A complete top-level pattern: depth returns to 0 after a non-empty chunk.
        if depth == 0 && !current.trim().is_empty() {
            let pat = current.trim().to_string();
            if !pat.is_empty() && !pat.starts_with(';') {
                patterns.push(pat);
            }
            current.clear();
        }
    }

    // Push any remaining content
    let remainder = current.trim().to_string();
    if !remainder.is_empty() {
        patterns.push(remainder);
    }

    patterns
}

#[cfg(test)]
mod tests {
    use super::*;

    fn maybe_parse(lang: &str, source: &str) -> Option<ts_pack::Tree> {
        if !ts_pack::has_language(lang) {
            return None;
        }
        ts_pack::parse_string(lang, source.as_bytes()).ok()
    }

    #[test]
    fn extracts_javascript_external_calls_and_consts() {
        let source = r#"
        const API_BASE = "https://api.example.com";
        export const loadData = () => fetch(API_BASE + "/v1/items");
        const other = axios(new URL("/v2/stats", API_BASE));
        "#;
        let Some(tree) = maybe_parse("javascript", source) else {
            return;
        };
        let tags = run_tags("javascript", &tree, source.as_bytes()).expect("tags");

        assert!(tags.exported_names.contains("loadData"));
        assert_eq!(tags.const_strings.get("API_BASE"), Some(&"https://api.example.com".to_string()));
        assert_eq!(tags.external_calls.len(), 2);
        assert!(matches!(
            &tags.external_calls[0].arg,
            ExternalCallArg::ConcatIdentLiteral { ident, literal }
                if ident == "API_BASE" && literal == "/v1/items"
        ));
        assert!(matches!(
            &tags.external_calls[1].arg,
            ExternalCallArg::UrlWithBaseIdent { path, base_ident }
                if path == "/v2/stats" && base_ident == "API_BASE"
        ));
    }

    #[test]
    fn extracts_python_launch_calls_from_literals_and_ident_lists() {
        let source = r#"
        import subprocess

        CMD = ["python", "scripts/worker.py"]
        subprocess.Popen(["python", "scripts/direct.py"])
        subprocess.run(CMD)
        "#;
        let Some(tree) = maybe_parse("python", source) else {
            return;
        };
        let tags = run_tags("python", &tree, source.as_bytes()).expect("tags");

        assert!(tags.launch_calls.contains(&"scripts/direct.py".to_string()));
        assert!(tags.launch_calls.contains(&"scripts/worker.py".to_string()));
    }

    #[test]
    fn extracts_swift_receivers_for_navigation_calls() {
        let source = r#"
        public struct Service {
            func run() {
                self.worker.start()
            }
        }
        "#;
        let Some(tree) = maybe_parse("swift", source) else {
            return;
        };
        let tags = run_tags("swift", &tree, source.as_bytes()).expect("tags");

        assert!(tags.call_sites.iter().any(|c| c.callee == "start" && c.receiver.as_deref() == Some("self")));
    }
}
