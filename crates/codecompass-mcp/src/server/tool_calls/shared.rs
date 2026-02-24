use super::*;

pub(super) fn map_state_error(err: &StateError) -> (&'static str, String, Option<Value>) {
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

pub(super) struct ToolCompatibilityParams<'a> {
    pub(super) id: Option<Value>,
    pub(super) schema_status: SchemaStatus,
    pub(super) compatibility_reason: Option<&'a str>,
    pub(super) config: &'a Config,
    pub(super) conn: Option<&'a rusqlite::Connection>,
    pub(super) workspace: &'a Path,
    pub(super) project_id: &'a str,
    pub(super) ref_name: &'a str,
}

pub(super) fn tool_compatibility_error(params: ToolCompatibilityParams<'_>) -> JsonRpcResponse {
    let ToolCompatibilityParams {
        id,
        schema_status,
        compatibility_reason,
        config,
        conn,
        workspace,
        project_id,
        ref_name,
    } = params;

    let metadata = build_metadata(ref_name, schema_status, config, conn, workspace, project_id);
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

pub(super) fn tool_error_response(
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
pub(super) fn tool_text_response(id: Option<Value>, payload: Value) -> JsonRpcResponse {
    JsonRpcResponse::success(
        id,
        json!({
            "content": [{"type": "text", "text": serde_json::to_string(&payload).unwrap_or_default()}]
        }),
    )
}

/// Result of freshness check + policy enforcement for query tools.
pub(super) struct FreshnessEnforced {
    pub(super) metadata: ProtocolMetadata,
    /// If the policy requires blocking, this holds the pre-built error response.
    pub(super) block_response: Option<JsonRpcResponse>,
}

/// Check freshness and enforce the configured policy. Returns metadata and an optional
/// block response. When `block_response` is `Some`, the caller must return it immediately.
#[allow(clippy::too_many_arguments)]
pub(super) fn check_and_enforce_freshness(
    id: Option<Value>,
    arguments: &Value,
    config: &Config,
    conn: Option<&rusqlite::Connection>,
    workspace: &Path,
    project_id: &str,
    effective_ref: &str,
    schema_status: SchemaStatus,
) -> FreshnessEnforced {
    let policy = resolve_freshness_policy(arguments, config);
    let freshness_result = check_freshness_with_scan_params(
        conn,
        workspace,
        project_id,
        effective_ref,
        config.index.max_file_size,
        Some(&config.index.languages),
    );
    let policy_action = apply_freshness_policy(policy, &freshness_result);
    let metadata = build_metadata_with_freshness(effective_ref, schema_status, &freshness_result);

    if let PolicyAction::BlockWithError {
        last_indexed_commit,
        current_head,
    } = &policy_action
    {
        let block_metadata = metadata.clone();
        return FreshnessEnforced {
            block_response: Some(tool_error_response(
                id,
                "index_stale",
                "Index is stale and freshness_policy is strict. Sync before querying.",
                Some(json!({
                    "last_indexed_commit": last_indexed_commit,
                    "current_head": current_head,
                    "suggestion": "Call sync_repo to update the index before querying.",
                })),
                metadata,
            )),
            metadata: block_metadata,
        };
    }
    if policy_action == PolicyAction::ProceedWithStaleIndicatorAndSync {
        trigger_async_sync(workspace, effective_ref);
    }

    FreshnessEnforced {
        metadata,
        block_response: None,
    }
}

/// Parse `detail_level` from MCP tool arguments, defaulting to `Signature`.
pub(super) fn parse_detail_level(arguments: &Value) -> DetailLevel {
    arguments
        .get("detail_level")
        .and_then(|v| v.as_str())
        .map(|s| match s {
            "location" => DetailLevel::Location,
            "context" => DetailLevel::Context,
            _ => DetailLevel::Signature,
        })
        .unwrap_or(DetailLevel::Signature)
}

pub(super) fn parse_compact(arguments: &Value) -> bool {
    arguments
        .get("compact")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
}

pub(super) fn resolve_ranking_explain_level(
    arguments: &Value,
    config: &Config,
) -> Result<codecompass_core::types::RankingExplainLevel, String> {
    if let Some(raw) = arguments
        .get("ranking_explain_level")
        .and_then(|v| v.as_str())
    {
        return parse_ranking_explain_level(raw).ok_or_else(|| {
            "Parameter `ranking_explain_level` must be `off`, `basic`, or `full`.".to_string()
        });
    }

    let level = parse_ranking_explain_level(&config.search.ranking_explain_level)
        .unwrap_or(codecompass_core::types::RankingExplainLevel::Off);
    // Config::load_with_file already promotes legacy `debug.ranking_reasons` into
    // `search.ranking_explain_level`. Keep this runtime fallback for compatibility
    // with direct/manual Config construction paths (e.g., focused tests).
    if level == codecompass_core::types::RankingExplainLevel::Off && config.debug.ranking_reasons {
        return Ok(codecompass_core::types::RankingExplainLevel::Full);
    }
    Ok(level)
}

pub(super) fn parse_ranking_explain_level(
    raw: &str,
) -> Option<codecompass_core::types::RankingExplainLevel> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "off" => Some(codecompass_core::types::RankingExplainLevel::Off),
        "basic" => Some(codecompass_core::types::RankingExplainLevel::Basic),
        "full" => Some(codecompass_core::types::RankingExplainLevel::Full),
        _ => None,
    }
}

