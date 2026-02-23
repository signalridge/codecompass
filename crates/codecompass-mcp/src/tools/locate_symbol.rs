use super::ToolDefinition;
use serde_json::json;

pub fn definition() -> ToolDefinition {
    ToolDefinition {
        name: "locate_symbol".into(),
        description: "Find symbol definitions by name. Returns precise file:line locations.".into(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "name": {
                    "type": "string",
                    "description": "Symbol name to locate"
                },
                "kind": {
                    "type": "string",
                    "description": "Filter by kind (fn, struct, class, method, etc.)"
                },
                "language": {
                    "type": "string",
                    "description": "Filter by language"
                },
                "ref": {
                    "type": "string",
                    "description": "Branch/ref scope"
                },
                "limit": {
                    "type": "integer",
                    "description": "Max results (default: 10)"
                }
            },
            "required": ["name"]
        }),
    }
}
