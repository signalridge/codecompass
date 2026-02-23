use crate::protocol::{JsonRpcRequest, JsonRpcResponse, ProtocolMetadata};
use crate::tools;
use codecompass_core::config::Config;
use codecompass_core::constants;
use codecompass_core::error::StateError;
use codecompass_core::types::{FreshnessStatus, SchemaStatus, generate_project_id};
use codecompass_query::locate;
use codecompass_query::search;
use codecompass_state::tantivy_index::IndexSet;
use serde_json::{Value, json};
use std::io::{self, BufRead, Write};
use std::path::Path;
use std::process::Stdio;
use tracing::{error, info};

/// Run the MCP server loop on stdin/stdout.
pub fn run_server(
    workspace: &Path,
    config_file: Option<&Path>,
) -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::load_with_file(Some(workspace), config_file)?;
    let project_id = generate_project_id(&workspace.to_string_lossy());
    let data_dir = config.project_data_dir(&project_id);
    let db_path = data_dir.join(constants::STATE_DB_FILE);

    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut stdout = stdout.lock();

    info!("MCP server started");

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(e) => {
                error!("stdin read error: {}", e);
                break;
            }
        };

        if line.trim().is_empty() {
            continue;
        }

        let request: JsonRpcRequest = match serde_json::from_str(&line) {
            Ok(r) => r,
            Err(e) => {
                let resp = JsonRpcResponse::error(None, -32700, format!("Parse error: {}", e));
                writeln!(stdout, "{}", serde_json::to_string(&resp)?)?;
                stdout.flush()?;
                continue;
            }
        };

        let index_runtime = load_index_runtime(&data_dir);
        let conn = codecompass_state::db::open_connection(&db_path).ok();
        let response = handle_request(
            &request,
            &config,
            index_runtime.index_set.as_ref(),
            index_runtime.schema_status,
            index_runtime.compatibility_reason.as_deref(),
            conn.as_ref(),
            workspace,
            &project_id,
        );
        writeln!(stdout, "{}", serde_json::to_string(&response)?)?;
        stdout.flush()?;
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn handle_request(
    request: &JsonRpcRequest,
    _config: &Config,
    index_set: Option<&IndexSet>,
    schema_status: SchemaStatus,
    compatibility_reason: Option<&str>,
    conn: Option<&rusqlite::Connection>,
    workspace: &Path,
    project_id: &str,
) -> JsonRpcResponse {
    match request.method.as_str() {
        "initialize" => JsonRpcResponse::success(
            request.id.clone(),
            json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {
                    "tools": {}
                },
                "serverInfo": {
                    "name": "codecompass",
                    "version": env!("CARGO_PKG_VERSION")
                }
            }),
        ),
        "notifications/initialized" => JsonRpcResponse::success(request.id.clone(), json!({})),
        "tools/list" => {
            let tools = tools::list_tools();
            JsonRpcResponse::success(request.id.clone(), json!({ "tools": tools }))
        }
        "tools/call" => {
            let tool_name = request
                .params
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let arguments = request
                .params
                .get("arguments")
                .cloned()
                .unwrap_or(json!({}));

            handle_tool_call(
                request.id.clone(),
                tool_name,
                &arguments,
                index_set,
                schema_status,
                compatibility_reason,
                conn,
                workspace,
                project_id,
            )
        }
        _ => JsonRpcResponse::error(
            request.id.clone(),
            -32601,
            format!("Method not found: {}", request.method),
        ),
    }
}

struct IndexRuntime {
    index_set: Option<IndexSet>,
    schema_status: SchemaStatus,
    compatibility_reason: Option<String>,
}

fn load_index_runtime(data_dir: &Path) -> IndexRuntime {
    match IndexSet::open_existing(data_dir) {
        Ok(index_set) => IndexRuntime {
            index_set: Some(index_set),
            schema_status: SchemaStatus::Compatible,
            compatibility_reason: None,
        },
        Err(err) => {
            let (schema_status, compatibility_reason) = classify_index_open_error(&err);
            IndexRuntime {
                index_set: None,
                schema_status,
                compatibility_reason: Some(compatibility_reason),
            }
        }
    }
}

