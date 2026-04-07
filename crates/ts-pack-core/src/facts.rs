use ahash::AHashMap;
use std::path::Path;

use crate::Error;
use crate::extract::{CaptureOutput, ExtractionConfig, ExtractionPattern, ExtractionResult, MatchResult};

const HTTP_METHODS: &[&str] = &["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"];
const NON_HTTP_CLIENTS: &[&str] = &["router", "app", "server"];

#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FileFacts {
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Vec::is_empty", default))]
    pub route_defs: Vec<RouteDefFact>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Vec::is_empty", default))]
    pub http_calls: Vec<HttpCallFact>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Vec::is_empty", default))]
    pub resource_refs: Vec<ResourceRefFact>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RouteDefFact {
    pub framework: String,
    pub method: String,
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct HttpCallFact {
    pub client: String,
    pub method: String,
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ResourceRefFact {
    pub kind: String,
    pub name: String,
    pub callee: String,
}

pub fn extract_file_facts(source: &str, language: &str, file_path: Option<&str>) -> Result<FileFacts, Error> {
    let Some(config) = config_for_language(language) else {
        return Ok(FileFacts::default());
    };
    let raw = crate::extract_patterns(source, &config)?;
    Ok(parse_file_facts(&raw, language, file_path))
}

fn parse_file_facts(raw: &ExtractionResult, language: &str, file_path: Option<&str>) -> FileFacts {
    let mut facts = FileFacts::default();
    let lang = language.to_ascii_lowercase();

    if matches!(lang.as_str(), "typescript" | "tsx" | "javascript") {
        for m in pattern_matches(raw, "express_routes") {
            let caps = capture_texts(m);
            let method = normalize_method(first_capture(&caps, "method"));
            let path = first_capture(&caps, "path");
            if let (Some(method), Some(path)) = (method, path)
                && path.starts_with('/')
            {
                facts.route_defs.push(RouteDefFact {
                    framework: "express".to_string(),
                    method,
                    path: path.to_string(),
                });
            }
        }

        if let Some(inferred_path) = file_path.and_then(route_path_from_file) {
            for m in pattern_matches(raw, "route_methods") {
                let caps = capture_texts(m);
                if let Some(method) = normalize_method(first_capture(&caps, "method")) {
                    facts.route_defs.push(RouteDefFact {
                        framework: "file_route".to_string(),
                        method,
                        path: inferred_path.clone(),
                    });
                }
            }
        }

        let pending_methods: Vec<Option<String>> = pattern_matches(raw, "http_method_props")
            .iter()
            .map(|m| {
                let caps = capture_texts(m);
                normalize_method(first_capture(&caps, "method"))
            })
            .collect();

        for m in pattern_matches(raw, "http_member_calls") {
            let caps = capture_texts(m);
            let client = first_capture(&caps, "client");
            let method = normalize_method(first_capture(&caps, "method")).unwrap_or_else(|| "ANY".to_string());
            let path = first_capture(&caps, "path");
            if let (Some(client), Some(path)) = (client, path)
                && path.starts_with('/')
                && !NON_HTTP_CLIENTS.contains(&client)
            {
                facts.http_calls.push(HttpCallFact {
                    client: client.to_string(),
                    method,
                    path: path.to_string(),
                });
            }
        }

        for (idx, m) in pattern_matches(raw, "http_fetch_calls").iter().enumerate() {
            let caps = capture_texts(m);
            let client = first_capture(&caps, "client");
            let path = first_capture(&caps, "path");
            if let (Some(client), Some(path)) = (client, path)
                && path.starts_with('/')
            {
                facts.http_calls.push(HttpCallFact {
                    client: client.to_string(),
                    method: pending_methods
                        .get(idx)
                        .and_then(|v| v.clone())
                        .unwrap_or_else(|| "ANY".to_string()),
                    path: path.to_string(),
                });
            }
        }
    }

    if lang == "swift" {
        for m in pattern_matches(raw, "resource_calls") {
            let caps = capture_texts(m);
            let callee = first_capture(&caps, "callee");
            let name = first_capture(&caps, "name");
            let kind = match callee {
                Some("Image" | "UIImage" | "NSImage") => Some("image"),
                Some("Color") => Some("color"),
                Some("UINib" | "NSNib") => Some("nib"),
                _ => None,
            };
            if let (Some(kind), Some(callee), Some(name)) = (kind, callee, name) {
                facts.resource_refs.push(ResourceRefFact {
                    kind: kind.to_string(),
                    name: name.to_string(),
                    callee: callee.to_string(),
                });
            }
        }
    }

    facts.route_defs.sort();
    facts.route_defs.dedup();
    facts.http_calls.sort();
    facts.http_calls.dedup();
    facts.resource_refs.sort();
    facts.resource_refs.dedup();
    facts
}

fn first_capture<'a>(caps: &'a AHashMap<String, Vec<String>>, name: &str) -> Option<&'a str> {
    caps.get(name).and_then(|values| values.first().map(String::as_str))
}

