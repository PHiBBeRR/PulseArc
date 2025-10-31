use std::convert::Infallible;
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use std::{env, panic};

use chrono::Utc;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Response, Server, StatusCode};
use infra_baselines::init_test_encryption_key;
use legacy_shim::mdm::{MdmClient, MdmConfig, PolicySetting, PolicyValue};
#[cfg(target_os = "macos")]
use legacy_shim::{check_ax_permission, MacOsActivityProvider};
use legacy_shim::{DbManager as LegacyDbManager, HttpClient};
use rusqlite::params;
use rustls::{Certificate as RustlsCertificate, PrivateKey, ServerConfig};
use rustls_pemfile::{certs as load_pem_certs, pkcs8_private_keys, rsa_private_keys};
use tempfile::TempDir;
use tokio::net::TcpListener;
use tokio::runtime::Runtime;
use tokio::sync::oneshot;
use tokio_rustls::TlsAcceptor;
use uuid::Uuid;

#[cfg(target_os = "macos")]
mod mac_ax;

type LegacyDbManagerInitResult = Result<(TempDir, Arc<LegacyDbManager>), String>;

#[derive(Clone)]
struct ActivitySnapshot {
    id: String,
    timestamp: i64,
    activity_context_json: String,
    detected_activity: String,
    work_type: Option<String>,
    activity_category: Option<String>,
    primary_app: String,
    processed: bool,
    batch_id: Option<String>,
    created_at: i64,
    processed_at: Option<i64>,
    is_idle: bool,
    idle_duration_secs: Option<i32>,
}

fn resolve_mdm_cert_dir() -> Option<PathBuf> {
    if let Ok(custom) = env::var("PULSARC_MDM_CERT_DIR") {
        let path = PathBuf::from(custom);
        if path.exists() {
            return Some(path);
        }
    }

    let cwd = PathBuf::from(".mdm-certs");
    if cwd.exists() {
        return Some(cwd);
    }

    if let Ok(manifest_dir) = env::var("CARGO_MANIFEST_DIR") {
        let candidate = Path::new(&manifest_dir).join("..").join("..").join(".mdm-certs");
        if candidate.exists() {
            return Some(candidate);
        }
    }

    None
}

