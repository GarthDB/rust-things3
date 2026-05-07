use crate::models::{Area, Project, Task, ThingsId};
use anyhow::Result;
use moka::future::Cache;

use super::config::{CacheConfig, CacheDependency};
use super::stats::{CachedData, CachePreloader, CacheStats};
use super::ThingsCache;

impl ThingsCache {
    /// Create a new cache with the given configuration
    #[must_use]
    pub fn new(config: &CacheConfig) -> Self {
        let tasks = Cache::builder()
            .max_capacity(config.max_capacity)
            .time_to_live(config.ttl)
            .time_to_idle(config.tti)
            .build();

        let projects = Cache::builder()
            .max_capacity(config.max_capacity)
            .time_to_live(config.ttl)
            .time_to_idle(config.tti)
            .build();

        let areas = Cache::builder()
            .max_capacity(config.max_capacity)
            .time_to_live(config.ttl)
            .time_to_idle(config.tti)
            .build();

        let search_results = Cache::builder()
            .max_capacity(config.max_capacity)
            .time_to_live(config.ttl)
            .time_to_idle(config.tti)
            .build();

        let mut cache = Self {
            tasks,
            projects,
            areas,
            search_results,
            stats: std::sync::Arc::new(parking_lot::RwLock::new(CacheStats::default())),
            config: config.clone(),
            warming_entries: std::sync::Arc::new(parking_lot::RwLock::new(
                std::collections::HashMap::new(),
            )),
            preloader: std::sync::Arc::new(parking_lot::RwLock::new(None)),
            warming_task: None,
        };

        // Start cache warming task if enabled
        if config.enable_cache_warming {
            cache.start_cache_warming();
        }

        cache
    }

    /// Create a new cache with default configuration
    #[must_use]
    pub fn new_default() -> Self {
        Self::new(&CacheConfig::default())
    }

    /// Get tasks from cache or fetch if not cached
    ///
    /// # Errors
    ///
    /// Returns an error if the fetcher function fails.
    pub async fn get_tasks<F, Fut>(&self, key: &str, fetcher: F) -> Result<Vec<Task>>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<Vec<Task>>>,
    {
        if let Some(mut cached) = self.tasks.get(key).await {
            if !cached.is_expired() && !cached.is_idle(self.config.tti) {
                cached.record_access();
                self.record_hit();

                // Add to warming if frequently accessed
                if cached.access_count > 3 {
                    self.add_to_warming(key.to_string(), cached.warming_priority + 1);
                }

                self.notify_preloader(key);
                return Ok(cached.data);
            }
        }

        self.record_miss();
        let data = fetcher().await?;

        // Create dependencies for intelligent invalidation
        let dependencies = Self::create_task_dependencies(&data);
        let mut cached_data =
            CachedData::new_with_dependencies(data.clone(), self.config.ttl, dependencies);

        // Set initial warming priority based on key type
        let priority = if key.starts_with("inbox:") {
            10
        } else if key.starts_with("today:") {
            8
        } else {
            5
        };
        cached_data.update_warming_priority(priority);

        self.tasks.insert(key.to_string(), cached_data).await;
        self.notify_preloader(key);
        Ok(data)
    }

    /// Get projects from cache or fetch if not cached
    ///
    /// # Errors
    ///
    /// Returns an error if the fetcher function fails.
    pub async fn get_projects<F, Fut>(&self, key: &str, fetcher: F) -> Result<Vec<Project>>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<Vec<Project>>>,
    {
        if let Some(mut cached) = self.projects.get(key).await {
            if !cached.is_expired() && !cached.is_idle(self.config.tti) {
                cached.record_access();
                self.record_hit();

                // Add to warming if frequently accessed
                if cached.access_count > 3 {
                    self.add_to_warming(key.to_string(), cached.warming_priority + 1);
                }

                self.notify_preloader(key);
                return Ok(cached.data);
            }
        }

        self.record_miss();
        let data = fetcher().await?;

        // Create dependencies for intelligent invalidation
        let dependencies = Self::create_project_dependencies(&data);
        let mut cached_data =
            CachedData::new_with_dependencies(data.clone(), self.config.ttl, dependencies);

        // Set initial warming priority
        let priority = if key.starts_with("projects:") { 7 } else { 5 };
        cached_data.update_warming_priority(priority);

        self.projects.insert(key.to_string(), cached_data).await;
        self.notify_preloader(key);
        Ok(data)
    }

