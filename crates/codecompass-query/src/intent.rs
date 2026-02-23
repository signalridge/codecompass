use codecompass_core::types::QueryIntent;

/// Classify a search query into an intent category.
pub fn classify_intent(query: &str) -> QueryIntent {
    let trimmed = query.trim();

    // Path intent: contains / or has file extension
    if is_path_query(trimmed) {
        return QueryIntent::Path;
    }

    // Error intent: contains quotes, stack trace patterns
    if is_error_query(trimmed) {
        return QueryIntent::Error;
    }

    // Symbol intent: looks like an identifier (CamelCase or snake_case)
    if is_symbol_query(trimmed) {
        return QueryIntent::Symbol;
    }

    // Default: natural language
    QueryIntent::NaturalLanguage
}

fn is_path_query(query: &str) -> bool {
    // Contains path separators
    if query.contains('/') || query.contains('\\') {
        return true;
    }
    // Looks like a filename with extension
    let known_extensions = [
        ".rs", ".ts", ".tsx", ".js", ".jsx", ".py", ".go", ".java", ".c", ".h", ".cpp", ".rb",
        ".swift",
    ];
    for ext in &known_extensions {
        if query.ends_with(ext) {
            return true;
        }
    }
    false
}

fn is_error_query(query: &str) -> bool {
    // Contains quoted strings
    if query.contains('"') || query.contains('\'') {
        return true;
    }
    // Stack trace patterns
    let error_patterns = [
        "error:",
        "Error:",
        "panic:",
        "FATAL",
        "exception",
        "Exception",
        "traceback",
        "at line",
        "thread '",
    ];
    for pattern in &error_patterns {
        if query.contains(pattern) {
            return true;
        }
    }
    false
}

fn is_symbol_query(query: &str) -> bool {
    let words: Vec<&str> = query.split_whitespace().collect();

    // Single word that looks like an identifier
    if words.len() == 1 {
        let word = words[0];
        // CamelCase: has at least one uppercase after first char
        if word.len() > 1 && word.chars().skip(1).any(|c| c.is_uppercase()) {
            return true;
        }
        // snake_case: contains underscore
        if word.contains('_') {
            return true;
        }
        // Dotted: qualified name
        if word.contains("::") || (word.contains('.') && !is_path_query(word)) {
            return true;
        }
        // All alphanumeric, looks like an identifier
        if word.chars().all(|c| c.is_alphanumeric() || c == '_') && word.len() > 2 {
            return true;
        }
    }

    // Two words where one looks like a kind
    if words.len() == 2 {
        let kinds = [
            "fn",
            "func",
            "function",
            "struct",
            "class",
            "enum",
            "trait",
            "interface",
            "type",
            "const",
            "method",
        ];
        if kinds.contains(&words[0].to_lowercase().as_str()) {
            return true;
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_symbol_intent() {
        assert_eq!(classify_intent("validate_token"), QueryIntent::Symbol);
        assert_eq!(classify_intent("AuthHandler"), QueryIntent::Symbol);
        assert_eq!(classify_intent("auth::jwt::validate"), QueryIntent::Symbol);
    }

    #[test]
    fn test_path_intent() {
        assert_eq!(classify_intent("src/auth/handler.rs"), QueryIntent::Path);
        assert_eq!(classify_intent("handler.rs"), QueryIntent::Path);
    }

    #[test]
    fn test_error_intent() {
        assert_eq!(
            classify_intent("\"connection refused\""),
            QueryIntent::Error
        );
        assert_eq!(
            classify_intent("error: cannot find module"),
            QueryIntent::Error
        );
    }

    #[test]
    fn test_natural_language_intent() {
        assert_eq!(
            classify_intent("where is rate limiting implemented"),
            QueryIntent::NaturalLanguage
        );
        assert_eq!(
            classify_intent("how does authentication work"),
            QueryIntent::NaturalLanguage
        );
    }
}
