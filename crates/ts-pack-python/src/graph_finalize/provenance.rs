use neo4rs::{Graph, query};
use serde_json::{Value, json};
use std::sync::Arc;
use ts_pack_index::{graph_schema, provenance};

fn normalize_filter(value: Option<&str>) -> Option<String> {
    value.map(|v| v.trim().to_ascii_lowercase()).filter(|v| !v.is_empty())
}

fn contains_normalized(haystack: &str, needle: &str) -> bool {
    haystack.to_ascii_lowercase().contains(needle)
}

fn explicit_call_matches(src: &str, callee: &str, symbol_filter: Option<&str>, file_filter: Option<&str>) -> bool {
    let symbol_match = symbol_filter.is_none_or(|needle| contains_normalized(callee, needle));
    let file_match = file_filter.is_none_or(|needle| contains_normalized(&src.replace('\\', "/"), needle));
    symbol_match && file_match
}

fn explicit_file_pair_matches(src: &str, dst: &str, file_filter: Option<&str>) -> bool {
    file_filter.is_none_or(|needle| {
        contains_normalized(&src.replace('\\', "/"), needle) || contains_normalized(&dst.replace('\\', "/"), needle)
    })
}

pub(super) async fn emit_calls_file_samples(
    graph: &Arc<Graph>,
    project_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    if !provenance::provenance_enabled() {
        return Ok(());
    }
    let cypher = format!(
        "MATCH (src:{file_label} {{project_id:$pid}})-[:{contains_rel}]->(caller:Node {{project_id:$pid}})
         MATCH (caller)-[call:{calls_rel}|{calls_inferred_rel}]->(callee:Node {{project_id:$pid}})
         MATCH (dst:{file_label} {{project_id:$pid}})-[:{contains_rel}]->(callee)
         WHERE src <> dst
         RETURN src.filepath AS src, dst.filepath AS dst, caller.name AS caller, callee.name AS callee, type(call) AS via
         LIMIT 50",
        file_label = graph_schema::NODE_LABEL_FILE,
        contains_rel = graph_schema::REL_CONTAINS,
        calls_rel = graph_schema::REL_CALLS,
        calls_inferred_rel = graph_schema::REL_CALLS_INFERRED,
    );
    let mut rows = graph
        .execute(query(&cypher).param("pid", project_id.to_string()))
        .await?;
    while let Some(row) = rows.next().await? {
        let src: String = row.get("src").unwrap_or_default();
        let dst: String = row.get("dst").unwrap_or_default();
        let callee: String = row.get("callee").unwrap_or_default();
        if !provenance::file_pair_matches(&src, &dst) && !provenance::call_matches(&src, &callee, None, None) {
            continue;
        }
        provenance::emit(
            "finalize",
            "calls_file",
            &[
                ("src", src),
                ("dst", dst),
                ("caller", row.get::<String>("caller").unwrap_or_default()),
                ("callee", callee),
                ("via", row.get::<String>("via").unwrap_or_default()),
            ],
        );
    }
    Ok(())
}