fn capture_texts(m: &MatchResult) -> AHashMap<String, Vec<String>> {
    let mut out = AHashMap::new();
    for cap in &m.captures {
        if let Some(text) = &cap.text {
            out.entry(cap.name.clone()).or_insert_with(Vec::new).push(text.clone());
        }
    }
    out
}

fn pattern_matches<'a>(raw: &'a ExtractionResult, name: &str) -> &'a [MatchResult] {
    raw.results
        .get(name)
        .map(|entry| entry.matches.as_slice())
        .unwrap_or(&[])
}

fn normalize_method(value: Option<&str>) -> Option<String> {
    let method = value?.trim().to_ascii_uppercase();
    if HTTP_METHODS.contains(&method.as_str()) {
        Some(method)
    } else {
        None
    }
}

fn text_pattern(query: &str, max_results: usize) -> ExtractionPattern {
    ExtractionPattern {
        query: query.to_string(),
        capture_output: CaptureOutput::Text,
        child_fields: Vec::new(),
        max_results: Some(max_results),
        byte_range: None,
    }
}

fn config_for_language(language: &str) -> Option<ExtractionConfig> {
    let normalized = language.to_ascii_lowercase();
    let patterns = match normalized.as_str() {
        "javascript" | "typescript" | "tsx" => web_patterns(),
        "swift" => swift_patterns(),
        _ => return None,
    };
    Some(ExtractionConfig {
        language: normalized,
        patterns,
    })
}

fn web_patterns() -> AHashMap<String, ExtractionPattern> {
    let mut patterns = AHashMap::new();
    patterns.insert(
        "express_routes".to_string(),
        text_pattern(
            "(call_expression \
               function: (member_expression \
                 object: (identifier) @router \
                 property: (property_identifier) @method) \
               arguments: (arguments (string (string_fragment) @path))) @route_call",
            200,
        ),
    );
    patterns.insert(
        "http_member_calls".to_string(),
        text_pattern(
            "[(call_expression \
                function: (member_expression object: (identifier) @client property: (property_identifier) @method) \
                arguments: (arguments (string (string_fragment) @path))) \
              (call_expression \
                function: (member_expression object: (call_expression function: (identifier) @client) property: (property_identifier) @method) \
                arguments: (arguments (string (string_fragment) @path)))] @http_call",
            200,
        ),
    );
    patterns.insert(
        "http_fetch_calls".to_string(),
        text_pattern(
            "(call_expression \
               function: (identifier) @client \
               arguments: (arguments (string (string_fragment) @path))) @http_call \
             (#eq? @client \"fetch\")",
            200,
        ),
    );
    patterns.insert(
        "http_method_props".to_string(),
        text_pattern(
            "(pair \
               key: (property_identifier) @key \
               value: (string (string_fragment) @method)) @method_pair \
             (#eq? @key \"method\")",
            200,
        ),
    );
    patterns.insert(
        "route_methods".to_string(),
        text_pattern(
            "[(function_declaration name: (identifier) @method) \
              (lexical_declaration (variable_declarator name: (identifier) @method))]",
            50,
        ),
    );
    patterns
}

fn swift_patterns() -> AHashMap<String, ExtractionPattern> {
    let mut patterns = AHashMap::new();
    patterns.insert(
        "resource_calls".to_string(),
        text_pattern(
            "[(call_expression \
                called_expression: (simple_identifier) @callee \
                arguments: (call_suffix (value_arguments (value_argument (string_literal (string_literal_content) @name))))) \
              (call_expression \
                called_expression: (member_access_expr name: (simple_identifier) @callee) \
                arguments: (call_suffix (value_arguments (value_argument (string_literal (string_literal_content) @name)))))] @resource_call",
            200,
        ),
    );
    patterns
}

