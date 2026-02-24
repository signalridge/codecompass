use super::*;

pub(super) fn handle_index_or_sync(params: IndexToolParams<'_>) -> JsonRpcResponse {
    let IndexToolParams {
        id,
        tool_name,
        arguments,
        config,
        schema_status,
        conn,
        workspace,
        project_id,
    } = params;

    let force = arguments
        .get("force")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let mode = if force { "full" } else { "incremental" };
    let requested_ref = arguments.get("ref").and_then(|v| v.as_str());
    let effective_ref = resolve_tool_ref(requested_ref, workspace, conn, project_id);
    let metadata = build_metadata(
        &effective_ref,
        schema_status,
        config,
        conn,
        workspace,
        project_id,
    );

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

    let exe = std::env::current_exe().unwrap_or_else(|_| "codecompass".into());
    let workspace_str = workspace.to_string_lossy();
    let job_id = codecompass_core::ids::new_job_id();

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
    cmd.arg("--ref").arg(&effective_ref);

    match cmd.spawn() {
        Ok(child) => {
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
