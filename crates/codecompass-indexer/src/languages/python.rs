use super::ExtractedSymbol;
use codecompass_core::types::SymbolKind;

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
    match node.kind() {
        "function_definition" => {
            if let Some(sym) = extract_function(node, source, parent) {
                symbols.push(sym);
            }
        }
        "class_definition" => {
            if let Some(sym) = extract_class(node, source, parent) {
                let name = sym.name.clone();
                symbols.push(sym);
                // Extract methods inside the class body
                if let Some(body) = node.child_by_field_name("body") {
                    extract_children(body, source, Some(&name), symbols);
                }
                return;
            }
        }
        "decorated_definition" => {
            // Skip decorator, extract the actual definition
            for i in 0..node.child_count() {
                if let Some(child) = node.child(i)
                    && (child.kind() == "function_definition" || child.kind() == "class_definition")
                {
                    extract_from_node(child, source, parent, symbols);
                }
            }
            return;
        }
        "expression_statement" => {
            // Module-level assignments
            for i in 0..node.child_count() {
                if let Some(child) = node.child(i) {
                    if child.kind() != "assignment" || parent.is_some() {
                        continue;
                    }
                    if let Some(left) = child.child_by_field_name("left") {
                        if left.kind() != "identifier" {
                            continue;
                        }
                        let name = node_text(left, source);
                        symbols.push(ExtractedSymbol {
                            name: name.clone(),
                            qualified_name: name,
                            kind: SymbolKind::Variable,
                            language: "python".into(),
                            signature: None,
                            line_start: node.start_position().row as u32 + 1,
                            line_end: node.end_position().row as u32 + 1,
                            visibility: None,
                            parent_name: None,
                            body: Some(node_text(node, source)),
                        });
                    }
                }
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
    let sig = node_text(node, source).lines().next()?.trim().to_string();

    let kind = if parent.is_some() {
        SymbolKind::Method
    } else {
        SymbolKind::Function
    };

    Some(ExtractedSymbol {
        name: name.clone(),
        qualified_name: make_qualified(parent, &name),
        kind,
        language: "python".into(),
        signature: Some(sig),
        line_start: node.start_position().row as u32 + 1,
        line_end: node.end_position().row as u32 + 1,
        visibility: if name.starts_with('_') {
            Some("private".into())
        } else {
            Some("public".into())
        },
        parent_name: parent.map(String::from),
        body: Some(node_text(node, source)),
    })
}

fn extract_class(
    node: tree_sitter::Node,
    source: &str,
    parent: Option<&str>,
) -> Option<ExtractedSymbol> {
    let name_node = node.child_by_field_name("name")?;
    let name = node_text(name_node, source);

    Some(ExtractedSymbol {
        name: name.clone(),
        qualified_name: make_qualified(parent, &name),
        kind: SymbolKind::Class,
        language: "python".into(),
        signature: None,
        line_start: node.start_position().row as u32 + 1,
        line_end: node.end_position().row as u32 + 1,
        visibility: None,
        parent_name: parent.map(String::from),
        body: Some(node_text(node, source)),
    })
}

fn make_qualified(parent: Option<&str>, name: &str) -> String {
    match parent {
        Some(p) => format!("{}.{}", p, name),
        None => name.to_string(),
    }
}

fn node_text(node: tree_sitter::Node, source: &str) -> String {
    source[node.byte_range()].to_string()
}
