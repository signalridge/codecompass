use super::ToolDefinition;
use serde_json::json;

pub fn definition() -> ToolDefinition {
    ToolDefinition {
        name: "index_status".into(),
        description: "Get current indexing status and job history for a project.".into(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "ref": {
                    "type": "string",
                    "description": "Branch/ref scope"
                }
            }
        }),
    }
}