fn benchmark_legacy_db_manager(c: &mut Criterion) {
    init_test_encryption_key();

    // ---------------------------------------------------------------------
    // Single snapshot insert benchmark
    // ---------------------------------------------------------------------
    let (_single_dir, single_manager) = create_db_manager().expect("legacy DbManager init failed");
    let mut db_group = c.benchmark_group("legacy_db_manager");
    db_group.sample_size(200);
    db_group.warm_up_time(Duration::from_secs(5));
    db_group.measurement_time(Duration::from_secs(10));

    db_group.bench_function("save_snapshot_single", |b| {
        b.iter(|| {
            let snapshot = new_snapshot(Utc::now().timestamp());
            let conn = single_manager.get_connection().expect("failed to get pooled connection");

            conn.execute(
                "INSERT INTO activity_snapshots (
                    id, timestamp, activity_context_json, detected_activity,
                    work_type, activity_category, primary_app, processed,
                    batch_id, created_at, processed_at, is_idle, idle_duration_secs
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
                params![
                    snapshot.id,
                    snapshot.timestamp,
                    snapshot.activity_context_json,
                    snapshot.detected_activity,
                    snapshot.work_type,
                    snapshot.activity_category,
                    snapshot.primary_app,
                    snapshot.processed as i32,
                    snapshot.batch_id,
                    snapshot.created_at,
                    snapshot.processed_at,
                    snapshot.is_idle as i32,
                    snapshot.idle_duration_secs
                ],
            )
            .expect("failed to insert snapshot");
        });
    });

    // ---------------------------------------------------------------------
    // Time-range query benchmark (1 day window with 100 snapshots)
    // ---------------------------------------------------------------------
    let (_query_dir, query_manager) = create_db_manager().expect("legacy DbManager init failed");
    let day_start = 1_700_000_000;
    seed_snapshots(&query_manager, day_start, 100);

    db_group.bench_function("time_range_query_day_100_snapshots", |b| {
        b.iter(|| {
            let conn = query_manager.get_connection().expect("failed to get pooled connection");
            let mut stmt = conn
                .prepare(
                    "SELECT id, timestamp, activity_context_json, detected_activity,
                            work_type, activity_category, primary_app, processed,
                            batch_id, created_at, processed_at, is_idle, idle_duration_secs
                     FROM activity_snapshots
                     WHERE timestamp BETWEEN ?1 AND ?2",
                )
                .expect("failed to prepare time range query");

            let rows = stmt
                .query_map(params![day_start, day_start + 86_400], |row| {
                    Ok(ActivitySnapshot {
                        id: row.get(0)?,
                        timestamp: row.get(1)?,
                        activity_context_json: row.get(2)?,
                        detected_activity: row.get(3)?,
                        work_type: row.get(4)?,
                        activity_category: row.get(5)?,
                        primary_app: row.get(6)?,
                        processed: row.get::<_, i32>(7)? != 0,
                        batch_id: row.get(8)?,
                        created_at: row.get(9)?,
                        processed_at: row.get(10)?,
                        is_idle: row.get::<_, i32>(11)? != 0,
                        idle_duration_secs: row.get(12)?,
                    })
                })
                .expect("failed to execute time range query")
                .collect::<rusqlite::Result<Vec<_>>>()
                .expect("failed to collect time range rows");

            black_box(rows);
        });
    });
    // ---------------------------------------------------------------------
    // Bulk insert benchmark (1000 snapshots via transaction)
    // ---------------------------------------------------------------------
    let (_bulk_dir, bulk_manager) = create_db_manager().expect("legacy DbManager init failed");
    let bulk_snapshots = generate_snapshot_batch(1_000, 1_700_000_000);

    db_group.bench_function("bulk_insert_1000_snapshots", |b| {
        b.iter(|| {
            let mut conn = bulk_manager.get_connection().expect("failed to get pooled connection");

            conn.execute("DELETE FROM activity_snapshots", [])
                .expect("failed to clear activity_snapshots table");

            let tx = conn.transaction().expect("failed to start transaction for bulk insert");

            {
                let mut stmt = tx
                    .prepare(
                        "INSERT INTO activity_snapshots (
                            id, timestamp, activity_context_json, detected_activity,
                            work_type, activity_category, primary_app, processed,
                            batch_id, created_at, processed_at, is_idle, idle_duration_secs
                        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
                    )
                    .expect("failed to prepare bulk insert statement");

                for snapshot in &bulk_snapshots {
                    stmt.execute(params![
                        snapshot.id,
                        snapshot.timestamp,
                        snapshot.activity_context_json,
                        snapshot.detected_activity,
                        snapshot.work_type,
                        snapshot.activity_category,
                        snapshot.primary_app,
                        snapshot.processed as i32,
                        snapshot.batch_id,
                        snapshot.created_at,
                        snapshot.processed_at,
                        snapshot.is_idle as i32,
                        snapshot.idle_duration_secs
                    ])
                    .expect("bulk insert failed");
                }
            }

            tx.commit().expect("failed to commit bulk insert transaction");
        });
    });

    db_group.finish();
}

