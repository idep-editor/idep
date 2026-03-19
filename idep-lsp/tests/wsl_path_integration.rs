use std::fs;
use std::process::Command;

use anyhow::{Context, Result};
use idep_lsp::{client::LspClient, document::DocumentManager};
use lsp_types::request::{GotoDefinition, Request};
use lsp_types::{
    GotoDefinitionParams, Position, TextDocumentIdentifier, Url, WorkDoneProgressParams,
};
use tokio::sync::Mutex;

// Integration test: runs only when RUN_WSL_RA_TEST=1 and /mnt/c exists.
#[tokio::test]
async fn resolves_definition_under_mnt_c() -> Result<()> {
    if std::env::var("RUN_WSL_RA_TEST").unwrap_or_default() != "1" {
        return Ok(());
    }

    // Run only in real WSL environments with /mnt/c.
    if !is_wsl() {
        eprintln!("Skipping WSL RA test: not running in WSL");
        return Ok(());
    }

    if !std::path::Path::new("/mnt/c").exists() {
        eprintln!("Skipping WSL RA test: /mnt/c does not exist");
        return Ok(());
    }

    // Ensure rust-analyzer is available. Skip if not installed.
    let version_out = Command::new("rust-analyzer").arg("--version").output();
    if version_out.is_err() || !version_out.unwrap().status.success() {
        eprintln!("Skipping WSL RA test: rust-analyzer not available");
        return Ok(());
    }

    // Create a workspace under /mnt/c
    let dir = tempfile::TempDir::new_in("/mnt/c").context("tempdir in /mnt/c")?;
    fs::create_dir_all(dir.path().join("src"))?;
    fs::write(
        dir.path().join("Cargo.toml"),
        r#"[package]
name = "wsl-path"
version = "0.1.0"
edition = "2021"

[workspace]
members = []
"#,
    )?;

    let file_path = dir.path().join("src/main.rs");
    let source = "fn main() { let foo = 1; let _ = foo; }";
    fs::write(&file_path, source)?;

    let root_uri = Url::from_directory_path(dir.path()).expect("root uri");
    let file_uri = Url::from_file_path(&file_path).expect("file uri");

    let client = std::sync::Arc::new(Mutex::new(LspClient::spawn("rust-analyzer", &[])?));

    // Initialize/initialized
    {
        let mut c = client.lock().await;
        c.initialize(root_uri.clone()).await?;
        c.initialized().await?;
    }

    // Open document via DocumentManager (applies to_server_uri internally)
    {
        let mut docs = DocumentManager::new(client.clone());
        docs.did_open(file_uri.clone(), "rust".into(), source.to_string())
            .await?;
    }

    // Request goto definition on the usage of foo
    let position = Position {
        line: 0,
        character: thirty_two(), // after "let _ = foo"
    };
    let params = GotoDefinitionParams {
        text_document_position_params: lsp_types::TextDocumentPositionParams {
            text_document: TextDocumentIdentifier {
                uri: file_uri.clone(),
            },
            position,
        },
        work_done_progress_params: WorkDoneProgressParams {
            work_done_token: None,
        },
        partial_result_params: Default::default(),
    };

    let response = {
        let mut c = client.lock().await;
        c.request(GotoDefinition::METHOD, params).await?
    };

    assert!(
        !response.is_null(),
        "definition response should not be null"
    );

    // Shutdown cleanly
    {
        let mut c = client.lock().await;
        let _ = c.shutdown().await;
    }

    Ok(())
}

fn is_wsl() -> bool {
    if std::env::var("WSL_DISTRO_NAME").is_ok() {
        return true;
    }
    if let Ok(version) = std::fs::read_to_string("/proc/version") {
        return version.to_lowercase().contains("microsoft");
    }
    false
}

// Helper to avoid clippy literal out of range on Position
const fn thirty_two() -> u32 {
    32
}
