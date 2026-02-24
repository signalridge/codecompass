use super::*;

pub(super) struct ToolCallParams<'a> {
    pub id: Option<Value>,
    pub tool_name: &'a str,
    pub arguments: &'a Value,
    pub config: &'a Config,
    pub index_set: Option<&'a IndexSet>,
    pub schema_status: SchemaStatus,
    pub compatibility_reason: Option<&'a str>,
    pub conn: Option<&'a rusqlite::Connection>,
    pub workspace: &'a Path,
    pub project_id: &'a str,
    pub prewarm_status: &'a AtomicU8,
    pub server_start: &'a Instant,
}

pub(super) struct QueryToolParams<'a> {
    pub id: Option<Value>,
    pub arguments: &'a Value,
    pub config: &'a Config,
    pub index_set: Option<&'a IndexSet>,
    pub schema_status: SchemaStatus,
    pub compatibility_reason: Option<&'a str>,
    pub conn: Option<&'a rusqlite::Connection>,
    pub workspace: &'a Path,
    pub project_id: &'a str,
}

pub(super) struct IndexToolParams<'a> {
    pub id: Option<Value>,
    pub tool_name: &'a str,
    pub arguments: &'a Value,
    pub config: &'a Config,
    pub schema_status: SchemaStatus,
    pub conn: Option<&'a rusqlite::Connection>,
    pub workspace: &'a Path,
    pub project_id: &'a str,
}

pub(super) struct ReadToolParams<'a> {
    pub id: Option<Value>,
    pub arguments: &'a Value,
    pub config: &'a Config,
    pub schema_status: SchemaStatus,
    pub compatibility_reason: Option<&'a str>,
    pub conn: Option<&'a rusqlite::Connection>,
    pub workspace: &'a Path,
    pub project_id: &'a str,
}

const DEFAULT_MAX_RESPONSE_BYTES: usize = 64 * 1024;

mod context_tools;
mod health_tools;
mod index_tools;
mod query_tools;
mod shared;
mod status_tools;
mod structure_tools;
use shared::*;

pub(super) fn handle_tool_call(params: ToolCallParams<'_>) -> JsonRpcResponse {
    // Handle health_check before destructuring since it needs the full params struct
    if params.tool_name == "health_check" {
        return health_tools::handle_health_check(&params);
    }

    let ToolCallParams {
        id,
        tool_name,
        arguments,
        config,
        index_set,
        schema_status,
        compatibility_reason,
        conn,
        workspace,
        project_id,
        ..
    } = params;

    match tool_name {
        "locate_symbol" => query_tools::handle_locate_symbol(QueryToolParams {
            id,
            arguments,
            config,
            index_set,
            schema_status,
            compatibility_reason,
            conn,
            workspace,
            project_id,
        }),
        "search_code" => query_tools::handle_search_code(QueryToolParams {
            id,
            arguments,
            config,
            index_set,
            schema_status,
            compatibility_reason,
            conn,
            workspace,
            project_id,
        }),
        "get_symbol_hierarchy" => structure_tools::handle_get_symbol_hierarchy(ReadToolParams {
            id,
            arguments,
            config,
            schema_status,
            compatibility_reason,
            conn,
            workspace,
            project_id,
        }),
        "find_related_symbols" => structure_tools::handle_find_related_symbols(ReadToolParams {
            id,
            arguments,
            config,
            schema_status,
            compatibility_reason,
            conn,
            workspace,
            project_id,
        }),
        "get_code_context" => context_tools::handle_get_code_context(QueryToolParams {
            id,
            arguments,
            config,
            index_set,
            schema_status,
            compatibility_reason,
            conn,
            workspace,
            project_id,
        }),
        "get_file_outline" => structure_tools::handle_get_file_outline(ReadToolParams {
            id,
            arguments,
            config,
            schema_status,
            compatibility_reason,
            conn,
            workspace,
            project_id,
        }),
        "index_status" => status_tools::handle_index_status(ReadToolParams {
            id,
            arguments,
            config,
            schema_status,
            compatibility_reason,
            conn,
            workspace,
            project_id,
        }),
        "index_repo" | "sync_repo" => index_tools::handle_index_or_sync(IndexToolParams {
            id,
            tool_name,
            arguments,
            config,
            schema_status,
            conn,
            workspace,
            project_id,
        }),
        _ => JsonRpcResponse::error(id, -32601, format!("Unknown tool: {}", tool_name)),
    }
}