pub async fn collect_provenance_report_async(
    neo4j_uri: &str,
    neo4j_user: &str,
    neo4j_pass: &str,
    neo4j_db: &str,
    project_id: &str,
    symbol_filter: Option<&str>,
    file_filter: Option<&str>,
) -> Result<Value, Box<dyn std::error::Error>> {
    let config = neo4rs::ConfigBuilder::default()
        .uri(neo4j_uri)
        .user(neo4j_user)
        .password(neo4j_pass)
        .db(neo4j_db)
        .max_connections(4)
        .fetch_size(200)
        .build()?;
    let graph = Arc::new(Graph::connect(config).await?);
    let symbol_filter = normalize_filter(symbol_filter);
    let file_filter = normalize_filter(file_filter);

    let mut calls_file_samples = Vec::new();
    let calls_file_cypher = format!(
        "MATCH (src:{file_label} {{project_id:$pid}})-[:{contains_rel}]->(caller:Node {{project_id:$pid}})
         MATCH (caller)-[call:{calls_rel}|{calls_inferred_rel}]->(callee:Node {{project_id:$pid}})
         MATCH (dst:{file_label} {{project_id:$pid}})-[:{contains_rel}]->(callee)
         WHERE src <> dst
         RETURN src.filepath AS src, dst.filepath AS dst, caller.name AS caller, callee.name AS callee, type(call) AS via
         LIMIT 100",
        file_label = graph_schema::NODE_LABEL_FILE,
        contains_rel = graph_schema::REL_CONTAINS,
        calls_rel = graph_schema::REL_CALLS,
        calls_inferred_rel = graph_schema::REL_CALLS_INFERRED,
    );
    let mut calls_rows = graph
        .execute(query(&calls_file_cypher).param("pid", project_id.to_string()))
        .await?;
    while let Some(row) = calls_rows.next().await? {
        let src: String = row.get("src").unwrap_or_default();
        let dst: String = row.get("dst").unwrap_or_default();
        let callee: String = row.get("callee").unwrap_or_default();
        if !explicit_file_pair_matches(&src, &dst, file_filter.as_deref())
            && !explicit_call_matches(&src, &callee, symbol_filter.as_deref(), file_filter.as_deref())
        {
            continue;
        }
        calls_file_samples.push(json!({
            "src": src,
            "dst": dst,
            "caller": row.get::<String>("caller").unwrap_or_default(),
            "callee": callee,
            "via": row.get::<String>("via").unwrap_or_default(),
        }));
    }

    let mut file_graph_link_samples = Vec::new();
    for rel in [
        graph_schema::REL_IMPORTS,
        graph_schema::REL_ASSET_LINKS,
        graph_schema::REL_CALLS_API,
        graph_schema::REL_CALLS_SERVICE,
        graph_schema::REL_CALLS_DB,
        graph_schema::REL_CALLS_FILE,
        graph_schema::REL_CALLS_API_ROUTE,
    ] {
        let cypher = if rel == graph_schema::REL_CALLS_API_ROUTE {
            format!(
                "MATCH (src:{file_label} {{project_id:$pid}})-[:{calls_api_route_rel}]->(route:{api_route_label} {{project_id:$pid}})-[:{handled_by_rel}]->(dst:{file_label} {{project_id:$pid}})
                 WHERE src <> dst
                 RETURN src.filepath AS src, dst.filepath AS dst, route.path AS route, route.method AS method
                 LIMIT 50",
                file_label = graph_schema::NODE_LABEL_FILE,
                api_route_label = graph_schema::NODE_LABEL_API_ROUTE,
                calls_api_route_rel = graph_schema::REL_CALLS_API_ROUTE,
                handled_by_rel = graph_schema::REL_HANDLED_BY,
            )
        } else {
            format!(
                "MATCH (src:{file_label} {{project_id:$pid}})-[:{rel_type}]->(dst:{file_label} {{project_id:$pid}})
                 WHERE src <> dst
                 RETURN src.filepath AS src, dst.filepath AS dst
                 LIMIT 50",
                file_label = graph_schema::NODE_LABEL_FILE,
                rel_type = rel,
            )
        };
        let mut rows = graph
            .execute(query(&cypher).param("pid", project_id.to_string()))
            .await?;
        while let Some(row) = rows.next().await? {
            let src: String = row.get("src").unwrap_or_default();
            let dst: String = row.get("dst").unwrap_or_default();
            if !explicit_file_pair_matches(&src, &dst, file_filter.as_deref()) {
                continue;
            }
            file_graph_link_samples.push(json!({
                "src": src,
                "dst": dst,
                "source_rel": rel,
                "route": row.get::<String>("route").unwrap_or_default(),
                "method": row.get::<String>("method").unwrap_or_default(),
            }));
        }
    }

    Ok(json!({
        "project_id": project_id,
        "symbol_filter": symbol_filter,
        "file_filter": file_filter,
        "finalize": {
            "calls_file_samples": calls_file_samples,
            "file_graph_link_samples": file_graph_link_samples,
        }
    }))
}

pub(super) async fn emit_file_graph_link_samples(
    graph: &Arc<Graph>,
    project_id: &str,
    rel: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    if !provenance::provenance_enabled() {
        return Ok(());
    }
    let cypher = format!(
        "MATCH (src:{file_label} {{project_id:$pid}})-[:{rel_type}]->(dst:{file_label} {{project_id:$pid}})
         WHERE src <> dst
         RETURN src.filepath AS src, dst.filepath AS dst
         LIMIT 50",
        file_label = graph_schema::NODE_LABEL_FILE,
        rel_type = rel,
    );
    let mut rows = graph
        .execute(query(&cypher).param("pid", project_id.to_string()))
        .await?;
    while let Some(row) = rows.next().await? {
        let src: String = row.get("src").unwrap_or_default();
        let dst: String = row.get("dst").unwrap_or_default();
        if !provenance::file_pair_matches(&src, &dst) {
            continue;
        }
        provenance::emit(
            "finalize",
            "file_graph_link",
            &[("src", src), ("dst", dst), ("source_rel", rel.to_string())],
        );
    }
    Ok(())
}

pub(super) async fn emit_api_route_link_samples(
    graph: &Arc<Graph>,
    project_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    if !provenance::provenance_enabled() {
        return Ok(());
    }
    let cypher = format!(
        "MATCH (src:{file_label} {{project_id:$pid}})-[:{calls_api_route_rel}]->(route:{api_route_label} {{project_id:$pid}})-[:{handled_by_rel}]->(dst:{file_label} {{project_id:$pid}})
         WHERE src <> dst
         RETURN src.filepath AS src, dst.filepath AS dst, route.path AS path, route.method AS method
         LIMIT 50",
        file_label = graph_schema::NODE_LABEL_FILE,
        api_route_label = graph_schema::NODE_LABEL_API_ROUTE,
        calls_api_route_rel = graph_schema::REL_CALLS_API_ROUTE,
        handled_by_rel = graph_schema::REL_HANDLED_BY,
    );
    let mut rows = graph
        .execute(query(&cypher).param("pid", project_id.to_string()))
        .await?;
    while let Some(row) = rows.next().await? {
        let src: String = row.get("src").unwrap_or_default();
        let dst: String = row.get("dst").unwrap_or_default();
        if !provenance::file_pair_matches(&src, &dst) {
            continue;
        }
        provenance::emit(
            "finalize",
            "file_graph_link",
            &[
                ("src", src),
                ("dst", dst),
                ("source_rel", graph_schema::REL_CALLS_API_ROUTE.to_string()),
                ("route", row.get::<String>("path").unwrap_or_default()),
                ("method", row.get::<String>("method").unwrap_or_default()),
            ],
        );
    }
    Ok(())
}
