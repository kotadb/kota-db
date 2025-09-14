// Integration tests for /api/v1 endpoints on the Services HTTP server
// Anti-mock: spins up a real axum server and uses reqwest HTTP client

use anyhow::Result;
use kotadb::{
    create_file_storage, create_primary_index_for_tests, create_services_server,
    create_trigram_index_for_tests, Index, Storage,
};
use reqwest::{Client, StatusCode};
use serde_json::Value;
use std::sync::Arc;
use tempfile::TempDir;
use tokio::{sync::Mutex, time::Duration};

/// Start Services HTTP server on a random available port for testing
async fn start_services_test_server() -> (String, TempDir, tokio::task::JoinHandle<Result<()>>) {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let storage_path = temp_dir.path().join("storage");
    let primary_path = temp_dir.path().join("primary");
    let trigram_path = temp_dir.path().join("trigram");

    // Real storage and indices
    let storage = create_file_storage(storage_path.to_str().unwrap(), Some(100))
        .await
        .expect("Failed to create storage");
    let primary_index = create_primary_index_for_tests(primary_path.to_str().unwrap())
        .await
        .expect("Failed to create primary index");
    let trigram_index = create_trigram_index_for_tests(trigram_path.to_str().unwrap())
        .await
        .expect("Failed to create trigram index");

    let storage_arc: Arc<Mutex<dyn Storage>> = Arc::new(Mutex::new(storage));
    let primary_arc: Arc<Mutex<dyn Index>> = Arc::new(Mutex::new(primary_index));
    let trigram_arc: Arc<Mutex<dyn Index>> = Arc::new(Mutex::new(trigram_index));

    let app = create_services_server(
        storage_arc.clone(),
        primary_arc.clone(),
        trigram_arc.clone(),
        temp_dir.path().to_path_buf(),
    );

    // Bind ephemeral port
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("Failed to bind port");
    let port = listener.local_addr().unwrap().port();
    let base_url = format!("http://127.0.0.1:{port}");

    // Spawn server
    let server_handle = tokio::spawn(async move {
        axum::serve(listener, app)
            .await
            .map_err(|e| anyhow::anyhow!(e))
    });

    // Give the server a moment to start
    tokio::time::sleep(Duration::from_millis(100)).await;

    (base_url, temp_dir, server_handle)
}

/// Initialize a tiny git repository with a single Rust file for ingestion
fn init_test_git_repo(root: &std::path::Path) -> Result<std::path::PathBuf> {
    use std::process::Command;
    let repo_dir = root.join("repo");
    std::fs::create_dir_all(&repo_dir)?;

    // git init and config
    assert!(
        Command::new("git")
            .arg("init")
            .current_dir(&repo_dir)
            .status()?
            .success(),
        "git init failed"
    );
    assert!(
        Command::new("git")
            .args(["config", "user.email", "test@example.com"])
            .current_dir(&repo_dir)
            .status()?
            .success(),
        "git config user.email failed"
    );
    assert!(
        Command::new("git")
            .args(["config", "user.name", "Test User"])
            .current_dir(&repo_dir)
            .status()?
            .success(),
        "git config user.name failed"
    );

    // Create file content
    let src_dir = repo_dir.join("src");
    std::fs::create_dir_all(&src_dir)?;
    let lib_rs = r#"
pub async fn process_test_item() {
    // async function used for search
    println!("processing");
}

pub fn hello_world() {
    println!("hello world");
}
"#;
    std::fs::write(src_dir.join("lib.rs"), lib_rs)?;

    // Commit
    assert!(
        Command::new("git")
            .args(["add", "."])
            .current_dir(&repo_dir)
            .status()?
            .success(),
        "git add failed"
    );
    assert!(
        Command::new("git")
            .args(["commit", "-m", "Initial commit"])
            .current_dir(&repo_dir)
            .status()?
            .success(),
        "git commit failed"
    );

    Ok(repo_dir)
}