    /// Get areas from cache or fetch if not cached
    ///
    /// # Errors
    ///
    /// Returns an error if the fetcher function fails.
    pub async fn get_areas<F, Fut>(&self, key: &str, fetcher: F) -> Result<Vec<Area>>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<Vec<Area>>>,
    {
        if let Some(mut cached) = self.areas.get(key).await {
            if !cached.is_expired() && !cached.is_idle(self.config.tti) {
                cached.record_access();
                self.record_hit();

                // Add to warming if frequently accessed
                if cached.access_count > 3 {
                    self.add_to_warming(key.to_string(), cached.warming_priority + 1);
                }

                self.notify_preloader(key);
                return Ok(cached.data);
            }
        }

        self.record_miss();
        let data = fetcher().await?;

        // Create dependencies for intelligent invalidation
        let dependencies = Self::create_area_dependencies(&data);
        let mut cached_data =
            CachedData::new_with_dependencies(data.clone(), self.config.ttl, dependencies);

        // Set initial warming priority
        let priority = if key.starts_with("areas:") { 6 } else { 5 };
        cached_data.update_warming_priority(priority);

        self.areas.insert(key.to_string(), cached_data).await;
        self.notify_preloader(key);
        Ok(data)
    }

    /// Get search results from cache or fetch if not cached
    ///
    /// # Errors
    ///
    /// Returns an error if the fetcher function fails.
    pub async fn get_search_results<F, Fut>(&self, key: &str, fetcher: F) -> Result<Vec<Task>>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<Vec<Task>>>,
    {
        if let Some(mut cached) = self.search_results.get(key).await {
            if !cached.is_expired() && !cached.is_idle(self.config.tti) {
                cached.record_access();
                self.record_hit();

                // Add to warming if frequently accessed
                if cached.access_count > 3 {
                    self.add_to_warming(key.to_string(), cached.warming_priority + 1);
                }

                self.notify_preloader(key);
                return Ok(cached.data);
            }
        }

        self.record_miss();
        let data = fetcher().await?;

        // Create dependencies for intelligent invalidation
        let dependencies = Self::create_task_dependencies(&data);
        let mut cached_data =
            CachedData::new_with_dependencies(data.clone(), self.config.ttl, dependencies);

        // Set initial warming priority for search results
        let priority = if key.starts_with("search:") { 4 } else { 3 };
        cached_data.update_warming_priority(priority);

        self.search_results
            .insert(key.to_string(), cached_data)
            .await;
        self.notify_preloader(key);
        Ok(data)
    }

    /// Invalidate all caches
    pub fn invalidate_all(&self) {
        self.tasks.invalidate_all();
        self.projects.invalidate_all();
        self.areas.invalidate_all();
        self.search_results.invalidate_all();
    }

    /// Invalidate specific cache entry
    pub async fn invalidate(&self, key: &str) {
        self.tasks.remove(key).await;
        self.projects.remove(key).await;
        self.areas.remove(key).await;
        self.search_results.remove(key).await;
    }

    /// Get cache statistics
    #[must_use]
    pub fn get_stats(&self) -> CacheStats {
        let mut stats = self.stats.read().clone();
        stats.entries = self.tasks.entry_count()
            + self.projects.entry_count()
            + self.areas.entry_count()
            + self.search_results.entry_count();
        stats.calculate_hit_rate();
        stats
    }

    /// Reset cache statistics
    pub fn reset_stats(&self) {
        let mut stats = self.stats.write();
        *stats = CacheStats::default();
    }

    /// Record a cache hit
    fn record_hit(&self) {
        let mut stats = self.stats.write();
        stats.hits += 1;
    }

    /// Record a cache miss
    fn record_miss(&self) {
        let mut stats = self.stats.write();
        stats.misses += 1;
    }

    /// Create dependencies for task data
    fn create_task_dependencies(tasks: &[Task]) -> Vec<CacheDependency> {
        let mut dependencies = Vec::new();

        // Add dependencies for each task
        for task in tasks {
            dependencies.push(CacheDependency {
                entity_type: "task".to_string(),
                entity_id: Some(task.uuid.clone()),
                invalidating_operations: vec![
                    "task_updated".to_string(),
                    "task_deleted".to_string(),
                    "task_completed".to_string(),
                ],
            });

            // Add project dependency if task belongs to a project
            if let Some(project_uuid) = &task.project_uuid {
                dependencies.push(CacheDependency {
                    entity_type: "project".to_string(),
                    entity_id: Some(project_uuid.clone()),
                    invalidating_operations: vec![
                        "project_updated".to_string(),
                        "project_deleted".to_string(),
                    ],
                });
            }

            // Add area dependency if task belongs to an area
            if let Some(area_uuid) = &task.area_uuid {
                dependencies.push(CacheDependency {
                    entity_type: "area".to_string(),
                    entity_id: Some(area_uuid.clone()),
                    invalidating_operations: vec![
                        "area_updated".to_string(),
                        "area_deleted".to_string(),
                    ],
                });
            }
        }

        dependencies
    }

