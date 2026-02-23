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
        "function_declaration" | "function" => {
            if let Some(sym) = extract_function(node, source, parent) {
                symbols.push(sym);
            }
        }
        "class_declaration" => {
            if let Some(sym) = extract_named(node, source, parent, SymbolKind::Class) {
                let name = sym.name.clone();
                symbols.push(sym);
                extract_children(node, source, Some(&name), symbols);
                return;
            }
        }
        "interface_declaration" => {
            if let Some(sym) = extract_named(node, source, parent, SymbolKind::Interface) {
                symbols.push(sym);
            }
        }
        "enum_declaration" => {
            if let Some(sym) = extract_named(node, source, parent, SymbolKind::Enum) {
                symbols.push(sym);
            }
        }
        "type_alias_declaration" => {
            if let Some(sym) = extract_named(node, source, parent, SymbolKind::TypeAlias) {
                symbols.push(sym);
            }
        }
        "method_definition" => {
            if let Some(sym) = extract_method(node, source, parent) {
                symbols.push(sym);
            }
        }
        "lexical_declaration" | "variable_declaration" => {
            // Extract const/let/var declarations
            for i in 0..node.child_count() {
                if let Some(child) = node.child(i) {
                    if child.kind() != "variable_declarator" {
                        continue;
                    }
                    if let Some(name_node) = child.child_by_field_name("name") {
                        let name = node_text(name_node, source);
                        symbols.push(ExtractedSymbol {
                            name: name.clone(),
                            qualified_name: make_qualified(parent, &name),
                            kind: SymbolKind::Constant,
                            language: "typescript".into(),
                            signature: None,
                            line_start: node.start_position().row as u32 + 1,
                            line_end: node.end_position().row as u32 + 1,
                            visibility: None,
                            parent_name: parent.map(String::from),
                            body: Some(node_text(node, source)),
                        });
                    }
                }
            }
        }
        "export_statement" => {
            // Look inside export for declarations
            extract_children(node, source, parent, symbols);
            return;
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

    Some(ExtractedSymbol {
        name: name.clone(),
        qualified_name: make_qualified(parent, &name),
        kind: if parent.is_some() {
            SymbolKind::Method
        } else {
            SymbolKind::Function
        },
        language: "typescript".into(),
        signature: Some(sig),
        line_start: node.start_position().row as u32 + 1,
        line_end: node.end_position().row as u32 + 1,
        visibility: None,
        parent_name: parent.map(String::from),
        body: Some(node_text(node, source)),
    })
}

fn extract_method(
    node: tree_sitter::Node,
    source: &str,
    parent: Option<&str>,
) -> Option<ExtractedSymbol> {
    let name_node = node.child_by_field_name("name")?;
    let name = node_text(name_node, source);
    let sig = node_text(node, source).lines().next()?.trim().to_string();

    Some(ExtractedSymbol {
        name: name.clone(),
        qualified_name: make_qualified(parent, &name),
        kind: SymbolKind::Method,
        language: "typescript".into(),
        signature: Some(sig),
        line_start: node.start_position().row as u32 + 1,
        line_end: node.end_position().row as u32 + 1,
        visibility: None,
        parent_name: parent.map(String::from),
        body: Some(node_text(node, source)),
    })
}

fn extract_named(
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
        language: "typescript".into(),
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