pub(super) fn ranking_reasons_payload(
    reasons: Vec<codecompass_core::types::RankingReasons>,
    level: codecompass_core::types::RankingExplainLevel,
) -> Option<Value> {
    match level {
        codecompass_core::types::RankingExplainLevel::Off => None,
        codecompass_core::types::RankingExplainLevel::Full => serde_json::to_value(reasons).ok(),
        codecompass_core::types::RankingExplainLevel::Basic => {
            serde_json::to_value(ranking::to_basic_ranking_reasons(&reasons)).ok()
        }
    }
}

pub(super) fn dedup_search_results(
    results: Vec<search::SearchResult>,
) -> (Vec<search::SearchResult>, Vec<usize>, usize) {
    use std::collections::HashSet;

    let mut seen = HashSet::new();
    let mut deduped = Vec::with_capacity(results.len());
    let mut kept_indices = Vec::with_capacity(results.len());
    let mut suppressed = 0usize;
    for (index, result) in results.into_iter().enumerate() {
        let key =
            if let Some(stable_id) = result.symbol_stable_id.as_ref().filter(|s| !s.is_empty()) {
                format!("stable:{}", stable_id)
            } else {
                format!(
                    "{}:{}:{}:{}:{}",
                    result.result_type,
                    result.path,
                    result.line_start,
                    result.line_end,
                    result.name.as_deref().unwrap_or("")
                )
            };
        if seen.insert(key) {
            deduped.push(result);
            kept_indices.push(index);
        } else {
            suppressed += 1;
        }
    }
    (deduped, kept_indices, suppressed)
}

pub(super) fn dedup_locate_results(
    results: Vec<locate::LocateResult>,
) -> (Vec<locate::LocateResult>, usize) {
    use std::collections::HashSet;

    let mut seen = HashSet::new();
    let mut deduped = Vec::with_capacity(results.len());
    let mut suppressed = 0usize;
    for result in results {
        let key = if result.symbol_stable_id.is_empty() {
            format!(
                "{}:{}:{}:{}",
                result.path, result.line_start, result.line_end, result.name
            )
        } else {
            format!("stable:{}", result.symbol_stable_id)
        };
        if seen.insert(key) {
            deduped.push(result);
        } else {
            suppressed += 1;
        }
    }
    (deduped, suppressed)
}

