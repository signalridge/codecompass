use super::ExtractedSymbol;
use codecompass_core::types::SymbolKind;

pub fn extract(tree: &tree_sitter::Tree, source: &str) -> Vec<ExtractedSymbol> {
    let mut symbols = Vec::new();
    let root = tree.root_node();
    extract_from_node(root, source, &mut symbols);
    symbols
}

fn extract_from_node(node: tree_sitter::Node, source: &str, symbols: &mut Vec<ExtractedSymbol>) {
    match node.kind() {
        "function_declaration" => {
            if let Some(sym) = extract_function(node, source) {
                symbols.push(sym);
            }
        }
        "method_declaration" => {
            if let Some(sym) = extract_method(node, source) {
                symbols.push(sym);
            }
        }
        "type_declaration" => {
            for i in 0..node.child_count() {
                if let Some(child) = node.child(i) {
                    if child.kind() != "type_spec" {
                        continue;
                    }
                    if let Some(sym) = extract_type_spec(child, source) {
                        symbols.push(sym);
                    }
                }
            }
        }
        "const_declaration" | "var_declaration" => {
            for i in 0..node.child_count() {
                if let Some(child) = node.child(i) {
                    if child.kind() != "const_spec" && child.kind() != "var_spec" {
                        continue;
                    }
                    if let Some(name_node) = child.child_by_field_name("name") {
                        let name = node_text(name_node, source);
                        let kind = if node.kind() == "const_declaration" {
                            SymbolKind::Constant
                        } else {
                            SymbolKind::Variable
                        };
                        let vis = if name.chars().next().is_some_and(|c| c.is_uppercase()) {
                            Some("public".into())
                        } else {
                            Some("private".into())
                        };
                        symbols.push(ExtractedSymbol {
                            name: name.clone(),
                            qualified_name: name,
                            kind,
                            language: "go".into(),
                            signature: None,
                            line_start: child.start_position().row as u32 + 1,
                            line_end: child.end_position().row as u32 + 1,
                            visibility: vis,
                            parent_name: None,
                            body: Some(node_text(child, source)),
                        });
                    }
                }
            }
        }
        _ => {}
    }

    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            extract_from_node(child, source, symbols);
        }
    }
}

fn extract_function(node: tree_sitter::Node, source: &str) -> Option<ExtractedSymbol> {
    let name_node = node.child_by_field_name("name")?;
    let name = node_text(name_node, source);
    let sig = node_text(node, source).lines().next()?.trim().to_string();
    let vis = if name.chars().next().is_some_and(|c| c.is_uppercase()) {
        Some("public".into())
    } else {
        Some("private".into())
    };

    Some(ExtractedSymbol {
        name: name.clone(),
        qualified_name: name,
        kind: SymbolKind::Function,
        language: "go".into(),
        signature: Some(sig),
        line_start: node.start_position().row as u32 + 1,
        line_end: node.end_position().row as u32 + 1,
        visibility: vis,
        parent_name: None,
        body: Some(node_text(node, source)),
    })
}

fn extract_method(node: tree_sitter::Node, source: &str) -> Option<ExtractedSymbol> {
    let name_node = node.child_by_field_name("name")?;
    let name = node_text(name_node, source);
    let sig = node_text(node, source).lines().next()?.trim().to_string();

    // Extract receiver type
    let receiver = node.child_by_field_name("receiver").and_then(|r| {
        let mut found = None;
        for i in 0..r.child_count() {
            if let Some(c) = r.child(i)
                && c.kind() == "parameter_declaration"
            {
                found = c
                    .child_by_field_name("type")
                    .map(|t| node_text(t, source).replace('*', ""));
                break;
            }
        }
        found
    });

    let vis = if name.chars().next().is_some_and(|c| c.is_uppercase()) {
        Some("public".into())
    } else {
        Some("private".into())
    };

    Some(ExtractedSymbol {
        name: name.clone(),
        qualified_name: match &receiver {
            Some(r) => format!("{}.{}", r, name),
            None => name.clone(),
        },
        kind: SymbolKind::Method,
        language: "go".into(),
        signature: Some(sig),
        line_start: node.start_position().row as u32 + 1,
        line_end: node.end_position().row as u32 + 1,
        visibility: vis,
        parent_name: receiver,
        body: Some(node_text(node, source)),
    })
}

fn extract_type_spec(node: tree_sitter::Node, source: &str) -> Option<ExtractedSymbol> {
    let name_node = node.child_by_field_name("name")?;
    let name = node_text(name_node, source);

    let type_node = node.child_by_field_name("type")?;
    let kind = match type_node.kind() {
        "struct_type" => SymbolKind::Struct,
        "interface_type" => SymbolKind::Interface,
        _ => SymbolKind::TypeAlias,
    };

    let vis = if name.chars().next().is_some_and(|c| c.is_uppercase()) {
        Some("public".into())
    } else {
        Some("private".into())
    };

    Some(ExtractedSymbol {
        name: name.clone(),
        qualified_name: name,
        kind,
        language: "go".into(),
        signature: None,
        line_start: node.start_position().row as u32 + 1,
        line_end: node.end_position().row as u32 + 1,
        visibility: vis,
        parent_name: None,
        body: Some(node_text(node, source)),
    })
}

fn node_text(node: tree_sitter::Node, source: &str) -> String {
    source[node.byte_range()].to_string()
}
