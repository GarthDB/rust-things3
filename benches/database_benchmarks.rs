use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use things3_core::test_utils::create_test_database;
use things3_core::{CreateTaskRequest, ThingsDatabase};
use tokio::runtime::Runtime;

fn create_test_db_with_data(task_count: usize) -> (tempfile::NamedTempFile, ThingsDatabase) {
    let rt = Runtime::new().unwrap();
    let temp_file = tempfile::NamedTempFile::new().unwrap();
    let db_path = temp_file.path();

    rt.block_on(async {
        create_test_database(db_path).await.unwrap();
        let db = ThingsDatabase::new(db_path).await.unwrap();

        // Insert test tasks
        for i in 0..task_count {
            let request = CreateTaskRequest {
                title: format!("Benchmark Task {}", i),
                notes: Some(format!("Notes for benchmark task {}", i)),
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

        (temp_file, db)
    })
}

fn bench_get_inbox(c: &mut Criterion) {
    let mut group = c.benchmark_group("get_inbox");
    let rt = Runtime::new().unwrap();

    for size in [10, 50, 100, 500].iter() {
        group.throughput(Throughput::Elements(*size as u64));

        let (_temp, db) = create_test_db_with_data(*size);

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            b.to_async(&rt).iter(|| async {
                let tasks = db.get_inbox(Some(*size)).await.unwrap();
                black_box(tasks);
            });
        });
    }

    group.finish();
}

fn bench_get_today(c: &mut Criterion) {
    let mut group = c.benchmark_group("get_today");
    let rt = Runtime::new().unwrap();

    for size in [10, 50, 100, 500].iter() {
        group.throughput(Throughput::Elements(*size as u64));

        let (_temp, db) = create_test_db_with_data(*size);

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            b.to_async(&rt).iter(|| async {
                let tasks = db.get_today(Some(*size)).await.unwrap();
                black_box(tasks);
            });
        });
    }

    group.finish();
}

fn bench_search_tasks(c: &mut Criterion) {
    let mut group = c.benchmark_group("search_tasks");
    let rt = Runtime::new().unwrap();

    for size in [100, 500, 1000].iter() {
        group.throughput(Throughput::Elements(*size as u64));

        let (_temp, db) = create_test_db_with_data(*size);

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            b.to_async(&rt).iter(|| async {
                let tasks = db.search_tasks("Test").await.unwrap();
                black_box(tasks);
            });
        });
    }

    group.finish();
}

fn bench_get_projects(c: &mut Criterion) {
    let mut group = c.benchmark_group("get_projects");
    let rt = Runtime::new().unwrap();

    for size in [10, 50, 100].iter() {
        let (_temp, db) = create_test_db_with_data(*size);

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            b.to_async(&rt).iter(|| async {
                let projects = db.get_projects(None).await.unwrap();
                black_box(projects);
            });
        });
    }

    group.finish();
}

fn bench_get_areas(c: &mut Criterion) {
    let mut group = c.benchmark_group("get_areas");
    let rt = Runtime::new().unwrap();

    for size in [5, 10, 20].iter() {
        let (_temp, db) = create_test_db_with_data(*size);

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            b.to_async(&rt).iter(|| async {
                let areas = db.get_areas().await.unwrap();
                black_box(areas);
            });
        });
    }

    group.finish();
}

fn bench_get_stats(c: &mut Criterion) {
    let mut group = c.benchmark_group("get_stats");
    let rt = Runtime::new().unwrap();

    for size in [100, 500, 1000].iter() {
        let (_temp, db) = create_test_db_with_data(*size);

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            b.to_async(&rt).iter(|| async {
                let stats = db.get_stats().await.unwrap();
                black_box(stats);
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_get_inbox,
    bench_get_today,
    bench_search_tasks,
    bench_get_projects,
    bench_get_areas,
    bench_get_stats
);
criterion_main!(benches);

