use anyhow::Result;
use std::sync::Arc;

use super::stats::CachePreloader;
use super::ThingsCache;

/// Default [`CachePreloader`] with a small set of hardcoded heuristics over
/// the existing top-level cache keys.
///
/// Holds a [`Weak`] reference to the cache to avoid the obvious
/// `Arc<ThingsCache>` ↔ `Arc<dyn CachePreloader>` reference cycle. Once the
/// last strong reference to the cache is dropped, [`CachePreloader::warm`]
/// becomes a no-op.
///
/// Heuristics:
/// - Accessing `inbox:all` predicts `today:all` (priority 8).
/// - Accessing `today:all` predicts `inbox:all` (priority 10).
/// - Accessing `areas:all` predicts `projects:all` (priority 7).
///
/// Other keys produce no predictions. Future preloaders (per-project tasks,
/// search-history-driven) plug in via the same trait.
///
/// # Warm-loop behaviour
///
/// The `inbox:all` ↔ `today:all` pair is mutually predictive, which would
/// ordinarily create a perpetual warming loop. [`ThingsCache::notify_preloader`]
/// guards against this: a predicted key is only enqueued when it is *not*
/// already present in the cache. Once both keys are warm, no further
/// enqueuing occurs until one of them expires or is invalidated.
pub struct DefaultPreloader {
    cache: std::sync::Weak<ThingsCache>,
    db: Arc<crate::database::ThingsDatabase>,
}

impl DefaultPreloader {
    /// Construct a preloader that holds a [`Weak`] handle to `cache` and a
    /// strong handle to `db`. Wrap in [`Arc`] before registering with
    /// [`ThingsCache::set_preloader`].
    #[must_use]
    pub fn new(cache: &Arc<ThingsCache>, db: Arc<crate::database::ThingsDatabase>) -> Arc<Self> {
        Arc::new(Self {
            cache: Arc::downgrade(cache),
            db,
        })
    }
}

impl CachePreloader for DefaultPreloader {
    fn predict(&self, accessed_key: &str) -> Vec<(String, u32)> {
        match accessed_key {
            "inbox:all" => vec![("today:all".to_string(), 8)],
            "today:all" => vec![("inbox:all".to_string(), 10)],
            "areas:all" => vec![("projects:all".to_string(), 7)],
            _ => vec![],
        }
    }

    fn warm(&self, key: &str) {
        let Some(cache) = self.cache.upgrade() else {
            return;
        };
        let db = Arc::clone(&self.db);
        let key = key.to_string();
        tokio::spawn(async move {
            let result: Result<()> = match key.as_str() {
                "inbox:all" => cache
                    .get_tasks(&key, || async {
                        db.get_inbox(None).await.map_err(anyhow::Error::from)
                    })
                    .await
                    .map(|_| ()),
                "today:all" => cache
                    .get_tasks(&key, || async {
                        db.get_today(None).await.map_err(anyhow::Error::from)
                    })
                    .await
                    .map(|_| ()),
                "areas:all" => cache
                    .get_areas(&key, || async {
                        db.get_areas().await.map_err(anyhow::Error::from)
                    })
                    .await
                    .map(|_| ()),
                "projects:all" => cache
                    .get_projects(&key, || async {
                        db.get_projects(None).await.map_err(anyhow::Error::from)
                    })
                    .await
                    .map(|_| ()),
                _ => Ok(()),
            };
            if let Err(e) = result {
                tracing::warn!("DefaultPreloader::warm({key}) failed: {e}");
            }
        });
    }
}

/// Cache key generators
pub mod keys {
    /// Generate cache key for inbox tasks
    #[must_use]
    pub fn inbox(limit: Option<usize>) -> String {
        format!(
            "inbox:{}",
            limit.map_or("all".to_string(), |l| l.to_string())
        )
    }

    /// Generate cache key for today's tasks
    #[must_use]
    pub fn today(limit: Option<usize>) -> String {
        format!(
            "today:{}",
            limit.map_or("all".to_string(), |l| l.to_string())
        )
    }

    /// Generate cache key for projects
    #[must_use]
    pub fn projects(area_uuid: Option<&str>) -> String {
        format!("projects:{}", area_uuid.unwrap_or("all"))
    }

    /// Generate cache key for areas
    #[must_use]
    pub fn areas() -> String {
        "areas:all".to_string()
    }

    /// Generate cache key for search results
    #[must_use]
    pub fn search(query: &str, limit: Option<usize>) -> String {
        format!(
            "search:{}:{}",
            query,
            limit.map_or("all".to_string(), |l| l.to_string())
        )
    }
}

