//! Integration test: open file with error, verify diagnostic displayed
//!
//! This test verifies that when a file with errors is opened,
//! the TUI properly receives and stores diagnostics from the LSP server.

use std::sync::Arc;

use anyhow::{Context, Result};
use idep_lsp::{client::LspClient, document::DocumentManager};
use lsp_types::{Diagnostic, DiagnosticSeverity, Url};
use tokio::sync::Mutex;

/// Helper to read next notification from LSP client
async fn read_next_notification(
    client: &Arc<Mutex<LspClient>>,
) -> Result<lsp_server::Notification> {
    let mut c = client.lock().await;
    tokio::time::timeout(std::time::Duration::from_secs(10), c.read_notification())
        .await
        .context("timed out waiting for LSP notification")?
        .context("failed to read LSP notification")
}

/// Integration test: open a Rust file with a type error and verify
/// that rust-analyzer publishes diagnostics that are captured.
#[tokio::test]
async fn open_file_with_error_verify_diagnostic_displayed() -> Result<()> {
    // Skip if not running integration tests
    if std::env::var("RUN_RA_INT").unwrap_or_default() != "1" {
        return Ok(());
    }

    let dir = tempfile::tempdir()?;
    std::fs::create_dir_all(dir.path().join("src"))?;
    std::fs::write(
        dir.path().join("Cargo.toml"),
        r#"[package]
name = "diag-test"
version = "0.1.0"
edition = "2021"
"#,
    )?;

    // Create a file with a clear type error: adding string to integer
    let file_path = dir.path().join("src/main.rs");
    let source = r#"fn main() {
    let x = "hello";
    let y = x + 1;
}"#;
    std::fs::write(&file_path, source)?;

    let root_uri = Url::from_directory_path(dir.path()).unwrap();
    let file_uri = Url::from_file_path(&file_path).unwrap();

    // Spawn rust-analyzer
    let client = Arc::new(Mutex::new(LspClient::spawn("rust-analyzer", &[])?));
    {
        let mut c = client.lock().await;
        c.initialize(root_uri.clone()).await?;
        c.initialized().await?;
    }

    // Set up document manager (like the TUI does)
    let mut doc_manager = DocumentManager::new(client.clone());
    doc_manager
        .did_open(file_uri.clone(), "rust".into(), source.to_string())
        .await?;

    // Wait for publishDiagnostics notification
    let mut found_error = false;
    let mut received_diagnostics: Vec<Diagnostic>;

    for _ in 0..50 {
        let notif = read_next_notification(&client).await?;
        if notif.method == "textDocument/publishDiagnostics" {
            let params: lsp_types::PublishDiagnosticsParams = serde_json::from_value(notif.params)?;
            doc_manager.handle_publish_diagnostics(params.clone());
            received_diagnostics = params.diagnostics;

            // Check if any diagnostic is an error
            for diag in &received_diagnostics {
                if diag.severity == Some(DiagnosticSeverity::ERROR) {
                    found_error = true;
                    break;
                }
            }
            if found_error {
                break;
            }
        }
    }

    // Verify we found at least one error diagnostic
    assert!(
        found_error,
        "expected to receive an ERROR diagnostic for the type error"
    );

    // Verify diagnostics are stored in DocumentManager
    let stored = doc_manager.get_diagnostics(&file_uri);
    assert!(
        !stored.is_empty(),
        "diagnostics should be stored in DocumentManager"
    );

    // Verify at least one error is stored
    let error_count = stored
        .iter()
        .filter(|d| d.severity == Some(DiagnosticSeverity::ERROR))
        .count();
    assert!(
        error_count > 0,
        "at least one ERROR diagnostic should be stored, got {} errors",
        error_count
    );

    // Verify the diagnostic points to the correct line (line 2, where x + 1 is)
    let line_2_errors: Vec<_> = stored
        .iter()
        .filter(|d| d.range.start.line == 2 && d.severity == Some(DiagnosticSeverity::ERROR))
        .collect();
    assert!(
        !line_2_errors.is_empty(),
        "expected error diagnostic on line 2 (the type error line)"
    );

    // Clean shutdown
    {
        let mut c = client.lock().await;
        c.shutdown().await?;
    }

    Ok(())
}

/// Test that verifies the TUI App would correctly receive and store diagnostics.
/// This simulates the TUI's diagnostic flow without requiring a full UI harness.
#[tokio::test]
async fn diagnostic_storage_matches_received() -> Result<()> {
    // Skip if not running integration tests
    if std::env::var("RUN_RA_INT").unwrap_or_default() != "1" {
        return Ok(());
    }

    let dir = tempfile::tempdir()?;
    std::fs::create_dir_all(dir.path().join("src"))?;
    std::fs::write(
        dir.path().join("Cargo.toml"),
        r#"[package]
name = "diag-storage"
version = "0.1.0"
edition = "2021"
"#,
    )?;

    // File with multiple issues: unused variable and type error
    let file_path = dir.path().join("src/lib.rs");
    let source = r#"pub fn foo() {
    let unused = 42;
    let x = "hello";
    x + 1
}"#;
    std::fs::write(&file_path, source)?;

    let root_uri = Url::from_directory_path(dir.path()).unwrap();
    let file_uri = Url::from_file_path(&file_path).unwrap();

    let client = Arc::new(Mutex::new(LspClient::spawn("rust-analyzer", &[])?));
    {
        let mut c = client.lock().await;
        c.initialize(root_uri.clone()).await?;
        c.initialized().await?;
    }

    let mut doc_manager = DocumentManager::new(client.clone());
    doc_manager
        .did_open(file_uri.clone(), "rust".into(), source.to_string())
        .await?;

    // Collect all diagnostics notifications
    let mut all_diagnostics: Vec<Diagnostic> = Vec::new();
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(8);

    while tokio::time::Instant::now() < deadline {
        match tokio::time::timeout(
            std::time::Duration::from_millis(500),
            read_next_notification(&client),
        )
        .await
        {
            Ok(Ok(notif)) => {
                if notif.method == "textDocument/publishDiagnostics" {
                    let params: lsp_types::PublishDiagnosticsParams =
                        serde_json::from_value(notif.params)?;
                    doc_manager.handle_publish_diagnostics(params.clone());
                    all_diagnostics.extend(params.diagnostics);
                }
            }
            _ => continue,
        }

        // Break if we have diagnostics
        if !all_diagnostics.is_empty() && !doc_manager.get_diagnostics(&file_uri).is_empty() {
            // Small delay to catch any trailing notifications
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
            break;
        }
    }

    // Verify DocumentManager has the same diagnostics we received
    let stored = doc_manager.get_diagnostics(&file_uri);
    assert!(
        !stored.is_empty(),
        "diagnostics should be stored after receiving publishDiagnostics"
    );

    // The stored diagnostics should match what was in the notification
    assert_eq!(
        stored.len(),
        all_diagnostics.len(),
        "stored diagnostics count should match received count"
    );

    // Verify diagnostic severity levels are preserved
    let error_count = stored
        .iter()
        .filter(|d| d.severity == Some(DiagnosticSeverity::ERROR))
        .count();
    let warning_count = stored
        .iter()
        .filter(|d| d.severity == Some(DiagnosticSeverity::WARNING))
        .count();

    println!(
        "Found {} errors and {} warnings",
        error_count, warning_count
    );

    // We expect at least one error (the type error on x + 1)
    assert!(
        error_count >= 1,
        "expected at least 1 error diagnostic, got {}",
        error_count
    );

    // Clean shutdown
    {
        let mut c = client.lock().await;
        c.shutdown().await?;
    }

    Ok(())
}
