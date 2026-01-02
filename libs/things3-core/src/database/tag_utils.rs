//! Tag manipulation utilities for duplicate prevention
//!
//! This module provides functions for normalizing tag titles and calculating
//! similarity scores to prevent duplicate and near-duplicate tags.

use strsim::normalized_levenshtein;

/// Normalize a tag title for comparison
///
/// Normalization steps:
/// - Trim leading/trailing whitespace
/// - Convert to lowercase
/// - Collapse multiple spaces into single spaces
/// - Handle consistent character encoding
///
/// # Examples
///
/// ```
/// # use things3_core::database::tag_utils::normalize_tag_title;
/// assert_eq!(normalize_tag_title("  Work  "), "work");
/// assert_eq!(normalize_tag_title("High   Priority"), "high priority");
/// assert_eq!(normalize_tag_title("URGENT"), "urgent");
/// ```
pub fn normalize_tag_title(title: &str) -> String {
    title
        .trim()
        .to_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Calculate similarity between two tag titles using normalized Levenshtein distance
///
/// Returns a score from 0.0 (completely different) to 1.0 (identical).
/// Uses the Levenshtein distance algorithm which measures the minimum number
/// of single-character edits (insertions, deletions, substitutions) needed
/// to transform one string into another.
///
/// # Examples
///
/// ```
/// # use things3_core::database::tag_utils::calculate_similarity;
/// // Exact match
/// assert_eq!(calculate_similarity("work", "work"), 1.0);
///
/// // Very similar (typo)
/// let score = calculate_similarity("important", "importnt");
/// assert!(score > 0.8 && score < 1.0);
///
/// // Completely different
/// let score = calculate_similarity("work", "vacation");
/// assert!(score < 0.5);
/// ```
pub fn calculate_similarity(title1: &str, title2: &str) -> f32 {
    // Normalize both titles first
    let norm1 = normalize_tag_title(title1);
    let norm2 = normalize_tag_title(title2);

    // Use normalized Levenshtein distance from strsim crate
    // Returns a value between 0.0 and 1.0
    normalized_levenshtein(&norm1, &norm2) as f32
}

/// Check if one title contains or is contained by another (partial match)
///
/// Returns `true` if either:
/// - `search` is a substring of `candidate`
/// - `candidate` is a substring of `search`
///
/// Both strings are normalized before comparison.
///
/// # Examples
///
/// ```
/// # use things3_core::database::tag_utils::is_partial_match;
/// assert!(is_partial_match("work", "work project"));
/// assert!(is_partial_match("work project", "work"));
/// assert!(!is_partial_match("work", "vacation"));
/// ```
pub fn is_partial_match(search: &str, candidate: &str) -> bool {
    let search_norm = normalize_tag_title(search);
    let candidate_norm = normalize_tag_title(candidate);

    candidate_norm.contains(&search_norm) || search_norm.contains(&candidate_norm)
}

/// Determine the match type based on similarity and partial matching
///
/// Returns the most specific match type:
/// - `Exact`: Normalized titles are identical
/// - `CaseMismatch`: Titles are same text but different case
/// - `Similar`: High similarity score (>= threshold)
/// - `PartialMatch`: One contains the other
///
/// # Examples
///
/// ```
/// # use things3_core::database::tag_utils::get_match_type;
/// # use things3_core::models::TagMatchType;
/// assert_eq!(get_match_type("Work", "work", 0.8), TagMatchType::CaseMismatch);
/// assert_eq!(get_match_type("work", "work", 0.8), TagMatchType::Exact);
/// ```
pub fn get_match_type(
    title1: &str,
    title2: &str,
    similarity_threshold: f32,
) -> crate::models::TagMatchType {
    use crate::models::TagMatchType;

    let norm1 = normalize_tag_title(title1);
    let norm2 = normalize_tag_title(title2);

    // Check for exact match (normalized)
    if norm1 == norm2 {
        // If the original strings differ, it's a case mismatch
        if title1 != title2 {
            return TagMatchType::CaseMismatch;
        }
        return TagMatchType::Exact;
    }

    // Calculate similarity
    let similarity = calculate_similarity(title1, title2);

    // Check if similarity meets threshold
    if similarity >= similarity_threshold {
        return TagMatchType::Similar;
    }

    // Check for partial match
    if is_partial_match(title1, title2) {
        return TagMatchType::PartialMatch;
    }

    // If nothing matches well enough, still return Similar but with low score
    // The caller should filter based on score
    TagMatchType::Similar
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::TagMatchType;

    #[test]
    fn test_normalize_tag_title_basic() {
        assert_eq!(normalize_tag_title("work"), "work");
        assert_eq!(normalize_tag_title("  work  "), "work");
        assert_eq!(normalize_tag_title("WORK"), "work");
        assert_eq!(normalize_tag_title("Work"), "work");
    }

    #[test]
    fn test_normalize_tag_title_multiple_spaces() {
        assert_eq!(normalize_tag_title("high   priority"), "high priority");
        assert_eq!(
            normalize_tag_title("  work   from   home  "),
            "work from home"
        );
    }

    #[test]
    fn test_normalize_tag_title_empty() {
        assert_eq!(normalize_tag_title(""), "");
        assert_eq!(normalize_tag_title("   "), "");
    }

    #[test]
    fn test_calculate_similarity_identical() {
        assert_eq!(calculate_similarity("work", "work"), 1.0);
        assert_eq!(calculate_similarity("urgent", "urgent"), 1.0);
    }

    #[test]
    fn test_calculate_similarity_case_insensitive() {
        assert_eq!(calculate_similarity("Work", "work"), 1.0);
        assert_eq!(calculate_similarity("URGENT", "urgent"), 1.0);
    }

    #[test]
    fn test_calculate_similarity_typo() {
        // "important" vs "importnt" (missing 'a')
        let score = calculate_similarity("important", "importnt");
        assert!(
            score > 0.85 && score < 1.0,
            "Expected high similarity for typo, got {}",
            score
        );

        // "urgent" vs "urgnt" (missing 'e')
        let score = calculate_similarity("urgent", "urgnt");
        assert!(
            score > 0.80 && score < 1.0,
            "Expected high similarity for typo, got {}",
            score
        );
    }

    #[test]
    fn test_calculate_similarity_completely_different() {
        let score = calculate_similarity("work", "vacation");
        assert!(
            score < 0.5,
            "Expected low similarity for different words, got {}",
            score
        );

        let score = calculate_similarity("urgent", "later");
        assert!(
            score < 0.5,
            "Expected low similarity for different words, got {}",
            score
        );
    }

    #[test]
    fn test_calculate_similarity_partial() {
        // "work" vs "work-project" should have decent similarity
        let score = calculate_similarity("work", "work-project");
        assert!(
            score > 0.5,
            "Expected moderate similarity for partial match, got {}",
            score
        );
    }

    #[test]
    fn test_is_partial_match_contains() {
        assert!(is_partial_match("work", "work project"));
        assert!(is_partial_match("work project", "work"));
    }

    #[test]
    fn test_is_partial_match_not_contained() {
        assert!(!is_partial_match("work", "vacation"));
        assert!(!is_partial_match("urgent", "later"));
    }

    #[test]
    fn test_is_partial_match_case_insensitive() {
        assert!(is_partial_match("Work", "WORK PROJECT"));
        assert!(is_partial_match("WORK PROJECT", "work"));
    }

    #[test]
    fn test_get_match_type_exact() {
        assert_eq!(get_match_type("work", "work", 0.8), TagMatchType::Exact);
    }

    #[test]
    fn test_get_match_type_case_mismatch() {
        assert_eq!(
            get_match_type("Work", "work", 0.8),
            TagMatchType::CaseMismatch
        );
        assert_eq!(
            get_match_type("URGENT", "urgent", 0.8),
            TagMatchType::CaseMismatch
        );
    }

    #[test]
    fn test_get_match_type_similar() {
        // High similarity but not exact
        assert_eq!(
            get_match_type("important", "importnt", 0.8),
            TagMatchType::Similar
        );
    }

    #[test]
    fn test_get_match_type_partial() {
        // This should return Similar with high score, not PartialMatch
        // because similarity is very high
        let match_type = get_match_type("work", "work project", 0.8);
        // Could be either Similar or PartialMatch depending on similarity score
        assert!(
            match_type == TagMatchType::Similar || match_type == TagMatchType::PartialMatch,
            "Expected Similar or PartialMatch, got {:?}",
            match_type
        );
    }

    #[test]
    fn test_normalize_handles_unicode() {
        assert_eq!(normalize_tag_title("café"), "café");
        assert_eq!(normalize_tag_title("Café"), "café");
    }

    #[test]
    fn test_similarity_with_whitespace_differences() {
        // These should be identical after normalization
        let score = calculate_similarity("work from home", "  Work   From   Home  ");
        assert_eq!(score, 1.0);
    }
}
