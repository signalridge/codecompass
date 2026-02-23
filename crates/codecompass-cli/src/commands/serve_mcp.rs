use anyhow::{Context, Result};
use std::path::Path;

pub fn run(workspace: &Path, config_file: Option<&Path>) -> Result<()> {
    let workspace = std::fs::canonicalize(workspace).context("Failed to resolve workspace path")?;

    codecompass_mcp::server::run_server(&workspace, config_file)
        .map_err(|e| anyhow::anyhow!("MCP server error: {}", e))
}
