use crate::search::SearchResult;

/// Apply rule-based reranking boosts to search results.
pub fn rerank(results: &mut [SearchResult], query: &str) {
    let query_lower = query.to_lowercase();

    for result in results.iter_mut() {
        let mut boost = 0.0_f32;

        // Exact symbol name match boost
        if let Some(ref name) = result.name
            && name.to_lowercase() == query_lower
        {
            boost += 5.0;
        }

        // Qualified name match boost
        if let Some(ref qn) = result.qualified_name
            && qn.to_lowercase().contains(&query_lower)
        {
            boost += 2.0;
        }

        // Definition-over-reference boost (definitions are kind != "reference")
        if result.result_type == "symbol" {
            boost += 1.0;
        }

        // Path affinity boost (if query partially matches path)
        if result.path.to_lowercase().contains(&query_lower) {
            boost += 1.0;
        }

        result.score += boost;
    }

    // Re-sort by score, with stable tiebreaker on result_id for determinism
    results.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.result_id.cmp(&b.result_id))
    });
}
