use super::*;

pub(super) fn handle_index_status(params: ReadToolParams<'_>) -> JsonRpcResponse {
    let ReadToolParams {
        id,
        arguments,
        config,
        schema_status,
        compatibility_reason,
        conn,
        workspace,
        project_id,
    } = params;

    let requested_ref = arguments.get("ref").and_then(|v| v.as_str());
    let effective_ref = resolve_tool_ref(requested_ref, workspace, conn, project_id);
    let stored_schema_version = conn.and_then(|c| {
        codecompass_state::project::get_by_id(c, project_id)
            .ok()
            .flatten()
            .map(|p| p.schema_version)
    });
    let (status, schema_status_str, current_schema_version) = match schema_status {
        SchemaStatus::Compatible => ("ready", "compatible", constants::SCHEMA_VERSION),
        SchemaStatus::NotIndexed => (
            "not_indexed",
            "not_indexed",
            stored_schema_version.unwrap_or(0),
        ),
        SchemaStatus::ReindexRequired => (
            "not_indexed",
            "reindex_required",
            stored_schema_version.unwrap_or(0),
        ),
        SchemaStatus::CorruptManifest => (
            "not_indexed",
            "corrupt_manifest",
            stored_schema_version.unwrap_or(0),
        ),
    };

    let (file_count, symbol_count) = conn
        .map(|c| {
            let fc =
                codecompass_state::manifest::file_count(c, project_id, &effective_ref).unwrap_or(0);
            let sc = codecompass_state::symbols::symbol_count(c, project_id, &effective_ref)
                .unwrap_or(0);
            (fc, sc)
        })
        .unwrap_or((0, 0));

    let recent_jobs = conn
        .and_then(|c| codecompass_state::jobs::get_recent_jobs(c, project_id, 5).ok())
        .unwrap_or_default();

    let active_job = conn.and_then(|c| {
        codecompass_state::jobs::get_active_job(c, project_id)
            .ok()
            .flatten()
    });

    let last_indexed_at: Option<String> = recent_jobs
        .iter()
        .find(|j| j.status == "published" && j.r#ref == effective_ref)
        .map(|j| j.updated_at.clone());

    let result = json!({
        "project_id": project_id,
        "repo_root": workspace.to_string_lossy(),
        "index_status": status,
        "schema_status": schema_status_str,
        "current_schema_version": current_schema_version,
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
        "metadata": build_metadata(
            &effective_ref,
            schema_status,
            config,
            conn,
            workspace,
            project_id
        ),
    });
    tool_text_response(id, result)
}
