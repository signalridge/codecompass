use super::ToolDefinition;
use serde_json::json;

pub fn definition() -> ToolDefinition {
    ToolDefinition {
        name: "search_code".into(),
        description: "Search across symbols, snippets, and files with query intent classification."
            .into(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Search query (symbol name, path, error string, or natural language)"
                },
                "ref": {
                    "type": "string",
                    "description": "Branch/ref scope"
                },
                "language": {
                    "type": "string",
                    "description": "Filter by language"
                },
                "limit": {
                    "type": "integer",
                    "description": "Max results (default: 10)"
                }
            },
            "required": ["query"]
        }),
    }
}