#[cfg(target_os = "macos")]
fn benchmark_legacy_macos_activity_provider_ax_on(c: &mut Criterion) {
    if std::env::var_os("PULSARC_ENABLE_MAC_BENCH").is_none() {
        eprintln!(
            "[macOS] PULSARC_ENABLE_MAC_BENCH not set; skipping AX-on benchmark. \n  Hint: run scripts/mac/prepare-ax-bench.sh and re-run with PULSARC_ENABLE_MAC_BENCH=1"
        );
        return;
    }

    let stability_probe = panic::catch_unwind(|| {
        let probe = MacOsActivityProvider::with_enrichment(false, false);
        let _ = probe.fetch();
    });

    if stability_probe.is_err() {
        eprintln!(
            "⚠️ Skipping macOS activity benchmarks: provider initialization panicked (likely missing permissions)."
        );
        return;
    }

    if !mac_ax::ax_trusted() {
        eprintln!(
            "⚠️ Accessibility not granted; skipping AX-on benchmark.\n  Hint: run scripts/mac/prepare-ax-bench.sh to open the correct System Settings pane."
        );
        return;
    }

    let mut group = c.benchmark_group("legacy_macos_activity_provider_ax_on");
    group.sample_size(100);
    group.warm_up_time(Duration::from_secs(3));
    group.measurement_time(Duration::from_secs(8));

    let provider_no_enrichment = MacOsActivityProvider::with_enrichment(false, false);
    group.bench_function("fetch_without_enrichment", |b| {
        b.iter(|| {
            let result = provider_no_enrichment.fetch();
            let _ = black_box(result);
        });
    });

    let provider_with_enrichment = MacOsActivityProvider::with_enrichment(false, true);
    group.bench_function("fetch_with_enrichment", |b| {
        b.iter(|| {
            let result = provider_with_enrichment.fetch();
            let _ = black_box(result);
        });
    });

    group.finish();
}

#[cfg(target_os = "macos")]
fn benchmark_legacy_macos_activity_provider_ax_off(c: &mut Criterion) {
    if std::env::var_os("PULSARC_ENABLE_MAC_BENCH").is_none() {
        println!("ℹ️ Skipping macOS AX-off benchmark (PULSARC_ENABLE_MAC_BENCH=1 required).");
        return;
    }

    let previous = std::env::var("PULSARC_FORCE_AX_DENIED").ok();
    std::env::set_var("PULSARC_FORCE_AX_DENIED", "1");

    let mut group = c.benchmark_group("legacy_macos_activity_provider_ax_off");
    group.sample_size(50);
    group.warm_up_time(Duration::from_secs(2));
    group.measurement_time(Duration::from_secs(5));

    let provider = MacOsActivityProvider::with_enrichment(false, false);
    group.bench_function("fetch_without_enrichment_ax_off", |b| {
        b.iter(|| {
            let result = provider.fetch();
            let _ = black_box(result);
        });
    });

    group.finish();

    match previous {
        Some(val) => std::env::set_var("PULSARC_FORCE_AX_DENIED", val),
        None => std::env::remove_var("PULSARC_FORCE_AX_DENIED"),
    }
}

#[cfg(not(target_os = "macos"))]
fn benchmark_legacy_macos_activity_provider_ax_on(_c: &mut Criterion) {}

#[cfg(not(target_os = "macos"))]
fn benchmark_legacy_macos_activity_provider_ax_off(_c: &mut Criterion) {}

fn benchmark_legacy_http_client_single(c: &mut Criterion) {
    let rt = Runtime::new().expect("failed to create tokio runtime");
    let http_client = HttpClient::new().expect("failed to create legacy HttpClient");

    let success_server =
        rt.block_on(start_success_server()).expect("failed to start success server");

    let mut group = c.benchmark_group("legacy_http_client_single");
    group.sample_size(200);
    group.warm_up_time(Duration::from_secs(5));
    group.measurement_time(Duration::from_secs(10));

    group.bench_function("single_request", |b| {
        b.iter(|| {
            let url = format!("{}/ok", success_server.base_url());
            let result = rt.block_on(async {
                http_client.execute_with_retry(http_client.inner().get(url)).await
            });
            black_box(result.expect("HTTP request failed"));
        });
    });

    group.finish();

    drop(success_server);
}