#[tokio::test]
async fn v1_stats_health_path() -> Result<()> {
    let (base, _tmp, server) = start_services_test_server().await;
    let client = Client::new();

    // /api/v1/analysis/stats should be reachable
    let resp = client
        .get(format!("{}/api/v1/analysis/stats", base))
        .send()
        .await?;
    assert_eq!(resp.status(), StatusCode::OK);
    let json: Value = resp.json().await?;
    assert!(json.is_object());

    server.abort();
    Ok(())
}

#[tokio::test]
async fn v1_repositories_index_and_search_code() -> Result<()> {
    let (base, temp_dir, server) = start_services_test_server().await;
    let client = Client::new();

    // Create a tiny git repo
    let repo_dir = init_test_git_repo(temp_dir.path())?;

    // Register repository (local path)
    let register_resp = client
        .post(format!("{}/api/v1/repositories", base))
        .json(&serde_json::json!({"path": repo_dir.to_string_lossy()}))
        .send()
        .await?;
    assert_eq!(register_resp.status(), StatusCode::OK);
    let reg: Value = register_resp.json().await?;
    let job_id = reg["job_id"].as_str().expect("job_id missing");

    // Poll status until completed (with timeout)
    let start = std::time::Instant::now();
    loop {
        let status_resp = client
            .get(format!("{}/api/v1/index/status?job_id={}", base, job_id))
            .send()
            .await?;
        assert_eq!(status_resp.status(), StatusCode::OK);
        let body: Value = status_resp.json().await?;
        if let Some(status) = body["job"]["status"].as_str() {
            if status == "completed" {
                break;
            } else if status == "failed" {
                panic!("index job failed: {:?}", body);
            }
        }
        if start.elapsed() > Duration::from_secs(10) {
            panic!("indexing did not complete in time: {:?}", body);
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }

    // Now search for a known token
    let search_resp = client
        .post(format!("{}/api/v1/search/code", base))
        .json(&serde_json::json!({"query": "hello world"}))
        .send()
        .await?;
    assert_eq!(search_resp.status(), StatusCode::OK);
    let search_json: Value = search_resp.json().await?;
    assert!(search_json.is_object());

    // Stats endpoint should still work
    let stats_resp = client
        .get(format!("{}/api/v1/analysis/stats", base))
        .send()
        .await?;
    assert_eq!(stats_resp.status(), StatusCode::OK);

    server.abort();
    Ok(())
}

#[tokio::test]
async fn v1_symbol_routes_behave_without_symbols_db() -> Result<()> {
    let (base, temp_dir, server) = start_services_test_server().await;
    let client = Client::new();

    // Create and index a repo without symbol extraction (feature may be disabled)
    let repo_dir = init_test_git_repo(temp_dir.path())?;
    let _ = client
        .post(format!("{}/api/v1/repositories", base))
        .json(&serde_json::json!({"path": repo_dir.to_string_lossy()}))
        .send()
        .await?;

    // File symbols should 404 if symbols.kota is absent
    let file_symbols_resp = client
        .get(format!("{}/api/v1/files/symbols/src/lib.rs", base))
        .send()
        .await?;
    assert_eq!(file_symbols_resp.status(), StatusCode::NOT_FOUND);

    // Symbol search should still return 200 with valid JSON shape (may be empty)
    let sym_search = client
        .post(format!("{}/api/v1/search/symbols", base))
        .json(&serde_json::json!({"pattern": "hello_*", "limit": 10}))
        .send()
        .await?;
    assert_eq!(sym_search.status(), StatusCode::OK);

    // Callers/impact should return 500 without symbols db; verify error path is stable
    let callers = client
        .get(format!(
            "{}/api/v1/symbols/{}/callers?symbol=hello_world",
            base, "hello_world"
        ))
        .send()
        .await?;
    assert_eq!(callers.status(), StatusCode::INTERNAL_SERVER_ERROR);

    let impact = client
        .get(format!(
            "{}/api/v1/symbols/{}/impact?symbol=hello_world",
            base, "hello_world"
        ))
        .send()
        .await?;
    assert_eq!(impact.status(), StatusCode::INTERNAL_SERVER_ERROR);

    server.abort();
    Ok(())
}
