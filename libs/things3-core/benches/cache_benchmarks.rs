use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use things3_core::test_utils::create_test_database;
use things3_core::{CreateTaskRequest, ThingsDatabase};
use tokio::runtime::Runtime;

fn create_test_db_with_data(task_count: usize) -> (tempfile::NamedTempFile, ThingsDatabase) {
    let rt = Runtime::new().unwrap();
    let temp_file = tempfile::NamedTempFile::new().unwrap();
    let db_path = temp_file.path().to_path_buf();

    let db = rt.block_on(async {
        create_test_database(&db_path).await.unwrap();
        let db = ThingsDatabase::new(&db_path).await.unwrap();

        // Insert test tasks
        for i in 0..task_count {
            let request = CreateTaskRequest {
                title: format!("Cache Test Task {}", i),
                notes: Some(format!("Task for cache benchmarking {}", i)),
                deadline: None,
                start_date: None,
                project_uuid: None,
                area_uuid: None,
                parent_uuid: None,
                tags: None,
                task_type: None,
                status: None,
            };
            db.create_task(request).await.unwrap();
        }

        db
    });

    (temp_file, db)
}

fn bench_cache_cold_read(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache_cold_read");
    let rt = Runtime::new().unwrap();

    for size in [100, 500, 1000].iter() {
        group.throughput(Throughput::Elements(*size as u64));

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.iter_batched(
                || {
                    // Setup: create fresh database (cold cache)
                    create_test_db_with_data(size)
                },
                |(_temp, db)| {
                    // Benchmark: first read (cache miss)
                    rt.block_on(async {
                        let tasks = db.get_inbox(Some(size)).await.unwrap();
                        black_box(tasks);
                    });
                },
                criterion::BatchSize::SmallInput,
            );
        });
    }

    group.finish();
}

fn bench_cache_warm_read(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache_warm_read");
    let rt = Runtime::new().unwrap();

    for size in [100, 500, 1000].iter() {
        group.throughput(Throughput::Elements(*size as u64));

        let (_temp, db) = create_test_db_with_data(*size);

        // Warm up the cache
        rt.block_on(async {
            let _ = db.get_inbox(Some(*size)).await.unwrap();
        });

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.to_async(&rt).iter(|| async {
                // Benchmark: subsequent reads (cache hit)
                let tasks = db.get_inbox(Some(size)).await.unwrap();
                black_box(tasks);
            });
        });
    }

    group.finish();
}

fn bench_cache_hit_rate(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache_hit_rate");
    let rt = Runtime::new().unwrap();

    let (_temp, db) = create_test_db_with_data(1000);

    // Warm up cache with initial reads
    rt.block_on(async {
        let _ = db.get_inbox(Some(100)).await.unwrap();
        let _ = db.get_today(Some(100)).await.unwrap();
        let _ = db.search_tasks("Test").await.unwrap();
    });

    group.bench_function("mixed_queries", |b| {
        b.to_async(&rt).iter(|| async {
            // Mix of cached and uncached queries
            let inbox = db.get_inbox(Some(50)).await.unwrap();
            let today = db.get_today(Some(50)).await.unwrap();
            let search = db.search_tasks("Cache").await.unwrap();
            black_box((inbox, today, search));
        });
    });

    group.finish();
}

fn bench_cache_eviction(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache_eviction");
    let rt = Runtime::new().unwrap();

    let (_temp, db) = create_test_db_with_data(5000);

    group.bench_function("large_dataset_queries", |b| {
        b.to_async(&rt).iter(|| async {
            // Query different subsets to trigger cache eviction
            for i in 0..10 {
                let query = format!("Task {}", i * 100);
                let results = db.search_tasks(&query).await.unwrap();
                black_box(results);
            }
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_cache_cold_read,
    bench_cache_warm_read,
    bench_cache_hit_rate,
    bench_cache_eviction
);
criterion_main!(benches);