    /// Create dependencies for project data
    fn create_project_dependencies(projects: &[Project]) -> Vec<CacheDependency> {
        let mut dependencies = Vec::new();

        for project in projects {
            dependencies.push(CacheDependency {
                entity_type: "project".to_string(),
                entity_id: Some(project.uuid.clone()),
                invalidating_operations: vec![
                    "project_updated".to_string(),
                    "project_deleted".to_string(),
                ],
            });

            if let Some(area_uuid) = &project.area_uuid {
                dependencies.push(CacheDependency {
                    entity_type: "area".to_string(),
                    entity_id: Some(area_uuid.clone()),
                    invalidating_operations: vec![
                        "area_updated".to_string(),
                        "area_deleted".to_string(),
                    ],
                });
            }
        }

        dependencies
    }

    /// Create dependencies for area data
    fn create_area_dependencies(areas: &[Area]) -> Vec<CacheDependency> {
        let mut dependencies = Vec::new();

        for area in areas {
            dependencies.push(CacheDependency {
                entity_type: "area".to_string(),
                entity_id: Some(area.uuid.clone()),
                invalidating_operations: vec![
                    "area_updated".to_string(),
                    "area_deleted".to_string(),
                ],
            });
        }

        dependencies
    }

    /// Start cache warming background task.
    ///
    /// Each tick, drains the top-priority queued keys and dispatches each to
    /// the registered [`CachePreloader`] (if any). Keys are removed from the
    /// queue after dispatch — the preloader's own `predict` calls re-add them
    /// later if they remain hot.
    pub(super) fn start_cache_warming(&mut self) {
        let warming_entries = std::sync::Arc::clone(&self.warming_entries);
        let preloader = std::sync::Arc::clone(&self.preloader);
        let stats = std::sync::Arc::clone(&self.stats);
        let warming_interval = self.config.warming_interval;
        let max_entries = self.config.max_warming_entries;

        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(warming_interval);
            loop {
                interval.tick().await;

                let entries_to_warm = {
                    let entries = warming_entries.read();
                    let mut sorted_entries: Vec<_> = entries.iter().collect();
                    sorted_entries.sort_by(|a, b| b.1.cmp(a.1));
                    sorted_entries
                        .into_iter()
                        .take(max_entries)
                        .map(|(key, _)| key.clone())
                        .collect::<Vec<_>>()
                };

                if entries_to_warm.is_empty() {
                    continue;
                }

                let p_snapshot = preloader.read().clone();
                if let Some(p) = p_snapshot {
                    for key in &entries_to_warm {
                        p.warm(key);
                    }
                    let mut s = stats.write();
                    s.warming_runs += 1;
                    s.warmed_keys += entries_to_warm.len() as u64;
                } else {
                    tracing::debug!(
                        "Cache warming {} entries (no preloader registered)",
                        entries_to_warm.len()
                    );
                }

                let mut entries = warming_entries.write();
                for key in &entries_to_warm {
                    entries.remove(key);
                }
            }
        });

        self.warming_task = Some(handle);
    }

    /// Register a preloader. Replaces any previously-registered preloader.
    ///
    /// The preloader's `predict` will be invoked after every `get_*` call,
    /// and `warm` will be invoked by the warming-loop tick for queued keys.
    pub fn set_preloader(&self, preloader: std::sync::Arc<dyn CachePreloader>) {
        *self.preloader.write() = Some(preloader);
    }

    /// Remove the registered preloader. Subsequent `get_*` calls and warming
    /// ticks become no-ops with respect to predictive preloading.
    pub fn clear_preloader(&self) {
        *self.preloader.write() = None;
    }

    /// Returns `true` if `key` is present in any of the four underlying caches.
    fn contains_cached_key(&self, key: &str) -> bool {
        self.tasks.contains_key(key)
            || self.projects.contains_key(key)
            || self.areas.contains_key(key)
            || self.search_results.contains_key(key)
    }

    /// Snapshot the registered preloader and call its `predict`, pushing any
    /// returned `(key, priority)` pairs into `warming_entries`.
    /// Keys already present in the cache are skipped — this prevents a
    /// self-reinforcing loop where warming a key triggers predict on its
    /// counterpart, which re-enqueues the original key indefinitely.
    fn notify_preloader(&self, accessed_key: &str) {
        let p_snapshot = self.preloader.read().clone();
        let Some(p) = p_snapshot else { return };
        for (k, prio) in p.predict(accessed_key) {
            if !self.contains_cached_key(&k) {
                self.add_to_warming(k, prio);
            }
        }
    }

    /// Add entry to cache warming list
    pub fn add_to_warming(&self, key: String, priority: u32) {
        let mut entries = self.warming_entries.write();
        entries.insert(key, priority);
    }

    /// Remove entry from cache warming list
    pub fn remove_from_warming(&self, key: &str) {
        let mut entries = self.warming_entries.write();
        entries.remove(key);
    }

    /// Selectively invalidate cache entries whose dependencies match
    /// `(entity_type, entity_id)`. Returns the number of keys submitted for
    /// eviction (moka eviction may complete asynchronously).
    ///
    /// `entity_id == None` is a wildcard that matches any cached entry
    /// depending on `entity_type`. Entries that do not depend on the mutated
    /// entity are left untouched.
    pub async fn invalidate_by_entity(
        &self,
        entity_type: &str,
        entity_id: Option<&ThingsId>,
    ) -> usize {
        let (task_keys, project_keys, area_keys, search_keys) = {
            let pred = |dep: &CacheDependency| dep.matches(entity_type, entity_id);
            (
                collect_matching_keys(&self.tasks, &pred),
                collect_matching_keys(&self.projects, &pred),
                collect_matching_keys(&self.areas, &pred),
                collect_matching_keys(&self.search_results, &pred),
            )
        };
        let removed = evict_keys(&self.tasks, &task_keys).await
            + evict_keys(&self.projects, &project_keys).await
            + evict_keys(&self.areas, &area_keys).await
            + evict_keys(&self.search_results, &search_keys).await;

        tracing::debug!(
            "Invalidated {} cache entries depending on {} {:?}",
            removed,
            entity_type,
            entity_id
        );
        removed
    }

    /// Selectively invalidate cache entries whose dependencies list `operation`
    /// among their invalidating operations. Returns the number of keys submitted
    /// for eviction (moka eviction may complete asynchronously).
    pub async fn invalidate_by_operation(&self, operation: &str) -> usize {
        let (task_keys, project_keys, area_keys, search_keys) = {
            let pred = |dep: &CacheDependency| dep.matches_operation(operation);
            (
                collect_matching_keys(&self.tasks, &pred),
                collect_matching_keys(&self.projects, &pred),
                collect_matching_keys(&self.areas, &pred),
                collect_matching_keys(&self.search_results, &pred),
            )
        };
        let removed = evict_keys(&self.tasks, &task_keys).await
            + evict_keys(&self.projects, &project_keys).await
            + evict_keys(&self.areas, &area_keys).await
            + evict_keys(&self.search_results, &search_keys).await;

        tracing::debug!(
            "Invalidated {} cache entries due to operation {}",
            removed,
            operation
        );
        removed
    }

    /// Get cache warming statistics
    #[must_use]
    pub fn get_warming_stats(&self) -> (usize, u32) {
        let entries = self.warming_entries.read();
        let count = entries.len();
        let max_priority = entries.values().max().copied().unwrap_or(0);
        (count, max_priority)
    }

    /// Stop cache warming
    pub fn stop_cache_warming(&mut self) {
        if let Some(handle) = self.warming_task.take() {
            handle.abort();
        }
    }
}

