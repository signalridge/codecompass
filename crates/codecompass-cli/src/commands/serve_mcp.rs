use anyhow::{Context, Result};
use codecompass_core::types::WorkspaceConfig;
use std::path::Path;

pub fn run(
    workspace: &Path,
    config_file: Option<&Path>,
    no_prewarm: bool,
    workspace_config: WorkspaceConfig,
) -> Result<()> {
    let workspace = std::fs::canonicalize(workspace).context("Failed to resolve workspace path")?;

    codecompass_mcp::server::run_server(&workspace, config_file, no_prewarm, workspace_config)
        .map_err(|e| anyhow::anyhow!("MCP server error: {}", e))
}