fn benchmark_legacy_http_client_retry(c: &mut Criterion) {
    let rt = Runtime::new().expect("failed to create tokio runtime");
    let http_client = HttpClient::new().expect("failed to create legacy HttpClient");

    let retry_server = rt.block_on(start_retry_server()).expect("failed to start retry server");

    let mut group = c.benchmark_group("legacy_http_client_retry");
    group.sample_size(10);
    group.warm_up_time(Duration::from_secs(3));
    group.measurement_time(Duration::from_secs(5));

    group.bench_function("request_with_retry", |b| {
        b.iter(|| {
            retry_server.reset();
            let url = format!("{}/retry", retry_server.base_url());
            let result = rt.block_on(async {
                http_client.execute_with_retry(http_client.inner().get(url)).await
            });
            black_box(result.expect("HTTP request with retry failed"));
        });
    });

    group.finish();

    drop(retry_server);
}

fn benchmark_legacy_mdm_client(c: &mut Criterion) {
    use std::sync::Arc as StdArc;

    let cert_dir = match resolve_mdm_cert_dir() {
        Some(dir) => dir,
        None => {
            println!("⚠️ Skipping MDM benchmarks: .mdm-certs directory not found.");
            return;
        }
    };

    if !cert_dir.exists() {
        println!("⚠️ Skipping MDM benchmarks: .mdm-certs directory not found.");
        return;
    }

    let rt = Runtime::new().expect("failed to create tokio runtime for MDM benchmarks");

    let server = match rt.block_on(start_mdm_server(&cert_dir)) {
        Ok(server) => server,
        Err(err) => {
            println!("⚠️ Skipping MDM benchmarks: failed to start test server ({err})");
            return;
        }
    };

    let config_url = format!("{}/config", server.base_url());
    let ca_path = cert_dir.join("ca-cert.pem");
    let client = match MdmClient::with_ca_cert(&config_url, &ca_path) {
        Ok(client) => StdArc::new(client),
        Err(err) => {
            println!("⚠️ Skipping MDM benchmarks: failed to construct client with CA cert ({err})");
            return;
        }
    };

    let local_config = StdArc::new(
        MdmConfig::builder()
            .policy_enforcement(false)
            .allow_local_override(true)
            .update_interval_secs(1800)
            .build()
            .expect("local config should validate"),
    );

    let mut group = c.benchmark_group("legacy_mdm_client_warm");
    group.sample_size(200);
    group.warm_up_time(Duration::from_secs(5));
    group.measurement_time(Duration::from_secs(10));

    let client_fetch = StdArc::clone(&client);
    group.bench_function("fetch_config", |b| {
        let client = StdArc::clone(&client_fetch);
        b.to_async(&rt).iter(move || {
            let client = StdArc::clone(&client);
            async move {
                let config = client.fetch_config().await.expect("MDM fetch_config should succeed");
                black_box(config);
            }
        });
    });

    let client_merge = StdArc::clone(&client);
    let local_config_merge = StdArc::clone(&local_config);
    group.bench_function("fetch_and_merge", |b| {
        let client = StdArc::clone(&client_merge);
        let local_config = StdArc::clone(&local_config_merge);
        b.to_async(&rt).iter(move || {
            let client = StdArc::clone(&client);
            let local_config = StdArc::clone(&local_config);
            async move {
                let merged = client
                    .fetch_and_merge((*local_config).clone())
                    .await
                    .expect("MDM fetch_and_merge should succeed");
                black_box(merged);
            }
        });
    });

    group.finish();

    let mut cold_group = c.benchmark_group("legacy_mdm_client_cold");
    cold_group.sample_size(50);
    cold_group.warm_up_time(Duration::from_secs(3));
    cold_group.measurement_time(Duration::from_secs(6));

    cold_group.bench_function("fetch_config_cold", |b| {
        let url = config_url.clone();
        let ca = ca_path.clone();
        b.to_async(&rt).iter(move || {
            let url = url.clone();
            let ca = ca.clone();
            async move {
                let client =
                    MdmClient::with_ca_cert(&url, &ca).expect("MDM fetch_config_cold client init");
                let config = client.fetch_config().await.expect("MDM cold fetch_config");
                black_box(config);
            }
        });
    });

    cold_group.finish();

    drop(server);
}