impl Default for ThingsCache {
    fn default() -> Self {
        Self::new_default()
    }
}

/// Walk a moka cache synchronously and collect keys whose dependency list
/// satisfies `pred`. Split from [`evict_keys`] so the (non-`Send`) predicate is
/// dropped before any `.await`, keeping the surrounding async fn `Send`.
fn collect_matching_keys<V>(
    cache: &moka::future::Cache<String, CachedData<V>>,
    pred: &dyn Fn(&CacheDependency) -> bool,
) -> Vec<String>
where
    V: Clone + Send + Sync + 'static,
{
    cache
        .iter()
        .filter_map(|(k, v)| {
            if v.dependencies.iter().any(pred) {
                Some((*k).clone())
            } else {
                None
            }
        })
        .collect()
}

/// Evict the given keys from a moka cache.
///
/// Returns the number of keys submitted for eviction. Moka's `invalidate` is
/// async but the actual removal may lag slightly; callers that need to observe
/// the post-eviction state should `await` a short yield or sleep.
async fn evict_keys<V>(
    cache: &moka::future::Cache<String, CachedData<V>>,
    keys: &[String],
) -> usize
where
    V: Clone + Send + Sync + 'static,
{
    for k in keys {
        cache.invalidate(k).await;
    }
    keys.len()
}