fn route_path_from_file(file_path: &str) -> Option<String> {
    let normalized = file_path.replace('\\', "/");
    let path = Path::new(&normalized);
    let parts: Vec<String> = path
        .components()
        .filter_map(|component| {
            let value = component.as_os_str().to_str()?;
            if value.is_empty() {
                None
            } else {
                Some(value.to_string())
            }
        })
        .collect();
    if parts.len() < 2 {
        return None;
    }

    let file_name = path.file_name()?.to_str()?;
    let mut idx = 0usize;
    while idx < parts.len() && matches!(parts[idx].as_str(), "packages" | "apps" | "src") {
        if matches!(parts[idx].as_str(), "packages" | "apps") && idx + 1 < parts.len() {
            idx += 2;
        } else {
            idx += 1;
        }
    }
    let relevant = &parts[idx..];
    if relevant.is_empty() {
        return None;
    }

    if relevant[0] == "app" && file_name.starts_with("route.") {
        let route_parts = &relevant[1..relevant.len().saturating_sub(1)];
        return Some(if route_parts.is_empty() {
            "/".to_string()
        } else {
            format!("/{}", route_parts.join("/"))
        });
    }

    if relevant.len() > 1 && relevant[0] == "pages" && relevant[1] == "api" {
        return route_path_from_segments(&relevant[2..]);
    }

    if relevant[0] == "api" {
        return route_path_from_segments(&relevant[1..]);
    }

    None
}

fn route_path_from_segments(segments: &[String]) -> Option<String> {
    if segments.is_empty() {
        return Some("/api".to_string());
    }
    let mut rel = segments.to_vec();
    let stem = Path::new(rel.last()?).file_stem()?.to_str()?.to_string();
    if matches!(stem.as_str(), "index" | "route") {
        rel.pop();
    } else if let Some(last) = rel.last_mut() {
        *last = stem;
    }

    Some(if rel.is_empty() {
        "/api".to_string()
    } else {
        format!("/api/{}", rel.join("/"))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_typescript_route_and_http_facts() {
        if !crate::has_language("typescript") {
            return;
        }

        let source = r#"
            export async function GET() {}
            router.post("/api/leases");
            const data = await fetch("/api/units", { method: "POST" });
            await client.get("/api/properties");
        "#;

        let facts = extract_file_facts(source, "typescript", Some("src/api/leases/route.ts")).unwrap();
        assert!(
            facts
                .route_defs
                .iter()
                .any(|item| item.method == "POST" && item.path == "/api/leases")
        );
        assert!(
            facts
                .route_defs
                .iter()
                .any(|item| item.method == "GET" && item.path == "/api/leases")
        );
        assert!(!facts.http_calls.iter().any(|item| item.client == "router"));
        assert!(
            facts
                .http_calls
                .iter()
                .any(|item| item.client == "fetch" && item.method == "POST" && item.path == "/api/units")
        );
        assert!(
            facts
                .http_calls
                .iter()
                .any(|item| item.client == "client" && item.method == "GET" && item.path == "/api/properties")
        );
    }

    #[test]
    fn extracts_swift_resource_refs() {
        if !crate::has_language("swift") {
            return;
        }

        let source = r#"
            let image = Image("hero")
            let color = Color("brand")
            let nib = UINib(nibName: "MainView", bundle: nil)
        "#;

        let facts = extract_file_facts(source, "swift", None).unwrap();
        assert!(
            facts
                .resource_refs
                .iter()
                .any(|item| item.kind == "image" && item.name == "hero")
        );
        assert!(
            facts
                .resource_refs
                .iter()
                .any(|item| item.kind == "color" && item.name == "brand")
        );
        assert!(
            facts
                .resource_refs
                .iter()
                .any(|item| item.kind == "nib" && item.name == "MainView")
        );
    }

    #[test]
    fn infers_route_path_from_file_layout() {
        assert_eq!(
            route_path_from_file("src/pages/api/users/index.ts"),
            Some("/api/users".to_string())
        );
        assert_eq!(
            route_path_from_file("apps/web/src/app/projects/[id]/route.ts"),
            Some("/projects/[id]".to_string())
        );
    }
}