pub(super) fn align_ranking_reasons_to_dedup(
    reasons: &[codecompass_core::types::RankingReasons],
    kept_indices: &[usize],
) -> Vec<codecompass_core::types::RankingReasons> {
    let mut aligned = Vec::with_capacity(kept_indices.len());
    for (new_index, old_index) in kept_indices.iter().copied().enumerate() {
        let Some(reason) = reasons.get(old_index) else {
            continue;
        };
        let mut updated = reason.clone();
        updated.result_index = new_index;
        aligned.push(updated);
    }
    aligned
}

pub(super) fn enforce_payload_safety_limit(
    results: Vec<Value>,
    max_bytes: usize,
) -> (Vec<Value>, bool) {
    let max_bytes = if max_bytes == 0 {
        DEFAULT_MAX_RESPONSE_BYTES
    } else {
        max_bytes
    };

    let mut output = Vec::new();
    let mut used = 2usize; // '[' + ']'
    let mut truncated = false;
    for item in results {
        let item_size = serde_json::to_vec(&item).map(|v| v.len()).unwrap_or(0);
        let separator = usize::from(!output.is_empty());
        if used + separator + item_size > max_bytes {
            truncated = true;
            break;
        }
        used += separator + item_size;
        output.push(item);
    }

    if output.is_empty() && truncated {
        // Under extremely small byte limits, even the first item may not fit.
        // Returning [] with `truncated=true` keeps behavior deterministic while
        // signaling callers to follow `suggested_next_actions`.
        return (Vec::new(), true);
    }
    (output, truncated)
}

pub(super) struct FilteredResultPayload {
    pub(super) filtered: Vec<Value>,
    pub(super) safety_limit_applied: bool,
}

pub(super) fn build_filtered_result_payload(
    mut result_values: Vec<Value>,
    detail_level: DetailLevel,
    compact: bool,
    conn: Option<&rusqlite::Connection>,
    project_id: &str,
    effective_ref: &str,
    max_response_bytes: usize,
) -> FilteredResultPayload {
    if detail_level == DetailLevel::Context && !compact {
        detail::enrich_body_previews(&mut result_values);
        if let Some(c) = conn {
            detail::enrich_results_with_relations(&mut result_values, c, project_id, effective_ref);
        }
    }

    let filtered = detail::serialize_results_at_level(&result_values, detail_level, compact);
    let (filtered, safety_limit_applied) =
        enforce_payload_safety_limit(filtered, max_response_bytes);
    FilteredResultPayload {
        filtered,
        safety_limit_applied,
    }
}

pub(super) fn deterministic_suggested_actions(
    existing: &[search::SuggestedAction],
    query: &str,
    effective_ref: &str,
    limit: usize,
) -> Vec<search::SuggestedAction> {
    if !existing.is_empty() {
        return existing.to_vec();
    }
    vec![search::SuggestedAction {
        tool: "search_code".to_string(),
        name: None,
        query: Some(query.to_string()),
        r#ref: Some(effective_ref.to_string()),
        limit: Some(limit.max(1) / 2 + 1),
    }]
}

