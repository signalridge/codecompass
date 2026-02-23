use crate::languages::ExtractedSymbol;
use codecompass_core::types::SnippetRecord;

/// Build SnippetRecords from extracted symbols.
/// Each function/method/class body becomes a snippet.
pub fn build_snippet_records(
    extracted: &[ExtractedSymbol],
    repo: &str,
    r#ref: &str,
    path: &str,
    commit: Option<&str>,
) -> Vec<SnippetRecord> {
    extracted
        .iter()
        .filter_map(|sym| {
            let body = sym.body.as_ref()?;
            if body.trim().is_empty() {
                return None;
            }

            let chunk_type = match sym.kind {
                codecompass_core::types::SymbolKind::Function => "function_body",
                codecompass_core::types::SymbolKind::Method => "method_body",
                codecompass_core::types::SymbolKind::Class => "class_body",
                codecompass_core::types::SymbolKind::Struct => "struct_body",
                codecompass_core::types::SymbolKind::Trait => "trait_body",
                codecompass_core::types::SymbolKind::Interface => "interface_body",
                codecompass_core::types::SymbolKind::Module => "module_body",
                _ => return None, // Skip constants, variables, etc.
            };

            Some(SnippetRecord {
                repo: repo.to_string(),
                r#ref: r#ref.to_string(),
                commit: commit.map(String::from),
                path: path.to_string(),
                language: sym.language.clone(),
                chunk_type: chunk_type.to_string(),
                imports: None,
                line_start: sym.line_start,
                line_end: sym.line_end,
                content: body.clone(),
            })
        })
        .collect()
}
