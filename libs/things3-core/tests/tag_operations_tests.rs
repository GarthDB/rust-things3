//! Tag operations tests - comprehensive coverage for tag management with duplicate prevention

use chrono::Utc;
use things3_core::{
    database::{tag_utils::*, ThingsDatabase},
    models::{CreateTagRequest, TagCreationResult, TagMatchType, UpdateTagRequest},
};
use uuid::Uuid;

#[cfg(feature = "test-utils")]
use things3_core::test_utils::create_test_database;

// ========================================================================
// TAG NORMALIZATION AND SIMILARITY TESTS
// ========================================================================

#[test]
fn test_normalize_tag_title() {
    assert_eq!(normalize_tag_title("work"), "work");
    assert_eq!(normalize_tag_title("  work  "), "work");
    assert_eq!(normalize_tag_title("WORK"), "work");
    assert_eq!(normalize_tag_title("Work"), "work");
    assert_eq!(normalize_tag_title("high   priority"), "high priority");
    assert_eq!(
        normalize_tag_title("  Work   From   Home  "),
        "work from home"
    );
    assert_eq!(normalize_tag_title(""), "");
    assert_eq!(normalize_tag_title("   "), "");
}

#[test]
fn test_calculate_similarity() {
    // Identical
    assert_eq!(calculate_similarity("work", "work"), 1.0);

    // Case-insensitive
    assert_eq!(calculate_similarity("Work", "work"), 1.0);

    // High similarity (typo)
    let score = calculate_similarity("important", "importnt");
    assert!(
        score > 0.85 && score < 1.0,
        "Expected high similarity, got {}",
        score
    );

    // Low similarity (completely different)
    let score = calculate_similarity("work", "vacation");
    assert!(score < 0.5, "Expected low similarity, got {}", score);
}

#[test]
fn test_is_partial_match() {
    assert!(is_partial_match("work", "work project"));
    assert!(is_partial_match("work project", "work"));
    assert!(is_partial_match("Work", "WORK PROJECT"));
    assert!(!is_partial_match("work", "vacation"));
}

#[test]
fn test_get_match_type() {
    assert_eq!(get_match_type("work", "work", 0.8), TagMatchType::Exact);
    assert_eq!(
        get_match_type("Work", "work", 0.8),
        TagMatchType::CaseMismatch
    );
    assert_eq!(
        get_match_type("important", "importnt", 0.8),
        TagMatchType::Similar
    );
}

// ========================================================================
// TAG CRUD OPERATIONS TESTS
// ========================================================================

#[tokio::test]
#[cfg(feature = "test-utils")]
async fn test_create_tag_prevents_exact_duplicate() {
    let db = create_test_database().await.unwrap();

    // Create first tag
    let request1 = CreateTagRequest {
        title: "work".to_string(),
        shortcut: None,
        parent_uuid: None,
    };
    let result1 = db.create_tag_smart(request1).await.unwrap();
    assert!(matches!(result1, TagCreationResult::Created { .. }));

    // Try to create duplicate (case-insensitive)
    let request2 = CreateTagRequest {
        title: "Work".to_string(),
        shortcut: None,
        parent_uuid: None,
    };
    let result2 = db.create_tag_smart(request2).await.unwrap();
    match result2 {
        TagCreationResult::Existing { tag, is_new } => {
            assert!(!is_new);
            assert_eq!(tag.title.to_lowercase(), "work");
        }
        _ => panic!("Expected Existing variant, got {:?}", result2),
    }
}

#[tokio::test]
#[cfg(feature = "test-utils")]
async fn test_create_tag_suggests_similar() {
    let db = create_test_database().await.unwrap();

    // Create first tag
    let request1 = CreateTagRequest {
        title: "important".to_string(),
        shortcut: None,
        parent_uuid: None,
    };
    db.create_tag_force(request1).await.unwrap();

    // Try to create similar tag (typo)
    let request2 = CreateTagRequest {
        title: "importnt".to_string(),
        shortcut: None,
        parent_uuid: None,
    };
    let result2 = db.create_tag_smart(request2).await.unwrap();
    match result2 {
        TagCreationResult::SimilarFound { similar_tags, .. } => {
            assert!(!similar_tags.is_empty());
            assert!(similar_tags[0].similarity_score > 0.8);
        }
        _ => panic!("Expected SimilarFound variant, got {:?}", result2),
    }
}