pub(super) fn deterministic_locate_suggested_actions(
    name: &str,
    effective_ref: &str,
    limit: usize,
) -> Vec<search::SuggestedAction> {
    vec![
        search::SuggestedAction {
            tool: "locate_symbol".to_string(),
            name: Some(name.to_string()),
            query: None,
            r#ref: Some(effective_ref.to_string()),
            limit: Some((limit / 2).max(1)),
        },
        search::SuggestedAction {
            tool: "search_code".to_string(),
            name: None,
            query: Some(name.to_string()),
            r#ref: Some(effective_ref.to_string()),
            limit: Some(5),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub(super) fn dedup_search_results_by_stable_id() {
        let base = search::SearchResult {
            result_id: "r1".to_string(),
            symbol_id: Some("sym1".to_string()),
            symbol_stable_id: Some("stable1".to_string()),
            result_type: "symbol".to_string(),
            path: "src/lib.rs".to_string(),
            line_start: 10,
            line_end: 20,
            kind: Some("fn".to_string()),
            name: Some("foo".to_string()),
            qualified_name: Some("foo".to_string()),
            language: "rust".to_string(),
            signature: None,
            visibility: None,
            score: 1.0,
            snippet: None,
        };
        let mut second = base.clone();
        second.result_id = "r2".to_string();
        let third = search::SearchResult {
            result_id: "r3".to_string(),
            symbol_id: None,
            symbol_stable_id: None,
            result_type: "file".to_string(),
            path: "src/other.rs".to_string(),
            line_start: 1,
            line_end: 1,
            kind: None,
            name: None,
            qualified_name: None,
            language: "rust".to_string(),
            signature: None,
            visibility: None,
            score: 0.5,
            snippet: None,
        };

        let (deduped, kept_indices, suppressed) = dedup_search_results(vec![base, second, third]);
        assert_eq!(suppressed, 1);
        assert_eq!(deduped.len(), 2);
        assert_eq!(kept_indices, vec![0, 2]);
    }

    #[test]
    pub(super) fn dedup_locate_results_by_stable_id() {
        let a = locate::LocateResult {
            symbol_id: "s1".to_string(),
            symbol_stable_id: "stable".to_string(),
            path: "src/lib.rs".to_string(),
            line_start: 10,
            line_end: 20,
            kind: "fn".to_string(),
            name: "foo".to_string(),
            qualified_name: "foo".to_string(),
            signature: None,
            language: "rust".to_string(),
            visibility: None,
            score: 1.0,
        };
        let mut b = a.clone();
        b.symbol_id = "s2".to_string();
        let (deduped, suppressed) = dedup_locate_results(vec![a, b]);
        assert_eq!(suppressed, 1);
        assert_eq!(deduped.len(), 1);
    }

    #[test]
    fn safety_limit_truncates_results() {
        let results = vec![
            json!({"name": "a", "payload": "x".repeat(80)}),
            json!({"name": "b", "payload": "y".repeat(80)}),
            json!({"name": "c", "payload": "z".repeat(80)}),
        ];
        let (trimmed, truncated) = enforce_payload_safety_limit(results, 120);
        assert!(truncated);
        assert!(trimmed.len() < 3);
    }

    #[test]
    fn filtered_payload_compact_context_omits_heavy_fields() {
        let results = vec![json!({
            "symbol_id": "sym_1",
            "symbol_stable_id": "stable_1",
            "result_id": "res_1",
            "result_type": "symbol",
            "path": "src/lib.rs",
            "line_start": 10,
            "line_end": 20,
            "kind": "function",
            "name": "validate_token",
            "snippet": "fn validate_token() { /* body */ }",
            "body_preview": "preview",
        })];

        let payload = build_filtered_result_payload(
            results,
            DetailLevel::Context,
            true,
            None,
            "proj_1",
            "main",
            4096,
        );

        assert!(!payload.safety_limit_applied);
        let first = payload
            .filtered
            .first()
            .and_then(|v| v.as_object())
            .unwrap();
        assert!(first.get("snippet").is_none());
        assert!(first.get("body_preview").is_none());
    }

    #[test]
    fn ranking_payload_basic_uses_compact_fields() {
        let reasons = vec![codecompass_core::types::RankingReasons {
            result_index: 0,
            exact_match_boost: 5.0,
            qualified_name_boost: 2.0,
            path_affinity: 1.0,
            definition_boost: 1.0,
            kind_match: 0.0,
            bm25_score: 10.0,
            final_score: 19.0,
        }];

        let payload =
            ranking_reasons_payload(reasons, codecompass_core::types::RankingExplainLevel::Basic)
                .unwrap();
        let first = payload.as_array().unwrap().first().unwrap();
        assert!(first.get("exact_match").is_some());
        assert!(first.get("path_boost").is_some());
        assert!(first.get("semantic_similarity").is_some());
        assert!(first.get("qualified_name_boost").is_none());
    }

    #[test]
    fn ranking_reasons_remain_aligned_after_dedup() {
        let results = vec![
            search::SearchResult {
                result_id: "r1".to_string(),
                symbol_id: Some("sym1".to_string()),
                symbol_stable_id: Some("stable1".to_string()),
                result_type: "symbol".to_string(),
                path: "src/lib.rs".to_string(),
                line_start: 10,
                line_end: 20,
                kind: Some("fn".to_string()),
                name: Some("foo".to_string()),
                qualified_name: Some("foo".to_string()),
                language: "rust".to_string(),
                signature: None,
                visibility: None,
                score: 2.0,
                snippet: None,
            },
            search::SearchResult {
                result_id: "r2".to_string(),
                symbol_id: Some("sym2".to_string()),
                symbol_stable_id: Some("stable1".to_string()),
                result_type: "symbol".to_string(),
                path: "src/lib.rs".to_string(),
                line_start: 10,
                line_end: 20,
                kind: Some("fn".to_string()),
                name: Some("foo_dup".to_string()),
                qualified_name: Some("foo_dup".to_string()),
                language: "rust".to_string(),
                signature: None,
                visibility: None,
                score: 1.5,
                snippet: None,
            },
            search::SearchResult {
                result_id: "r3".to_string(),
                symbol_id: Some("sym3".to_string()),
                symbol_stable_id: Some("stable3".to_string()),
                result_type: "symbol".to_string(),
                path: "src/main.rs".to_string(),
                line_start: 30,
                line_end: 40,
                kind: Some("fn".to_string()),
                name: Some("bar".to_string()),
                qualified_name: Some("bar".to_string()),
                language: "rust".to_string(),
                signature: None,
                visibility: None,
                score: 1.0,
                snippet: None,
            },
        ];
        let reasons = vec![
            codecompass_core::types::RankingReasons {
                result_index: 0,
                exact_match_boost: 5.0,
                qualified_name_boost: 1.0,
                path_affinity: 0.0,
                definition_boost: 1.0,
                kind_match: 0.0,
                bm25_score: 10.0,
                final_score: 17.0,
            },
            codecompass_core::types::RankingReasons {
                result_index: 1,
                exact_match_boost: 0.0,
                qualified_name_boost: 1.0,
                path_affinity: 0.0,
                definition_boost: 1.0,
                kind_match: 0.0,
                bm25_score: 9.0,
                final_score: 11.0,
            },
            codecompass_core::types::RankingReasons {
                result_index: 2,
                exact_match_boost: 0.0,
                qualified_name_boost: 0.0,
                path_affinity: 1.0,
                definition_boost: 1.0,
                kind_match: 0.0,
                bm25_score: 8.0,
                final_score: 10.0,
            },
        ];

        let (_deduped, kept_indices, suppressed) = dedup_search_results(results);
        assert_eq!(suppressed, 1);
        let aligned = align_ranking_reasons_to_dedup(&reasons, &kept_indices);
        assert_eq!(aligned.len(), 2);
        assert_eq!(aligned[0].result_index, 0);
        assert_eq!(aligned[0].bm25_score, 10.0);
        assert_eq!(aligned[1].result_index, 1);
        assert_eq!(aligned[1].bm25_score, 8.0);
    }

    #[test]
    fn deterministic_locate_actions_are_stable() {
        let actions = deterministic_locate_suggested_actions("validate_token", "main", 10);
        assert_eq!(actions.len(), 2);
        assert_eq!(actions[0].tool, "locate_symbol");
        assert_eq!(actions[0].name.as_deref(), Some("validate_token"));
        assert_eq!(actions[0].limit, Some(5));
        assert_eq!(actions[1].tool, "search_code");
        assert_eq!(actions[1].query.as_deref(), Some("validate_token"));
        assert_eq!(actions[1].limit, Some(5));
    }
}
