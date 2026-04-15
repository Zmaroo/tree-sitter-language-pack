use neo4rs::{Graph, query};
use std::sync::Arc;
use ts_pack_index::{graph_schema, provenance};

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
