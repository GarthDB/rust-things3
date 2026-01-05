use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use things3_core::test_utils::create_test_database;
use things3_core::{
    BulkCompleteRequest, BulkDeleteRequest, BulkMoveRequest, CreateTaskRequest, ThingsDatabase,
};
use tokio::runtime::Runtime;
use uuid::Uuid;

fn create_test_db_with_tasks(
    task_count: usize,
) -> (tempfile::NamedTempFile, ThingsDatabase, Vec<Uuid>) {
    let rt = Runtime::new().unwrap();
    let temp_file = tempfile::NamedTempFile::new().unwrap();
    let db_path = temp_file.path().to_path_buf();

    let (db, task_uuids) = rt.block_on(async {
        create_test_database(&db_path).await.unwrap();
        let db = ThingsDatabase::new(&db_path).await.unwrap();

        let mut task_uuids = Vec::new();
        for i in 0..task_count {
            let request = CreateTaskRequest {
                title: format!("Bulk Test Task {}", i),
                notes: Some(format!("Task for bulk operations {}", i)),
                deadline: None,
                start_date: None,
                project_uuid: None,
                area_uuid: None,
                parent_uuid: None,
                tags: None,
                task_type: None,
                status: None,
            };
            let uuid = db.create_task(request).await.unwrap();
            task_uuids.push(uuid);
        }

        (db, task_uuids)
    });

    (temp_file, db, task_uuids)
}

fn bench_bulk_move(c: &mut Criterion) {
    let mut group = c.benchmark_group("bulk_move");
    group.warm_up_time(std::time::Duration::from_secs(3));
    group.measurement_time(std::time::Duration::from_secs(10));
    let rt = Runtime::new().unwrap();

    for size in [10, 50, 100, 500, 1000].iter() {
        group.throughput(Throughput::Elements(*size as u64));

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.iter_batched(
                || {
                    // Setup: create fresh database with tasks
                    let (_temp, db, task_uuids) = create_test_db_with_tasks(size);
                    (db, task_uuids)
                },
                |(db, task_uuids)| {
                    // Benchmark: move tasks to inbox (clear project/area)
                    rt.block_on(async {
                        let request = BulkMoveRequest {
                            task_uuids: task_uuids.clone(),
                            project_uuid: None,
                            area_uuid: None,
                        };
                        let result = db.bulk_move(request).await.unwrap();
                        black_box(result);
                    });
                },
                criterion::BatchSize::SmallInput,
            );
        });
    }

    group.finish();
}

fn bench_bulk_complete(c: &mut Criterion) {
    let mut group = c.benchmark_group("bulk_complete");
    group.warm_up_time(std::time::Duration::from_secs(3));
    group.measurement_time(std::time::Duration::from_secs(10));
    let rt = Runtime::new().unwrap();

    for size in [10, 50, 100, 500, 1000].iter() {
        group.throughput(Throughput::Elements(*size as u64));

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.iter_batched(
                || {
                    let (_temp, db, task_uuids) = create_test_db_with_tasks(size);
                    (db, task_uuids)
                },
                |(db, task_uuids)| {
                    rt.block_on(async {
                        let request = BulkCompleteRequest { task_uuids };
                        let result = db.bulk_complete(request).await.unwrap();
                        black_box(result);
                    });
                },
                criterion::BatchSize::SmallInput,
            );
        });
    }

    group.finish();
}

fn bench_bulk_delete(c: &mut Criterion) {
    let mut group = c.benchmark_group("bulk_delete");
    group.warm_up_time(std::time::Duration::from_secs(3));
    group.measurement_time(std::time::Duration::from_secs(10));
    let rt = Runtime::new().unwrap();

    for size in [10, 50, 100, 500, 1000].iter() {
        group.throughput(Throughput::Elements(*size as u64));

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.iter_batched(
                || {
                    let (_temp, db, task_uuids) = create_test_db_with_tasks(size);
                    (db, task_uuids)
                },
                |(db, task_uuids)| {
                    rt.block_on(async {
                        let request = BulkDeleteRequest { task_uuids };
                        let result = db.bulk_delete(request).await.unwrap();
                        black_box(result);
                    });
                },
                criterion::BatchSize::SmallInput,
            );
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_bulk_move,
    bench_bulk_complete,
    bench_bulk_delete
);
criterion_main!(benches);
