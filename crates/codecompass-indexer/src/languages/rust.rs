use super::ExtractedSymbol;
use codecompass_core::types::SymbolKind;

/// Extract symbols from a Rust syntax tree.
pub fn extract(tree: &tree_sitter::Tree, source: &str) -> Vec<ExtractedSymbol> {
    let mut symbols = Vec::new();
    let root = tree.root_node();
    extract_from_node(root, source, None, &mut symbols);
    symbols
}

fn extract_from_node(
    node: tree_sitter::Node,
    source: &str,
    parent: Option<&str>,
    symbols: &mut Vec<ExtractedSymbol>,
) {
    let kind_str = node.kind();

    match kind_str {
        "function_item" => {
            if let Some(sym) = extract_function(node, source, parent) {
                symbols.push(sym);
            }
        }
        "struct_item" => {
            if let Some(sym) = extract_named_item(node, source, parent, SymbolKind::Struct) {
                let name = sym.name.clone();
                symbols.push(sym);
                // Extract methods inside impl blocks are handled separately
                extract_children(node, source, Some(&name), symbols);
                return;
            }
        }
        "enum_item" => {
            if let Some(sym) = extract_named_item(node, source, parent, SymbolKind::Enum) {
                symbols.push(sym);
            }
        }
        "trait_item" => {
            if let Some(sym) = extract_named_item(node, source, parent, SymbolKind::Trait) {
                let name = sym.name.clone();
                symbols.push(sym);
                extract_children(node, source, Some(&name), symbols);
                return;
            }
        }
        "impl_item" => {
            // Get the type name being implemented
            let type_name = node
                .child_by_field_name("type")
                .map(|n| node_text(n, source));
            extract_children(node, source, type_name.as_deref(), symbols);
            return;
        }
        "const_item" | "static_item" => {
            if let Some(sym) = extract_named_item(node, source, parent, SymbolKind::Constant) {
                symbols.push(sym);
            }
        }
        "type_item" => {
            if let Some(sym) = extract_named_item(node, source, parent, SymbolKind::TypeAlias) {
                symbols.push(sym);
            }
        }
        "mod_item" => {
            if let Some(name_node) = node.child_by_field_name("name") {
                let name = node_text(name_node, source);
                symbols.push(ExtractedSymbol {
                    name: name.clone(),
                    qualified_name: make_qualified(parent, &name),
                    kind: SymbolKind::Module,
                    language: "rust".into(),
                    signature: None,
                    line_start: node.start_position().row as u32 + 1,
                    line_end: node.end_position().row as u32 + 1,
                    visibility: extract_visibility(node, source),
                    parent_name: parent.map(String::from),
                    body: Some(node_text(node, source)),
                });
                extract_children(node, source, Some(&name), symbols);
                return;
            }
        }
        _ => {}
    }

    extract_children(node, source, parent, symbols);
}

fn extract_children(
    node: tree_sitter::Node,
    source: &str,
    parent: Option<&str>,
    symbols: &mut Vec<ExtractedSymbol>,
) {
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            extract_from_node(child, source, parent, symbols);
        }
    }
}

fn extract_function(
    node: tree_sitter::Node,
    source: &str,
    parent: Option<&str>,
) -> Option<ExtractedSymbol> {
    let name_node = node.child_by_field_name("name")?;
    let name = node_text(name_node, source);

    // Build signature from the function definition line
    let sig = extract_signature(node, source);

    let kind = if parent.is_some() {
        SymbolKind::Method
    } else {
        SymbolKind::Function
    };

    Some(ExtractedSymbol {
        name: name.clone(),
        qualified_name: make_qualified(parent, &name),
        kind,
        language: "rust".into(),
        signature: Some(sig),
        line_start: node.start_position().row as u32 + 1,
        line_end: node.end_position().row as u32 + 1,
        visibility: extract_visibility(node, source),
        parent_name: parent.map(String::from),
        body: Some(node_text(node, source)),
    })
}

fn extract_named_item(
    node: tree_sitter::Node,
    source: &str,
    parent: Option<&str>,
    kind: SymbolKind,
) -> Option<ExtractedSymbol> {
    let name_node = node.child_by_field_name("name")?;
    let name = node_text(name_node, source);

    Some(ExtractedSymbol {
        name: name.clone(),
        qualified_name: make_qualified(parent, &name),
        kind,
        language: "rust".into(),
        signature: None,
        line_start: node.start_position().row as u32 + 1,
        line_end: node.end_position().row as u32 + 1,
        visibility: extract_visibility(node, source),
        parent_name: parent.map(String::from),
        body: Some(node_text(node, source)),
    })
}

fn extract_signature(node: tree_sitter::Node, source: &str) -> String {
    // Take the first line of the function as the signature
    let text = node_text(node, source);
    text.lines().next().unwrap_or("").trim().to_string()
}

fn extract_visibility(node: tree_sitter::Node, source: &str) -> Option<String> {
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i)
            && child.kind() == "visibility_modifier"
        {
            return Some(node_text(child, source));
        }
    }
    None
}

fn make_qualified(parent: Option<&str>, name: &str) -> String {
    match parent {
        Some(p) => format!("{}::{}", p, name),
        None => name.to_string(),
    }
}

fn node_text(node: tree_sitter::Node, source: &str) -> String {
    source[node.byte_range()].to_string()
}