fn create_db_manager() -> LegacyDbManagerInitResult {
    let temp_dir = TempDir::new().map_err(|e| format!("tempdir creation failed: {e}"))?;
    let db_path = temp_dir.path().join("legacy.db");
    let manager = LegacyDbManager::new(&db_path).map_err(|e| format!("{e}"))?;
    Ok((temp_dir, Arc::new(manager)))
}

fn new_snapshot(timestamp: i64) -> ActivitySnapshot {
    ActivitySnapshot {
        id: Uuid::now_v7().to_string(),
        timestamp,
        activity_context_json: r#"{"activity":"coding"}"#.to_string(),
        detected_activity: "coding".to_string(),
        work_type: Some("focus".to_string()),
        activity_category: Some("development".to_string()),
        primary_app: "com.example.Editor".to_string(),
        processed: false,
        batch_id: None,
        created_at: timestamp,
        processed_at: None,
        is_idle: false,
        idle_duration_secs: Some(0),
    }
}

fn generate_snapshot_batch(count: usize, start_timestamp: i64) -> Vec<ActivitySnapshot> {
    (0..count).map(|idx| new_snapshot(start_timestamp + idx as i64)).collect()
}

fn seed_snapshots(manager: &LegacyDbManager, day_start: i64, count: usize) {
    let snapshots = generate_snapshot_batch(count, day_start);
    let mut conn = manager.get_connection().expect("failed to get pooled connection for seeding");

    let tx = conn.transaction().expect("failed to start transaction for seeding");
    {
        let mut stmt = tx
            .prepare(
                "INSERT INTO activity_snapshots (
                    id, timestamp, activity_context_json, detected_activity,
                    work_type, activity_category, primary_app, processed,
                    batch_id, created_at, processed_at, is_idle, idle_duration_secs
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            )
            .expect("failed to prepare seed insert statement");

        for snapshot in snapshots {
            stmt.execute(params![
                snapshot.id,
                snapshot.timestamp,
                snapshot.activity_context_json,
                snapshot.detected_activity,
                snapshot.work_type,
                snapshot.activity_category,
                snapshot.primary_app,
                snapshot.processed as i32,
                snapshot.batch_id,
                snapshot.created_at,
                snapshot.processed_at,
                snapshot.is_idle as i32,
                snapshot.idle_duration_secs
            ])
            .expect("failed to insert seed snapshot");
        }
    }
    tx.commit().expect("failed to commit seed snapshots transaction");
}

struct MdmTestServer {
    base_url: String,
    shutdown_tx: Option<oneshot::Sender<()>>,
}

impl MdmTestServer {
    fn base_url(&self) -> &str {
        &self.base_url
    }
}

impl Drop for MdmTestServer {
    fn drop(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
    }
}