#[tokio::test]
#[cfg(feature = "test-utils")]
async fn test_create_tag_force_skips_check() {
    let db = create_test_database().await.unwrap();

    // Create first tag
    let request1 = CreateTagRequest {
        title: "work".to_string(),
        shortcut: None,
        parent_uuid: None,
    };
    db.create_tag_force(request1).await.unwrap();

    // Force create duplicate
    let request2 = CreateTagRequest {
        title: "work".to_string(),
        shortcut: None,
        parent_uuid: None,
    };
    let uuid2 = db.create_tag_force(request2).await.unwrap();
    assert!(!uuid2.is_nil());

    // Both should exist
    let all_tags = db.get_all_tags().await.unwrap();
    assert_eq!(all_tags.len(), 2);
}

#[tokio::test]
#[cfg(feature = "test-utils")]
async fn test_find_tag_by_normalized_title() {
    let db = create_test_database().await.unwrap();

    let request = CreateTagRequest {
        title: "Work".to_string(),
        shortcut: Some("w".to_string()),
        parent_uuid: None,
    };
    db.create_tag_force(request).await.unwrap();

    // Should find with different case
    let found = db.find_tag_by_normalized_title("work").await.unwrap();
    assert!(found.is_some());
    assert_eq!(found.unwrap().title, "Work"); // Original case preserved
}

#[tokio::test]
#[cfg(feature = "test-utils")]
async fn test_find_similar_tags_returns_sorted() {
    let db = create_test_database().await.unwrap();

    // Create several tags
    let tags = vec!["work", "working", "worker", "vacation"];
    for title in tags {
        let request = CreateTagRequest {
            title: title.to_string(),
            shortcut: None,
            parent_uuid: None,
        };
        db.create_tag_force(request).await.unwrap();
    }

    // Find similar to "work"
    let similar = db.find_similar_tags("work", 0.6).await.unwrap();
    assert!(!similar.is_empty());

    // Should be sorted by similarity (highest first)
    for i in 1..similar.len() {
        assert!(similar[i - 1].similarity_score >= similar[i].similarity_score);
    }
}

#[tokio::test]
#[cfg(feature = "test-utils")]
async fn test_update_tag_checks_duplicates() {
    let db = create_test_database().await.unwrap();

    // Create two tags
    let request1 = CreateTagRequest {
        title: "work".to_string(),
        shortcut: None,
        parent_uuid: None,
    };
    let uuid1 = db.create_tag_force(request1).await.unwrap();

    let request2 = CreateTagRequest {
        title: "personal".to_string(),
        shortcut: None,
        parent_uuid: None,
    };
    db.create_tag_force(request2).await.unwrap();

    // Try to rename tag1 to "personal" (should fail)
    let update_request = UpdateTagRequest {
        uuid: uuid1,
        title: Some("personal".to_string()),
        shortcut: None,
        parent_uuid: None,
    };
    let result = db.update_tag(update_request).await;
    assert!(result.is_err());
}

#[tokio::test]
#[cfg(feature = "test-utils")]
async fn test_update_tag_success() {
    let db = create_test_database().await.unwrap();

    let request = CreateTagRequest {
        title: "work".to_string(),
        shortcut: None,
        parent_uuid: None,
    };
    let uuid = db.create_tag_force(request).await.unwrap();

    // Update tag
    let update_request = UpdateTagRequest {
        uuid,
        title: Some("professional".to_string()),
        shortcut: Some("p".to_string()),
        parent_uuid: None,
    };
    db.update_tag(update_request).await.unwrap();

    // Verify update
    let found = db
        .find_tag_by_normalized_title("professional")
        .await
        .unwrap();
    assert!(found.is_some());
    let tag = found.unwrap();
    assert_eq!(tag.title, "professional");
    assert_eq!(tag.shortcut, Some("p".to_string()));
}

#[tokio::test]
#[cfg(feature = "test-utils")]
async fn test_delete_tag() {
    let db = create_test_database().await.unwrap();

    let request = CreateTagRequest {
        title: "work".to_string(),
        shortcut: None,
        parent_uuid: None,
    };
    let uuid = db.create_tag_force(request).await.unwrap();

    // Delete tag
    db.delete_tag(&uuid, false).await.unwrap();

    // Verify deletion
    let found = db.find_tag_by_normalized_title("work").await.unwrap();
    assert!(found.is_none());
}

