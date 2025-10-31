use std::sync::Arc;

use async_trait::async_trait;
use chrono::{TimeZone, Utc};
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use pulsearc_core::classification::ports::{Classifier, TimeEntryRepository};
use pulsearc_core::ClassificationService;
use pulsearc_domain::types::database::ActivitySnapshot;
use pulsearc_domain::{Result as DomainResult, TimeEntry, TimeEntryParams};
use tokio::sync::Mutex;
use uuid::Uuid;

fn sample_snapshots() -> Vec<ActivitySnapshot> {
    (0..5)
        .map(|idx| ActivitySnapshot {
            id: format!("snap-{idx}"),
            timestamp: 1_700_000_000 + idx as i64,
            activity_context_json: r#"{"app":"example"}"#.to_string(),
            detected_activity: "typing".to_string(),
            work_type: Some("modeling".to_string()),
            activity_category: Some("client_work".to_string()),
            primary_app: "Spreadsheet".to_string(),
            processed: true,
            batch_id: None,
            created_at: 1_700_000_000,
            processed_at: None,
            is_idle: false,
            idle_duration_secs: Some(0),
        })
        .collect()
}

fn sample_time_entry() -> TimeEntry {
    let params = TimeEntryParams {
        id: Uuid::from_u128(0x1234),
        start_time: Utc.timestamp_opt(1_700_000_000, 0).single().unwrap(),
        end_time: Some(Utc.timestamp_opt(1_700_000_000 + 1_800, 0).single().unwrap()),
        duration_seconds: Some(1_800),
        description: "Benchmark time entry".to_string(),
        project_id: Some("PROJ-123".to_string()),
        wbs_code: Some("WBS-001".to_string()),
    };

    TimeEntry::new(params)
}

#[derive(Clone)]
struct MockClassifier {
    entry: TimeEntry,
}

impl MockClassifier {
    fn new(entry: TimeEntry) -> Self {
        Self { entry }
    }
}

#[async_trait]
impl Classifier for MockClassifier {
    async fn classify(&self, snapshots: Vec<ActivitySnapshot>) -> DomainResult<TimeEntry> {
        black_box(snapshots);
        Ok(self.entry.clone())
    }
}

#[derive(Default)]
struct MockTimeEntryRepository {
    entries: Arc<Mutex<Vec<TimeEntry>>>,
}

impl MockTimeEntryRepository {
    fn with_entries(entries: Vec<TimeEntry>) -> Self {
        Self { entries: Arc::new(Mutex::new(entries)) }
    }
}

#[async_trait]
impl TimeEntryRepository for MockTimeEntryRepository {
    async fn save_entry(&self, entry: TimeEntry) -> DomainResult<()> {
        let mut guard = self.entries.lock().await;
        guard.clear();
        guard.push(entry);
        Ok(())
    }

    async fn get_entries(
        &self,
        _start: chrono::DateTime<chrono::Utc>,
        _end: chrono::DateTime<chrono::Utc>,
    ) -> DomainResult<Vec<TimeEntry>> {
        let guard = self.entries.lock().await;
        Ok(guard.clone())
    }

    async fn update_entry(&self, entry: TimeEntry) -> DomainResult<()> {
        let mut guard = self.entries.lock().await;
        if let Some(existing) = guard.iter_mut().find(|item| item.id == entry.id) {
            *existing = entry;
        } else {
            guard.push(entry);
        }
        Ok(())
    }

    async fn delete_entry(&self, id: Uuid) -> DomainResult<()> {
        let mut guard = self.entries.lock().await;
        guard.retain(|item| item.id != id);
        Ok(())
    }
}

fn classify_and_save_benchmark(c: &mut Criterion) {
    let snapshots = sample_snapshots();
    let entry = sample_time_entry();

    let classifier = Arc::new(MockClassifier::new(entry.clone()));
    let repository = Arc::new(MockTimeEntryRepository::default());
    let service = Arc::new(ClassificationService::new(classifier, repository));

    let mut group = c.benchmark_group("classification_service");
    group.sample_size(20).measurement_time(std::time::Duration::from_secs(10));

    group.bench_function("classify_and_save", |b| {
        let runtime = tokio::runtime::Runtime::new().expect("runtime");
        let service = Arc::clone(&service);
        let snapshots = snapshots.clone();

        b.iter(|| {
            let service = Arc::clone(&service);
            let iteration_snapshots = snapshots.clone();
            runtime.block_on(async move {
                service.classify_and_save(iteration_snapshots).await.unwrap();
            });
        });
    });

    group.bench_function("get_entries", |b| {
        let runtime = tokio::runtime::Runtime::new().expect("runtime");
        let service = Arc::new(ClassificationService::new(
            Arc::new(MockClassifier::new(entry.clone())),
            Arc::new(MockTimeEntryRepository::with_entries(vec![entry.clone(); 64])),
        ));
        let start = Utc.timestamp_opt(1_700_000_000, 0).single().unwrap();
        let end = Utc.timestamp_opt(1_700_086_400, 0).single().unwrap();

        b.iter(|| {
            let service = Arc::clone(&service);
            runtime.block_on(async move {
                service.get_entries(start, end).await.unwrap();
            });
        });
    });

    group.finish();
}

criterion_group!(core_benchmarks, classify_and_save_benchmark);
criterion_main!(core_benchmarks);
