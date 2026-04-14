use std::collections::HashMap;

use crate::{GoFunctionReturnAssignment, GoMethodReturnAssignment};

fn infer_ctor_return_type(expr: &str) -> Option<String> {
    let open = expr.find('(')?;
    let callee = expr[..open].trim();
    let base = callee.rsplit('.').next().unwrap_or(callee).trim();
    let stripped = base.strip_prefix("New")?;
    let mut chars = stripped.chars();
    let first = chars.next()?;
    if !first.is_uppercase() {
        return None;
    }
    Some(format!("{first}{}", chars.as_str()))
}

fn normalize_go_type_name(raw: &str) -> Option<String> {
    let ty = raw.trim().trim_start_matches('*').trim_start_matches("[]").trim();
    if ty.is_empty() {
        return None;
    }
    let short = ty.split('.').next_back().unwrap_or(ty).trim();
    let first = short.chars().next()?;
    if !first.is_uppercase() {
        return None;
    }
    Some(short.to_string())
}

fn infer_method_return_assignment(expr: &str) -> Option<(String, String)> {
    let open = expr.find('(')?;
    let callee = expr[..open].trim();
    let (receiver, method) = callee.rsplit_once('.')?;
    let receiver = receiver.trim().trim_start_matches('*');
    let method = method.trim();
    if receiver.is_empty() || method.is_empty() {
        return None;
    }
    Some((receiver.to_string(), method.to_string()))
}

fn infer_function_return_assignment(expr: &str) -> Option<String> {
    let open = expr.find('(')?;
    let callee = expr[..open].trim();
    if callee.is_empty() || callee.contains('.') {
        return None;
    }
    let first = callee.chars().next()?;
    if !(first.is_ascii_lowercase() || first.is_ascii_uppercase() || first == '_') {
        return None;
    }
    Some(callee.to_string())
}

pub(crate) fn parse_go_method_return_types(source: &str) -> HashMap<String, String> {
    let mut out = HashMap::new();
    for raw_line in source.lines() {
        let line = raw_line.trim();
        if !line.starts_with("func (") {
            continue;
        }
        let Some(close_recv) = line.find(')') else {
            continue;
        };
        let recv_sig = &line["func (".len()..close_recv];
        let recv_type = recv_sig
            .split_whitespace()
            .nth(1)
            .or_else(|| recv_sig.split_whitespace().next())
            .and_then(normalize_go_type_name);
        let Some(recv_type) = recv_type else {
            continue;
        };

        let rest = line[close_recv + 1..].trim();
        let Some(method_open) = rest.find('(') else {
            continue;
        };
        let method_name = rest[..method_open].trim();
        if method_name.is_empty() {
            continue;
        }
        let after_params = &rest[method_open + 1..];
        let Some(params_close_rel) = after_params.find(')') else {
            continue;
        };
        let return_sig = after_params[params_close_rel + 1..].trim().trim_end_matches('{').trim();
        let first_return = if let Some(inner) = return_sig.strip_prefix('(') {
            inner.split(',').next().map(str::trim)
        } else {
            return_sig.split_whitespace().next().map(str::trim)
        };
        let Some(first_return) = first_return.and_then(normalize_go_type_name) else {
            continue;
        };
        out.insert(format!("{recv_type}.{method_name}"), first_return);
    }
    out
}

pub(crate) fn parse_go_function_return_types(source: &str) -> HashMap<String, String> {
    let mut out = HashMap::new();
    for raw_line in source.lines() {
        let line = raw_line.trim();
        if !line.starts_with("func ") || line.starts_with("func (") {
            continue;
        }
        let rest = &line["func ".len()..];
        let Some(name_open) = rest.find('(') else {
            continue;
        };
        let function_name = rest[..name_open].trim();
        if function_name.is_empty() {
            continue;
        }
        let after_params = &rest[name_open + 1..];
        let Some(params_close_rel) = after_params.find(')') else {
            continue;
        };
        let return_sig = after_params[params_close_rel + 1..].trim().trim_end_matches('{').trim();
        let first_return = if let Some(inner) = return_sig.strip_prefix('(') {
            inner.split(',').next().map(str::trim)
        } else {
            return_sig.split_whitespace().next().map(str::trim)
        };
        let Some(first_return) = first_return.and_then(normalize_go_type_name) else {
            continue;
        };
        out.insert(function_name.to_string(), first_return);
    }
    out
}