#[tokio::test]
#[cfg(feature = "test-utils")]
async fn test_merge_tags() {
    let db = create_test_database().await.unwrap();

    // Create two tags
    let request1 = CreateTagRequest {
        title: "work".to_string(),
        shortcut: None,
        parent_uuid: None,
    };
    let source_uuid = db.create_tag_force(request1).await.unwrap();

    let request2 = CreateTagRequest {
        title: "professional".to_string(),
        shortcut: None,
        parent_uuid: None,
    };
    let target_uuid = db.create_tag_force(request2).await.unwrap();

    // Merge tags
    db.merge_tags(&source_uuid, &target_uuid).await.unwrap();

    // Source should be deleted
    let source_found = db.find_tag_by_normalized_title("work").await.unwrap();
    assert!(source_found.is_none());

    // Target should still exist
    let target_found = db
        .find_tag_by_normalized_title("professional")
        .await
        .unwrap();
    assert!(target_found.is_some());
}

// ========================================================================
// TAG QUERY TESTS
// ========================================================================

#[tokio::test]
#[cfg(feature = "test-utils")]
async fn test_get_all_tags() {
    let db = create_test_database().await.unwrap();

    // Create several tags
    for title in &["work", "personal", "urgent", "later"] {
        let request = CreateTagRequest {
            title: title.to_string(),
            shortcut: None,
            parent_uuid: None,
        };
        db.create_tag_force(request).await.unwrap();
    }

    let all_tags = db.get_all_tags().await.unwrap();
    assert_eq!(all_tags.len(), 4);

    // Should be sorted by title
    for i in 1..all_tags.len() {
        assert!(all_tags[i - 1].title <= all_tags[i].title);
    }
}

#[tokio::test]
#[cfg(feature = "test-utils")]
async fn test_search_tags() {
    let db = create_test_database().await.unwrap();

    // Create several tags
    for title in &["work", "working", "worker", "vacation"] {
        let request = CreateTagRequest {
            title: title.to_string(),
            shortcut: None,
            parent_uuid: None,
        };
        db.create_tag_force(request).await.unwrap();
    }

    // Search for tags containing "work"
    let results = db.search_tags("work").await.unwrap();
    assert_eq!(results.len(), 3); // work, working, worker
}

#[tokio::test]
#[cfg(feature = "test-utils")]
async fn test_get_popular_tags() {
    let db = create_test_database().await.unwrap();

    // Create tags (usage count will be 0 initially)
    for title in &["alpha", "beta", "gamma"] {
        let request = CreateTagRequest {
            title: title.to_string(),
            shortcut: None,
            parent_uuid: None,
        };
        db.create_tag_force(request).await.unwrap();
    }

    let popular = db.get_popular_tags(10).await.unwrap();
    assert_eq!(popular.len(), 3);
}

#[tokio::test]
#[cfg(feature = "test-utils")]
async fn test_get_recent_tags() {
    let db = create_test_database().await.unwrap();

    // Create a tag
    let request = CreateTagRequest {
        title: "work".to_string(),
        shortcut: None,
        parent_uuid: None,
    };
    db.create_tag_force(request).await.unwrap();

    // Recent tags query (will be empty since usedDate is NULL)
    let recent = db.get_recent_tags(10).await.unwrap();
    assert!(recent.is_empty());
}

// ========================================================================
// TAG AUTO-COMPLETION TESTS
// ========================================================================

#[tokio::test]
#[cfg(feature = "test-utils")]
async fn test_get_tag_completions() {
    let db = create_test_database().await.unwrap();

    // Create several tags
    for title in &["work", "working", "worker", "vacation"] {
        let request = CreateTagRequest {
            title: title.to_string(),
            shortcut: None,
            parent_uuid: None,
        };
        db.create_tag_force(request).await.unwrap();
    }

    // Get completions for "wo"
    let completions = db.get_tag_completions("wo", 5).await.unwrap();
    assert!(!completions.is_empty());

    // Should prioritize prefix matches
    assert!(completions[0].tag.title.to_lowercase().starts_with("wo"));
}

// ========================================================================
// TAG ANALYTICS TESTS
// ========================================================================

#[tokio::test]
#[cfg(feature = "test-utils")]
async fn test_find_duplicate_tags() {
    let db = create_test_database().await.unwrap();

    // Create similar tags
    for title in &["work", "Work", "wrk", "working"] {
        let request = CreateTagRequest {
            title: title.to_string(),
            shortcut: None,
            parent_uuid: None,
        };
        db.create_tag_force(request).await.unwrap();
    }

    // Find duplicates with high similarity threshold
    let duplicates = db.find_duplicate_tags(0.85).await.unwrap();
    assert!(!duplicates.is_empty());

    // Should be sorted by similarity (highest first)
    for i in 1..duplicates.len() {
        assert!(duplicates[i - 1].similarity >= duplicates[i].similarity);
    }
}
