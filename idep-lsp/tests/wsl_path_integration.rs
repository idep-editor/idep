use std::fs;
use std::process::Command;

use anyhow::{Context, Result};
use idep_lsp::{client::LspClient, document::DocumentManager};
use lsp_types::request::{GotoDefinition, Request};
use lsp_types::{
    GotoDefinitionParams, Position, TextDocumentIdentifier, Url, WorkDoneProgressParams,
};
use tokio::sync::Mutex;

// Ignored by default: requires rust-analyzer in PATH and WSL /mnt/c filesystem.
#[tokio::test]
#[ignore = "requires rust-analyzer and WSL /mnt/c; set RUN_WSL_RA_TEST=1 to run"]
async fn resolves_definition_under_mnt_c() -> Result<()> {
    if std::env::var("RUN_WSL_RA_TEST").unwrap_or_default() != "1" {
        return Ok(());
    }

    if !std::path::Path::new("/mnt/c").exists() {
        return Ok(());
    }

    // Ensure rust-analyzer is available
    Command::new("rust-analyzer")
        .arg("--version")
        .output()
        .context("rust-analyzer not found in PATH")?;

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

// Helper to avoid clippy literal out of range on Position
const fn thirty_two() -> u32 {
    32
}