pub(crate) fn parse_go_var_types(
    source: &str,
) -> (
    HashMap<String, String>,
    Vec<GoMethodReturnAssignment>,
    Vec<GoFunctionReturnAssignment>,
) {
    let mut out = HashMap::new();
    let mut method_assignments = Vec::new();
    let mut function_assignments = Vec::new();
    for raw_line in source.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with("//") {
            continue;
        }

        if let Some((lhs, rhs)) = line.split_once(":=") {
            let names: Vec<&str> = lhs
                .split(',')
                .map(|part| part.trim())
                .filter(|part| !part.is_empty())
                .collect();
            let exprs: Vec<&str> = rhs
                .split(',')
                .map(|part| part.trim())
                .filter(|part| !part.is_empty())
                .collect();
            if let Some(name) = names.first().copied()
                && let Some(expr) = exprs.first().copied()
            {
                if let Some(ty) = infer_ctor_return_type(expr) {
                    out.insert(name.to_string(), ty);
                } else if let Some((receiver_var, method_name)) = infer_method_return_assignment(expr) {
                    method_assignments.push(GoMethodReturnAssignment {
                        var_name: name.to_string(),
                        receiver_var,
                        method_name,
                    });
                } else if let Some(function_name) = infer_function_return_assignment(expr) {
                    function_assignments.push(GoFunctionReturnAssignment {
                        var_name: name.to_string(),
                        function_name,
                    });
                }
            }
            continue;
        }

        if let Some(stripped) = line.strip_prefix("var ")
            && let Some((lhs, rhs)) = stripped.split_once('=')
        {
            let name = lhs.split_whitespace().next().unwrap_or("").trim();
            let expr = rhs.trim();
            if !name.is_empty() {
                if let Some(ty) = infer_ctor_return_type(expr) {
                    out.insert(name.to_string(), ty);
                } else if let Some((receiver_var, method_name)) = infer_method_return_assignment(expr) {
                    method_assignments.push(GoMethodReturnAssignment {
                        var_name: name.to_string(),
                        receiver_var,
                        method_name,
                    });
                } else if let Some(function_name) = infer_function_return_assignment(expr) {
                    function_assignments.push(GoFunctionReturnAssignment {
                        var_name: name.to_string(),
                        function_name,
                    });
                }
            }
        }
    }
    (out, method_assignments, function_assignments)
}

#[cfg(test)]
mod tests {
    use super::{parse_go_function_return_types, parse_go_method_return_types, parse_go_var_types};

    #[test]
    fn infers_go_short_var_constructor_types() {
        let source = r#"
            registry, err := tslp.NewRegistry()
            _, _ = registry, err
        "#;
        let (vars, method_assignments, function_assignments) = parse_go_var_types(source);
        assert_eq!(vars.get("registry").map(String::as_str), Some("Registry"));
        assert!(method_assignments.is_empty());
        assert!(function_assignments.is_empty());
    }

    #[test]
    fn infers_go_var_assignment_constructor_types() {
        let source = r#"
            var registry = tslp.NewRegistry()
        "#;
        let (vars, method_assignments, function_assignments) = parse_go_var_types(source);
        assert_eq!(vars.get("registry").map(String::as_str), Some("Registry"));
        assert!(method_assignments.is_empty());
        assert!(function_assignments.is_empty());
    }

    #[test]
    fn captures_go_method_return_assignments() {
        let source = r#"
            tree, err := registry.ParseString("python", "code")
            _ = err
            var result = registry.Process("code", config)
        "#;
        let (vars, method_assignments, function_assignments) = parse_go_var_types(source);
        assert!(vars.is_empty());
        assert_eq!(method_assignments.len(), 2);
        assert!(function_assignments.is_empty());
        assert_eq!(method_assignments[0].var_name, "tree");
        assert_eq!(method_assignments[0].receiver_var, "registry");
        assert_eq!(method_assignments[0].method_name, "ParseString");
        assert_eq!(method_assignments[1].var_name, "result");
        assert_eq!(method_assignments[1].method_name, "Process");
    }

    #[test]
    fn parses_go_method_return_types() {
        let source = r#"
            func (r *Registry) ParseString(language, source string) (*Tree, error) { return nil, nil }
            func (r *Registry) Process(source string, config ProcessConfig) (*ProcessResult, error) { return nil, nil }
            func (t *Tree) RootNodeType() (string, error) { return "", nil }
        "#;
        let method_returns = parse_go_method_return_types(source);
        assert_eq!(
            method_returns.get("Registry.ParseString").map(String::as_str),
            Some("Tree")
        );
        assert_eq!(
            method_returns.get("Registry.Process").map(String::as_str),
            Some("ProcessResult")
        );
        assert!(!method_returns.contains_key("Tree.RootNodeType"));
    }

    #[test]
    fn captures_go_function_return_assignments() {
        let source = r#"
            reg := newTestRegistry(t)
        "#;
        let (vars, method_assignments, function_assignments) = parse_go_var_types(source);
        assert!(vars.is_empty());
        assert!(method_assignments.is_empty());
        assert_eq!(function_assignments.len(), 1);
        assert_eq!(function_assignments[0].var_name, "reg");
        assert_eq!(function_assignments[0].function_name, "newTestRegistry");
    }

    #[test]
    fn parses_go_function_return_types() {
        let source = r#"
            func newTestRegistry(t *testing.T) *tspack.Registry { return nil }
            func helper() string { return "" }
        "#;
        let function_returns = parse_go_function_return_types(source);
        assert_eq!(
            function_returns.get("newTestRegistry").map(String::as_str),
            Some("Registry")
        );
        assert!(!function_returns.contains_key("helper"));
    }
}
