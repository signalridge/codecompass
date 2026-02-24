use super::*;

pub(super) fn handle_health_check(params: &ToolCallParams<'_>) -> JsonRpcResponse {
    let ToolCallParams {
        id,
        arguments,
        config,
        index_set,
        schema_status,
        conn,
        workspace,
        project_id,
        prewarm_status,
        server_start,
        ..
    } = params;

    let requested_workspace = arguments.get("workspace").and_then(|v| v.as_str());
    let effective_ref = resolve_tool_ref(None, workspace, *conn, project_id);
    let metadata = build_metadata(
        &effective_ref,
        *schema_status,
        config,
        *conn,
        workspace,
        project_id,
    );

    let projects = if let Some(c) = conn {
        if let Some(rw) = requested_workspace {
            match codecompass_state::project::get_by_root(c, rw)
                .ok()
                .flatten()
            {
                Some(p) => vec![p],
                None => {
                    return tool_error_response(
                        id.clone(),
                        "workspace_not_registered",
                        format!("The specified workspace '{}' is not registered.", rw),
                        Some(json!({
                            "requested_workspace": rw,
                        })),
                        metadata,
                    );
                }
            }
        } else {
            codecompass_state::project::list_projects(c).unwrap_or_default()
        }
    } else {
        Vec::new()
    };

    let pw_status = prewarm_status.load(Ordering::Acquire);
    let pw_label = prewarm_status_label(pw_status);

    let tantivy_checks = if let Some(idx) = index_set {
        codecompass_state::tantivy_index::check_tantivy_health(idx)
    } else {
        Vec::new()
    };
    let tantivy_ok = !tantivy_checks.is_empty() && tantivy_checks.iter().all(|c| c.ok);

    let (sqlite_ok, sqlite_error) = conn
        .and_then(|c| codecompass_state::db::check_sqlite_health(c).ok())
        .unwrap_or((false, Some("No database connection".into())));

    let supported = codecompass_indexer::parser::supported_languages();
    let mut grammars_available = Vec::new();
    let mut grammars_missing = Vec::new();
    for lang in &supported {
        match codecompass_indexer::parser::get_language(lang) {
            Ok(_) => grammars_available.push(*lang),
            Err(_) => grammars_missing.push(*lang),
        }
    }

    let mut overall_has_active_job = false;
    let mut active_job_payload: Option<Value> = None;
    let mut project_payloads = Vec::new();
    let mut any_error_project = false;
    let mut any_warming_project = false;

    if let Some(c) = conn {
        let iter_projects: Vec<_> = if projects.is_empty() {
            codecompass_state::project::get_by_id(c, project_id)
                .ok()
                .flatten()
                .into_iter()
                .collect()
        } else {
            projects
        };

        for p in iter_projects {
            let project_workspace = Path::new(&p.repo_root);
            let project_ref = if p.default_ref.trim().is_empty() {
                constants::REF_LIVE.to_string()
            } else {
                p.default_ref.clone()
            };
            let project_schema_status =
                resolve_project_schema_status(config, project_id, &p.project_id, *schema_status);
            let freshness_result = check_freshness_with_scan_params(
                Some(c),
                project_workspace,
                &p.project_id,
                &project_ref,
                config.index.max_file_size,
                Some(&config.index.languages),
            );
            let proj_freshness_status = freshness::freshness_status(&freshness_result);

            let active_job = codecompass_state::jobs::get_active_job(c, &p.project_id)
                .ok()
                .flatten();
            if let Some(j) = &active_job {
                overall_has_active_job = true;
                if active_job_payload.is_none() {
                    active_job_payload = Some(json!({
                        "job_id": j.job_id,
                        "project_id": j.project_id,
                        "mode": j.mode,
                        "status": j.status,
                        "ref": j.r#ref,
                        "changed_files": j.changed_files,
                        "started_at": j.created_at,
                    }));
                }
            }

            let (index_status, warming) = if active_job.is_some() {
                ("indexing", false)
            } else if !matches!(project_schema_status, SchemaStatus::Compatible) {
                ("error", false)
            } else if p.project_id == *project_id && pw_status == PREWARM_IN_PROGRESS {
                ("warming", true)
            } else if p.project_id == *project_id && pw_status == PREWARM_FAILED {
                ("error", false)
            } else {
                ("ready", false)
            };
            any_warming_project |= warming;
            any_error_project |= index_status == "error";

            let file_count =
                codecompass_state::manifest::file_count(c, &p.project_id, &project_ref)
                    .unwrap_or(0);
            let symbol_count =
                codecompass_state::symbols::symbol_count(c, &p.project_id, &project_ref)
                    .unwrap_or(0);
            let last_indexed_at = codecompass_state::jobs::get_recent_jobs(c, &p.project_id, 5)
                .ok()
                .and_then(|jobs| {
                    jobs.into_iter()
                        .find(|j| j.status == "published" && j.r#ref == project_ref)
                        .map(|j| j.updated_at)
                });

            project_payloads.push(json!({
                "project_id": p.project_id,
                "repo_root": p.repo_root,
                "index_status": index_status,
                "freshness_status": proj_freshness_status,
                "last_indexed_at": last_indexed_at,
                "ref": project_ref,
                "file_count": file_count,
                "symbol_count": symbol_count,
            }));
        }

        if project_payloads.is_empty() {
            let fallback_status = if matches!(
                schema_status,
                SchemaStatus::ReindexRequired
                    | SchemaStatus::CorruptManifest
                    | SchemaStatus::NotIndexed
            ) {
                any_error_project = true;
                "error"
            } else if pw_status == PREWARM_IN_PROGRESS {
                any_warming_project = true;
                "warming"
            } else if pw_status == PREWARM_FAILED {
                any_error_project = true;
                "error"
            } else {
                "ready"
            };
            project_payloads.push(json!({
                "project_id": project_id,
                "repo_root": workspace.to_string_lossy(),
                "index_status": fallback_status,
                "freshness_status": metadata.freshness_status,
                "last_indexed_at": Value::Null,
                "ref": effective_ref,
                "file_count": codecompass_state::manifest::file_count(c, project_id, &effective_ref).unwrap_or(0),
                "symbol_count": codecompass_state::symbols::symbol_count(c, project_id, &effective_ref).unwrap_or(0),
            }));
        }
    } else {
        project_payloads.push(json!({
            "project_id": project_id,
            "repo_root": workspace.to_string_lossy(),
            "index_status": "error",
            "freshness_status": metadata.freshness_status,
            "last_indexed_at": Value::Null,
            "ref": effective_ref,
            "file_count": 0,
            "symbol_count": 0,
        }));
        any_error_project = true;
    }

    let uptime_seconds = server_start.elapsed().as_secs();

    let stored_schema_version = conn.and_then(|c| {
        codecompass_state::project::get_by_id(c, project_id)
            .ok()
            .flatten()
            .map(|p| p.schema_version)
    });
    let current_schema_version = match schema_status {
        SchemaStatus::Compatible => constants::SCHEMA_VERSION,
        _ => stored_schema_version.unwrap_or(0),
    };
    let (index_compat_status, compat_message) = match schema_status {
        SchemaStatus::Compatible => ("compatible", None),
        SchemaStatus::NotIndexed => ("not_indexed", None),
        SchemaStatus::ReindexRequired => (
            "reindex_required",
            Some("Run `codecompass index --force` to reindex."),
        ),
        SchemaStatus::CorruptManifest => (
            "corrupt_manifest",
            Some("Run `codecompass index --force` to rebuild."),
        ),
    };
    let overall_status = if any_error_project {
        "error"
    } else if overall_has_active_job {
        "indexing"
    } else if any_warming_project {
        "warming"
    } else {
        "ready"
    };

    let result = json!({
        "status": overall_status,
        "version": env!("CARGO_PKG_VERSION"),
        "uptime_seconds": uptime_seconds,
        "tantivy_ok": tantivy_ok,
        "sqlite_ok": sqlite_ok,
        "sqlite_error": sqlite_error,
        "prewarm_status": pw_label,
        "grammars": {
            "available": grammars_available,
            "missing": grammars_missing,
        },
        "active_job": active_job_payload,
        "startup_checks": {
            "index": {
                "status": index_compat_status,
                "current_schema_version": current_schema_version,
                "required_schema_version": constants::SCHEMA_VERSION,
                "message": compat_message,
            }
        },
        "projects": project_payloads,
        "metadata": metadata,
    });
    tool_text_response(id.clone(), result)
}
