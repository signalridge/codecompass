pub mod go;
pub mod python;
pub mod rust;
pub mod typescript;

use codecompass_core::types::SymbolKind;

/// Extracted symbol from tree-sitter.
#[derive(Debug, Clone)]
pub struct ExtractedSymbol {
    pub name: String,
    pub qualified_name: String,
    pub kind: SymbolKind,
    pub language: String,
    pub signature: Option<String>,
    pub line_start: u32,
    pub line_end: u32,
    pub visibility: Option<String>,
    pub parent_name: Option<String>,
    pub body: Option<String>,
}

/// Extract symbols from a parsed tree for a given language.
pub fn extract_symbols(
    tree: &tree_sitter::Tree,
    source: &str,
    language: &str,
) -> Vec<ExtractedSymbol> {
    match language {
        "rust" => rust::extract(tree, source),
        "typescript" => typescript::extract(tree, source),
        "python" => python::extract(tree, source),
        "go" => go::extract(tree, source),
        _ => Vec::new(),
    }
}