async fn start_mdm_server(cert_dir: &Path) -> Result<MdmTestServer, String> {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|e| format!("failed to bind MDM listener: {e}"))?;
    let addr = listener.local_addr().map_err(|e| format!("failed to get local addr: {e}"))?;

    let server_config = load_server_config(cert_dir)?;
    let acceptor = TlsAcceptor::from(std::sync::Arc::new(server_config));
    let response_body = std::sync::Arc::new(sample_mdm_config_body()?);

    let (shutdown_tx, mut shutdown_rx) = oneshot::channel();

    tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = &mut shutdown_rx => break,
                accept = listener.accept() => {
                    match accept {
                        Ok((stream, _addr)) => {
                            let acceptor = acceptor.clone();
                            let body = response_body.clone();
                            tokio::spawn(async move {
                                match acceptor.accept(stream).await {
                                    Ok(tls_stream) => {
                                        let service_body = body.clone();
                                        let service = service_fn(move |req| {
                                            let body = service_body.clone();
                                            async move {
                                                if req.uri().path() == "/config" {
                                                    Ok::<_, Infallible>(
                                                        Response::builder()
                                                            .status(StatusCode::OK)
                                                            .header("content-type", "application/json")
                                                            .body(Body::from((*body).clone()))
                                                            .expect("valid /config response"),
                                                    )
                                                } else {
                                                    Ok::<_, Infallible>(
                                                        Response::builder()
                                                            .status(StatusCode::NOT_FOUND)
                                                            .body(Body::from("not found"))
                                                            .expect("valid not found response"),
                                                    )
                                                }
                                            }
                                        });

                                        if let Err(err) = hyper::server::conn::Http::new()
                                            .serve_connection(tls_stream, service)
                                            .await
                                        {
                                            eprintln!("MDM server connection error: {err}");
                                        }
                                    }
                                    Err(err) => eprintln!("MDM TLS accept error: {err}"),
                                }
                            });
                        }
                        Err(err) => {
                            eprintln!("MDM server accept error: {err}");
                        }
                    }
                }
            }
        }
    });

    Ok(MdmTestServer { base_url: format!("https://{}", addr), shutdown_tx: Some(shutdown_tx) })
}

fn load_server_config(cert_dir: &Path) -> Result<ServerConfig, String> {
    let certs = load_cert_chain(&cert_dir.join("server-fullchain.pem"))?;
    let key = load_private_key(&cert_dir.join("server-key.pem"))?;
    let mut config = ServerConfig::builder()
        .with_safe_defaults()
        .with_no_client_auth()
        .with_single_cert(certs, key)
        .map_err(|e| format!("failed to build TLS config: {e}"))?;
    config.alpn_protocols = vec![b"http/1.1".to_vec()];
    Ok(config)
}

fn load_cert_chain(path: &Path) -> Result<Vec<RustlsCertificate>, String> {
    let file = File::open(path)
        .map_err(|e| format!("failed to open certificate '{}': {e}", path.display()))?;
    let mut reader = BufReader::new(file);
    let certs = load_pem_certs(&mut reader)
        .map_err(|e| format!("failed to read certificates '{}': {e:?}", path.display()))?;
    if certs.is_empty() {
        return Err(format!("no certificates found in '{}'", path.display()));
    }
    Ok(certs.into_iter().map(RustlsCertificate).collect())
}

fn load_private_key(path: &Path) -> Result<PrivateKey, String> {
    let file = File::open(path)
        .map_err(|e| format!("failed to open private key '{}': {e}", path.display()))?;
    let mut reader = BufReader::new(file);
    let mut keys = pkcs8_private_keys(&mut reader)
        .map_err(|e| format!("failed to read private key '{}': {e:?}", path.display()))?;
    if keys.is_empty() {
        let file = File::open(path)
            .map_err(|e| format!("failed to reopen private key '{}': {e}", path.display()))?;
        let mut reader = BufReader::new(file);
        keys = rsa_private_keys(&mut reader)
            .map_err(|e| format!("failed to read RSA private key '{}': {e:?}", path.display()))?;
    }
    let key = keys.pop().ok_or_else(|| format!("no private keys found in '{}'", path.display()))?;
    Ok(PrivateKey(key))
}

fn sample_mdm_config_body() -> Result<Vec<u8>, String> {
    let sample_policy = PolicySetting::new(PolicyValue::Boolean(true));

    let config = MdmConfig::builder()
        .policy_enforcement(true)
        .remote_config_url("https://mdm.local/config")
        .add_policy("require_mdm", sample_policy)
        .update_interval_secs(900)
        .allow_local_override(false)
        .build()
        .map_err(|e| format!("failed to build sample config: {e}"))?;

    serde_json::to_vec(&config).map_err(|e| format!("failed to serialize sample config: {e}"))
}

struct HttpTestServer {
    base_url: String,
    shutdown_tx: Option<oneshot::Sender<()>>,
    counter: Option<Arc<AtomicUsize>>,
}