fn classify_index_open_error(err: &StateError) -> (SchemaStatus, String) {
    match err {
        StateError::Io(io_err) if io_err.kind() == std::io::ErrorKind::NotFound => (
            SchemaStatus::NotIndexed,
            "No index found. Run `codecompass index`.".to_string(),
        ),
        StateError::SchemaMigrationRequired { current, required } => (
            SchemaStatus::ReindexRequired,
            format!(
                "Index schema is incompatible (current={}, required={}).",
                current, required
            ),
        ),
        StateError::CorruptManifest(details) => (
            SchemaStatus::CorruptManifest,
            format!("Index appears corrupted: {}", details),
        ),
        StateError::Tantivy(details) => (
            SchemaStatus::CorruptManifest,
            format!("Index open failed: {}", details),
        ),
        other => (
            SchemaStatus::CorruptManifest,
            format!("Index open failed: {}", other),
        ),
    }
}

/// Check if there's an active indexing job.
fn has_active_job(conn: Option<&rusqlite::Connection>, project_id: &str) -> bool {
    conn.and_then(|c| {
        codecompass_state::jobs::get_active_job(c, project_id)
            .ok()
            .flatten()
    })
    .is_some()
}

/// Build protocol metadata aware of current state.
fn build_metadata(
    r#ref: &str,
    schema_status: SchemaStatus,
    conn: Option<&rusqlite::Connection>,
    workspace: &Path,
    project_id: &str,
) -> ProtocolMetadata {
    match schema_status {
        SchemaStatus::NotIndexed => ProtocolMetadata::not_indexed(r#ref),
        SchemaStatus::ReindexRequired => ProtocolMetadata::reindex_required(r#ref),
        SchemaStatus::CorruptManifest => ProtocolMetadata::corrupt_manifest(r#ref),
        SchemaStatus::Compatible => {
            let active = has_active_job(conn, project_id);
            let mut metadata = ProtocolMetadata::new(r#ref).with_active_job(active);
            if !active && is_ref_stale(conn, workspace, project_id, r#ref) {
                metadata.freshness_status = FreshnessStatus::Stale;
            }
            metadata
        }
    }
}

fn is_ref_stale(
    conn: Option<&rusqlite::Connection>,
    workspace: &Path,
    project_id: &str,
    r#ref: &str,
) -> bool {
    let Some(conn) = conn else {
        return false;
    };
    let Ok(Some(branch_state)) =
        codecompass_state::branch_state::get_branch_state(conn, project_id, r#ref)
    else {
        return false;
    };
    let Ok(head_branch) = codecompass_core::vcs::detect_head_branch(workspace) else {
        return false;
    };
    if head_branch != r#ref {
        return false;
    }
    let Ok(head_commit) = codecompass_core::vcs::detect_head_commit(workspace) else {
        return false;
    };
    branch_state.last_indexed_commit != head_commit
}

fn is_project_registered(conn: Option<&rusqlite::Connection>, workspace: &Path) -> bool {
    conn.and_then(|c| {
        codecompass_state::project::get_by_root(c, &workspace.to_string_lossy())
            .ok()
            .flatten()
    })
    .is_some()
}

/// Resolve the effective ref used by MCP tools.
///
/// Priority:
/// 1. Explicit `ref` argument
/// 2. Current HEAD branch (if available)
/// 3. Project default_ref from SQLite metadata
/// 4. `live` fallback
fn resolve_tool_ref(
    requested_ref: Option<&str>,
    workspace: &Path,
    conn: Option<&rusqlite::Connection>,
    project_id: &str,
) -> String {
    if let Some(r) = requested_ref {
        return r.to_string();
    }
    if let Ok(branch) = codecompass_core::vcs::detect_head_branch(workspace) {
        return branch;
    }
    if let Some(c) = conn
        && let Ok(Some(project)) = codecompass_state::project::get_by_id(c, project_id)
        && !project.default_ref.trim().is_empty()
    {
        return project.default_ref;
    }
    constants::REF_LIVE.to_string()
}

#[allow(clippy::too_many_arguments)]
fn handle_tool_call(
    id: Option<Value>,
    tool_name: &str,
    arguments: &Value,
    index_set: Option<&IndexSet>,
    schema_status: SchemaStatus,
    compatibility_reason: Option<&str>,
    conn: Option<&rusqlite::Connection>,
    workspace: &Path,
    project_id: &str,
) -> JsonRpcResponse {
    match tool_name {
        "locate_symbol" => {
            let name = arguments.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let kind = arguments.get("kind").and_then(|v| v.as_str());
            let language = arguments.get("language").and_then(|v| v.as_str());
            let requested_ref = arguments.get("ref").and_then(|v| v.as_str());
            let limit = arguments
                .get("limit")
                .and_then(|v| v.as_u64())
                .unwrap_or(10) as usize;
            let effective_ref = resolve_tool_ref(requested_ref, workspace, conn, project_id);
            let metadata =
                build_metadata(&effective_ref, schema_status, conn, workspace, project_id);

            if name.trim().is_empty() {
                return tool_error_response(
                    id,
                    "invalid_input",
                    "Parameter `name` is required.",
                    None,
                    metadata,
                );
            }

            let Some(index_set) = index_set else {
                return tool_compatibility_error(
                    id,
                    schema_status,
                    compatibility_reason,
                    conn,
                    workspace,
                    project_id,
                    &effective_ref,
                );
            };

            if schema_status != SchemaStatus::Compatible {
                return tool_compatibility_error(
                    id,
                    schema_status,
                    compatibility_reason,
                    conn,
                    workspace,
                    project_id,
                    &effective_ref,
                );
            }

            match locate::locate_symbol(
                &index_set.symbols,
                name,
                kind,
                language,
                Some(&effective_ref),
                limit,
            ) {
                Ok(results) => {
                    let response = json!({
                        "results": results,
                        "total_candidates": results.len(),
                        "metadata": metadata,
                    });
                    tool_text_response(id, response)
                }
                Err(e) => {
                    let (code, message, data) = map_state_error(&e);
                    tool_error_response(id, code, message, data, metadata)
                }
            }
        }
        "search_code" => {
            let query = arguments
                .get("query")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let requested_ref = arguments.get("ref").and_then(|v| v.as_str());
            let language = arguments.get("language").and_then(|v| v.as_str());
            let limit = arguments
                .get("limit")
                .and_then(|v| v.as_u64())
                .unwrap_or(10) as usize;
            let effective_ref = resolve_tool_ref(requested_ref, workspace, conn, project_id);
            let metadata =
                build_metadata(&effective_ref, schema_status, conn, workspace, project_id);

            if query.trim().is_empty() {
                return tool_error_response(
                    id,
                    "invalid_input",
                    "Parameter `query` is required.",
                    None,
                    metadata,
                );
            }

            let Some(index_set) = index_set else {
                return tool_compatibility_error(
                    id,
                    schema_status,
                    compatibility_reason,
                    conn,
                    workspace,
                    project_id,
                    &effective_ref,
                );
            };

            if schema_status != SchemaStatus::Compatible {
                return tool_compatibility_error(
                    id,
                    schema_status,
                    compatibility_reason,
                    conn,
                    workspace,
                    project_id,
                    &effective_ref,
                );
            }

            match search::search_code(
                index_set,
                conn,
                query,
                Some(&effective_ref),
                language,
                limit,
            ) {
                Ok(response) => {
                    let mut result = json!({
                        "results": &response.results,
                        "query_intent": &response.query_intent,
                        "total_candidates": response.total_candidates,
                        "suggested_next_actions": &response.suggested_next_actions,
                        "metadata": metadata,
                    });
                    if let Some(debug_payload) = &response.debug
                        && let Ok(value) = serde_json::to_value(debug_payload)
                    {
                        result["debug"] = value;
                    }
                    tool_text_response(id, result)
                }
                Err(e) => {
                    let (code, message, data) = map_state_error(&e);
                    tool_error_response(id, code, message, data, metadata)
                }
            }
        }
        "index_status" => {
            let requested_ref = arguments.get("ref").and_then(|v| v.as_str());
            let effective_ref = resolve_tool_ref(requested_ref, workspace, conn, project_id);
            let (status, schema_status_str) = match schema_status {
                SchemaStatus::Compatible => ("ready", "compatible"),
                SchemaStatus::NotIndexed => ("not_indexed", "not_indexed"),
                SchemaStatus::ReindexRequired => ("not_indexed", "reindex_required"),
                SchemaStatus::CorruptManifest => ("not_indexed", "corrupt_manifest"),
            };

            // Gather counts from SQLite if available
            let (file_count, symbol_count) = conn
                .map(|c| {
                    let fc = codecompass_state::manifest::file_count(c, project_id, &effective_ref)
                        .unwrap_or(0);
                    let sc =
                        codecompass_state::symbols::symbol_count(c, project_id, &effective_ref)
                            .unwrap_or(0);
                    (fc, sc)
                })
                .unwrap_or((0, 0));

            // Get recent jobs
            let recent_jobs = conn
                .and_then(|c| codecompass_state::jobs::get_recent_jobs(c, project_id, 5).ok())
                .unwrap_or_default();

            let active_job = conn.and_then(|c| {
                codecompass_state::jobs::get_active_job(c, project_id)
                    .ok()
                    .flatten()
            });

            // Derive last_indexed_at from the most recent published job for this ref
            let last_indexed_at: Option<String> = recent_jobs
                .iter()
                .find(|j| j.status == "published" && j.r#ref == effective_ref)
                .map(|j| j.updated_at.clone());

            let result = json!({
                "project_id": project_id,
                "repo_root": workspace.to_string_lossy(),
                "index_status": status,
                "schema_status": schema_status_str,
                "current_schema_version": constants::SCHEMA_VERSION,
                "required_schema_version": constants::SCHEMA_VERSION,
                "last_indexed_at": last_indexed_at,
                "ref": effective_ref,
                "file_count": file_count,
                "symbol_count": symbol_count,
                "compatibility_reason": compatibility_reason,
                "active_job": active_job.map(|j| json!({
                    "job_id": j.job_id,
                    "mode": j.mode,
                    "status": j.status,
                    "ref": j.r#ref,
                })),
                "recent_jobs": recent_jobs.iter().map(|j| json!({
                    "job_id": j.job_id,
                    "ref": j.r#ref,
                    "mode": j.mode,
                    "status": j.status,
                    "changed_files": j.changed_files,
                    "duration_ms": j.duration_ms,
                    "created_at": j.created_at,
                })).collect::<Vec<_>>(),
                "metadata": build_metadata(&effective_ref, schema_status, conn, workspace, project_id),
            });
            tool_text_response(id, result)
        }
        "index_repo" | "sync_repo" => {
            let force = arguments
                .get("force")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let mode = if force { "full" } else { "incremental" };
            let requested_ref = arguments.get("ref").and_then(|v| v.as_str());
            let effective_ref = resolve_tool_ref(requested_ref, workspace, conn, project_id);
            let metadata =
                build_metadata(&effective_ref, schema_status, conn, workspace, project_id);

            if !is_project_registered(conn, workspace) {
                return tool_error_response(
                    id,
                    "project_not_found",
                    "Project is not initialized for this workspace. Run `codecompass init` first.",
                    Some(json!({
                        "workspace": workspace.to_string_lossy(),
                        "remediation": "codecompass init --path <workspace>",
                    })),
                    metadata,
                );
            }
            if has_active_job(conn, project_id) {
                return tool_error_response(
                    id,
                    "index_in_progress",
                    "An indexing job is already running.",
                    Some(json!({
                        "project_id": project_id,
                        "remediation": "Use index_status to poll and retry after completion.",
                    })),
                    metadata,
                );
            }

            // Use current_exe() to find the binary reliably (works in MCP agent setups)
            let exe = std::env::current_exe().unwrap_or_else(|_| "codecompass".into());
            let workspace_str = workspace.to_string_lossy();
            let job_id = format!("{:016x}", rand_u64());

            let mut cmd = std::process::Command::new(exe);
            cmd.arg("index")
                .arg("--path")
                .arg(workspace_str.as_ref())
                .env("CODECOMPASS_JOB_ID", &job_id)
                .stdout(Stdio::null())
                .stderr(Stdio::null());
            if force {
                cmd.arg("--force");
            }
            // Pass the resolved ref so the subprocess uses the same scope and
            // avoids divergent fallback behavior.
            cmd.arg("--ref").arg(&effective_ref);

            match cmd.spawn() {
                Ok(child) => {
                    // Reap the child in a background thread to avoid zombie processes
                    std::thread::spawn(move || {
                        let mut child = child;
                        let _ = child.wait();
                    });
                    let mut payload = serde_json::Map::new();
                    payload.insert("job_id".to_string(), json!(job_id));
                    payload.insert("status".to_string(), json!("running"));
                    payload.insert("mode".to_string(), json!(mode));
                    if tool_name == "sync_repo" {
                        payload.insert("changed_files".to_string(), Value::Null);
                    } else {
                        payload.insert("file_count".to_string(), Value::Null);
                    }
                    payload.insert("metadata".to_string(), json!(metadata));
                    tool_text_response(id, Value::Object(payload))
                }
                Err(e) => tool_error_response(
                    id,
                    "internal_error",
                    "Failed to spawn indexer process.",
                    Some(json!({
                        "details": e.to_string(),
                        "remediation": "Run `codecompass index` manually to inspect logs.",
                    })),
                    metadata,
                ),
            }
        }
        _ => JsonRpcResponse::error(id, -32601, format!("Unknown tool: {}", tool_name)),
    }
}

fn map_state_error(err: &StateError) -> (&'static str, String, Option<Value>) {
    match err {
        StateError::SchemaMigrationRequired { current, required } => (
            "index_incompatible",
            "Index schema is incompatible. Run `codecompass index --force`.".to_string(),
            Some(json!({
                "current_schema_version": current,
                "required_schema_version": required,
                "remediation": "codecompass index --force",
            })),
        ),
        StateError::CorruptManifest(details) => (
            "index_incompatible",
            "Index metadata is corrupted. Run `codecompass index --force`.".to_string(),
            Some(json!({
                "details": details,
                "remediation": "codecompass index --force",
            })),
        ),
        other => (
            "internal_error",
            format!("Tool execution failed: {}", other),
            None,
        ),
    }
}

#[allow(clippy::too_many_arguments)]
fn tool_compatibility_error(
    id: Option<Value>,
    schema_status: SchemaStatus,
    compatibility_reason: Option<&str>,
    conn: Option<&rusqlite::Connection>,
    workspace: &Path,
    project_id: &str,
    r#ref: &str,
) -> JsonRpcResponse {
    let metadata = build_metadata(r#ref, schema_status, conn, workspace, project_id);
    if schema_status == SchemaStatus::NotIndexed && !is_project_registered(conn, workspace) {
        return tool_error_response(
            id,
            "project_not_found",
            "Project is not initialized for this workspace. Run `codecompass init` first.",
            Some(json!({
                "workspace": workspace.to_string_lossy(),
                "remediation": "codecompass init --path <workspace>",
            })),
            metadata,
        );
    }

    let remediation = match schema_status {
        SchemaStatus::NotIndexed => "codecompass index",
        SchemaStatus::ReindexRequired | SchemaStatus::CorruptManifest => {
            "codecompass index --force"
        }
        SchemaStatus::Compatible => "codecompass index",
    };
    let message = match schema_status {
        SchemaStatus::NotIndexed => "No index available. Run `codecompass index`.",
        SchemaStatus::ReindexRequired | SchemaStatus::CorruptManifest => {
            "Index is incompatible. Run `codecompass index --force`."
        }
        SchemaStatus::Compatible => "Index is unavailable.",
    };
    tool_error_response(
        id,
        "index_incompatible",
        message,
        Some(json!({
            "schema_status": schema_status,
            "reason": compatibility_reason,
            "remediation": remediation,
        })),
        metadata,
    )
}

fn tool_error_response(
    id: Option<Value>,
    code: &str,
    message: impl Into<String>,
    data: Option<Value>,
    metadata: ProtocolMetadata,
) -> JsonRpcResponse {
    let mut error_obj = serde_json::Map::new();
    error_obj.insert("code".to_string(), Value::String(code.to_string()));
    error_obj.insert("message".to_string(), Value::String(message.into()));
    if let Some(d) = data {
        error_obj.insert("data".to_string(), d);
    }

    let mut payload = serde_json::Map::new();
    payload.insert("error".to_string(), Value::Object(error_obj));
    payload.insert("metadata".to_string(), json!(metadata));

    tool_text_response(id, Value::Object(payload))
}

/// Helper: wrap a JSON value as MCP tool text content response.
fn tool_text_response(id: Option<Value>, payload: Value) -> JsonRpcResponse {
    JsonRpcResponse::success(
        id,
        json!({
            "content": [{"type": "text", "text": serde_json::to_string(&payload).unwrap_or_default()}]
        }),
    )
}

fn rand_u64() -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    std::time::SystemTime::now().hash(&mut hasher);
    std::thread::current().id().hash(&mut hasher);
    std::process::id().hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use codecompass_core::config::Config;
    use codecompass_core::types::Project;
    use serde_json::json;
    use std::path::Path;

    fn make_request(method: &str, params: serde_json::Value) -> JsonRpcRequest {
        JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(1)),
            method: method.into(),
            params,
        }
    }

    #[test]
    fn resolve_tool_ref_falls_back_to_project_default_when_head_unavailable() {
        let tmp = tempfile::tempdir().unwrap();
        let workspace = tmp.path();

        let db_path = tmp.path().join("state.db");
        let conn = codecompass_state::db::open_connection(&db_path).unwrap();
        codecompass_state::schema::create_tables(&conn).unwrap();

        let project_id = "proj_test";
        let project = Project {
            project_id: project_id.to_string(),
            repo_root: workspace.to_string_lossy().to_string(),
            display_name: Some("test".to_string()),
            default_ref: "main".to_string(),
            vcs_mode: true,
            schema_version: 1,
            parser_version: 1,
            created_at: "2026-01-01T00:00:00Z".to_string(),
            updated_at: "2026-01-01T00:00:00Z".to_string(),
        };
        codecompass_state::project::create_project(&conn, &project).unwrap();

        // Temp dir is non-git and has no HEAD branch; should fall back to project default_ref.
        let resolved = resolve_tool_ref(None, workspace, Some(&conn), project_id);
        assert_eq!(resolved, "main");

        // Explicit argument still has top priority.
        let explicit = resolve_tool_ref(Some("feat/auth"), workspace, Some(&conn), project_id);
        assert_eq!(explicit, "feat/auth");
    }

    // ------------------------------------------------------------------
    // T065: tools/list returns all five tools
    // ------------------------------------------------------------------

    #[test]
    fn t065_tools_list_returns_all_five_tools() {
        let config = Config::default();
        let workspace = Path::new("/tmp/fake-workspace");
        let project_id = "fake_project_id";

        let request = make_request("tools/list", json!({}));
        let response = handle_request(
            &request,
            &config,
            None,
            SchemaStatus::NotIndexed,
            None,
            None,
            workspace,
            project_id,
        );

        assert!(response.error.is_none(), "expected success, got error");
        let result = response.result.expect("result should be present");

        let tools = result
            .get("tools")
            .expect("result should contain 'tools'")
            .as_array()
            .expect("'tools' should be an array");

        assert_eq!(tools.len(), 5, "expected 5 tools, got {}", tools.len());

        let tool_names: Vec<&str> = tools
            .iter()
            .map(|t| t.get("name").unwrap().as_str().unwrap())
            .collect();

        let expected_names = [
            "index_repo",
            "sync_repo",
            "search_code",
            "locate_symbol",
            "index_status",
        ];
        for name in &expected_names {
            assert!(
                tool_names.contains(name),
                "missing tool: {name}; found: {tool_names:?}"
            );
        }

        for tool in tools {
            assert!(tool.get("name").is_some(), "tool missing 'name': {tool:?}");
            assert!(
                tool.get("description").is_some(),
                "tool missing 'description': {tool:?}"
            );
            assert!(
                tool.get("inputSchema").is_some(),
                "tool missing 'inputSchema': {tool:?}"
            );

            let desc = tool.get("description").unwrap().as_str().unwrap();
            assert!(!desc.is_empty(), "tool description is empty: {tool:?}");

            assert!(
                tool.get("inputSchema").unwrap().is_object(),
                "inputSchema should be an object: {tool:?}"
            );
        }
    }

    // ------------------------------------------------------------------
    // T066: locate_symbol via JSON-RPC with an indexed fixture
    // ------------------------------------------------------------------

    fn build_fixture_index(tmp_dir: &std::path::Path) -> IndexSet {
        use codecompass_indexer::{
            languages, parser, scanner, snippet_extract, symbol_extract, writer,
        };
        use codecompass_state::{db, schema, tantivy_index::IndexSet};

        let fixture_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../testdata/fixtures/rust-sample");
        assert!(
            fixture_dir.exists(),
            "fixture directory missing: {}",
            fixture_dir.display()
        );

        let data_dir = tmp_dir.join("data");
        std::fs::create_dir_all(&data_dir).unwrap();

        let index_set = IndexSet::open(&data_dir).unwrap();

        let db_path = data_dir.join("state.db");
        let conn = db::open_connection(&db_path).unwrap();
        schema::create_tables(&conn).unwrap();

        let scanned = scanner::scan_directory(&fixture_dir, 1_048_576);
        assert!(
            !scanned.is_empty(),
            "scanner found no files in fixture directory"
        );

        let repo = "test-repo";
        let r#ref = "live";

        for file in &scanned {
            let source = std::fs::read_to_string(&file.path).unwrap();
            let tree = match parser::parse_file(&source, &file.language) {
                Ok(t) => t,
                Err(_) => continue,
            };

            let extracted = languages::extract_symbols(&tree, &source, &file.language);
            let symbols = symbol_extract::build_symbol_records(
                &extracted,
                repo,
                r#ref,
                &file.relative_path,
                None,
            );
            let snippets = snippet_extract::build_snippet_records(
                &extracted,
                repo,
                r#ref,
                &file.relative_path,
                None,
            );

            let content_hash = blake3::hash(source.as_bytes()).to_hex().to_string();
            let filename = file.path.file_name().unwrap().to_string_lossy().to_string();
            let file_record = codecompass_core::types::FileRecord {
                repo: repo.to_string(),
                r#ref: r#ref.to_string(),
                commit: None,
                path: file.relative_path.clone(),
                filename,
                language: file.language.clone(),
                content_hash,
                size_bytes: source.len() as u64,
                updated_at: "2026-01-01T00:00:00Z".to_string(),
                content_head: source
                    .lines()
                    .take(10)
                    .collect::<Vec<_>>()
                    .join("\n")
                    .into(),
            };

            writer::write_file_records(&index_set, &conn, &symbols, &snippets, &file_record)
                .unwrap();
        }

        index_set
    }

    #[test]
    fn t066_locate_symbol_via_jsonrpc() {
        let tmp = tempfile::tempdir().unwrap();
        let index_set = build_fixture_index(tmp.path());

        let config = Config::default();
        let workspace = Path::new("/tmp/fake-workspace");
        let project_id = "test_project";

        let request = make_request(
            "tools/call",
            json!({
                "name": "locate_symbol",
                "arguments": {
                    "name": "validate_token"
                }
            }),
        );

        let response = handle_request(
            &request,
            &config,
            Some(&index_set),
            SchemaStatus::Compatible,
            None,
            None,
            workspace,
            project_id,
        );

        assert!(
            response.error.is_none(),
            "expected success, got error: {:?}",
            response.error
        );
        let result = response.result.expect("result should be present");

        let content = result
            .get("content")
            .expect("result should have 'content'")
            .as_array()
            .expect("'content' should be an array");

        assert!(!content.is_empty(), "content array should not be empty");

        let first = &content[0];
        assert_eq!(
            first.get("type").unwrap().as_str().unwrap(),
            "text",
            "content type should be 'text'"
        );

        let text = first.get("text").unwrap().as_str().unwrap();
        let payload: serde_json::Value =
            serde_json::from_str(text).expect("text payload should be valid JSON");

        let results = payload
            .get("results")
            .expect("payload should have 'results'")
            .as_array()
            .expect("'results' should be an array");

        assert!(
            !results.is_empty(),
            "results should contain at least one match for 'validate_token'"
        );

        let vt = results
            .iter()
            .find(|r| r.get("name").unwrap().as_str().unwrap() == "validate_token")
            .expect("results should contain a 'validate_token' entry");

        assert_eq!(vt.get("kind").unwrap().as_str().unwrap(), "function");
        assert_eq!(vt.get("language").unwrap().as_str().unwrap(), "rust");
        assert!(
            vt.get("path")
                .unwrap()
                .as_str()
                .unwrap()
                .contains("auth.rs"),
            "path should reference auth.rs"
        );
        assert!(vt.get("line_start").unwrap().as_u64().unwrap() > 0);
        assert!(vt.get("line_end").unwrap().as_u64().unwrap() > 0);
        assert!(
            !vt.get("symbol_id").unwrap().as_str().unwrap().is_empty(),
            "symbol_id should not be empty"
        );
        assert!(
            !vt.get("symbol_stable_id")
                .unwrap()
                .as_str()
                .unwrap()
                .is_empty(),
            "symbol_stable_id should not be empty"
        );

        // Verify Protocol v1 metadata
        let metadata = payload
            .get("metadata")
            .expect("payload should have 'metadata'");
        assert_eq!(
            metadata
                .get("codecompass_protocol_version")
                .unwrap()
                .as_str()
                .unwrap(),
            "1.0"
        );
        assert_eq!(
            metadata.get("ref").unwrap().as_str().unwrap(),
            "live",
            "ref should default to 'live'"
        );
    }
}