impl HttpTestServer {
    fn base_url(&self) -> &str {
        &self.base_url
    }

    fn reset(&self) {
        if let Some(counter) = &self.counter {
            counter.store(0, Ordering::SeqCst);
        }
    }
}

impl Drop for HttpTestServer {
    fn drop(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
    }
}

async fn start_success_server() -> Result<HttpTestServer, String> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|e| format!("failed to bind listener: {e}"))?;
    let addr = listener.local_addr().map_err(|e| format!("failed to get local addr: {e}"))?;
    let std_listener =
        listener.into_std().map_err(|e| format!("failed to convert listener: {e}"))?;
    std_listener.set_nonblocking(true).map_err(|e| format!("failed to set nonblocking: {e}"))?;

    let (shutdown_tx, shutdown_rx) = oneshot::channel();

    let make_svc = make_service_fn(|_| async {
        Ok::<_, std::convert::Infallible>(service_fn(|_req| async {
            Ok::<_, std::convert::Infallible>(Response::new(Body::from("ok")))
        }))
    });

    let server = Server::from_tcp(std_listener)
        .map_err(|e| format!("failed to create server from tcp: {e}"))?
        .serve(make_svc)
        .with_graceful_shutdown(async move {
            let _ = shutdown_rx.await;
        });

    tokio::spawn(async move {
        if let Err(err) = server.await {
            eprintln!("test HTTP server error: {err}");
        }
    });

    Ok(HttpTestServer {
        base_url: format!("http://{}", addr),
        shutdown_tx: Some(shutdown_tx),
        counter: None,
    })
}

async fn start_retry_server() -> Result<HttpTestServer, String> {
    let counter = Arc::new(AtomicUsize::new(0));
    let service_counter = Arc::clone(&counter);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|e| format!("failed to bind listener: {e}"))?;
    let addr = listener.local_addr().map_err(|e| format!("failed to get local addr: {e}"))?;
    let std_listener =
        listener.into_std().map_err(|e| format!("failed to convert listener: {e}"))?;
    std_listener.set_nonblocking(true).map_err(|e| format!("failed to set nonblocking: {e}"))?;

    let (shutdown_tx, shutdown_rx) = oneshot::channel();

    let make_svc = make_service_fn(move |_| {
        let counter_clone = Arc::clone(&service_counter);
        async move {
            Ok::<_, std::convert::Infallible>(service_fn(move |_req| {
                let counter_inner = Arc::clone(&counter_clone);
                async move {
                    let attempt = counter_inner.fetch_add(1, Ordering::SeqCst);
                    let status = if attempt == 0 {
                        StatusCode::INTERNAL_SERVER_ERROR
                    } else {
                        StatusCode::OK
                    };

                    Ok::<_, std::convert::Infallible>(
                        Response::builder()
                            .status(status)
                            .body(Body::from("retry"))
                            .expect("failed to build HTTP response"),
                    )
                }
            }))
        }
    });

    let server = Server::from_tcp(std_listener)
        .map_err(|e| format!("failed to create server from tcp: {e}"))?
        .serve(make_svc)
        .with_graceful_shutdown(async move {
            let _ = shutdown_rx.await;
        });

    tokio::spawn(async move {
        if let Err(err) = server.await {
            eprintln!("test HTTP server error: {err}");
        }
    });

    Ok(HttpTestServer {
        base_url: format!("http://{}", addr),
        shutdown_tx: Some(shutdown_tx),
        counter: Some(counter),
    })
}

criterion_group!(
    baseline,
    benchmark_legacy_db_manager,
    benchmark_legacy_http_client_single,
    benchmark_legacy_http_client_retry,
    benchmark_legacy_mdm_client,
    benchmark_legacy_macos_activity_provider_ax_on,
    benchmark_legacy_macos_activity_provider_ax_off
);
criterion_main!(baseline);
