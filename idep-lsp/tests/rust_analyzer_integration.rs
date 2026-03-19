use std::fs;
use std::sync::Arc;

use anyhow::{Context, Result};
use idep_lsp::{client::LspClient, document::DocumentManager};
use lsp_types::request::{Completion, Request};
use lsp_types::{
    CompletionParams, CompletionResponse, Position, TextDocumentIdentifier,
    TextDocumentPositionParams, Url,
};
use tokio::sync::Mutex;

async fn read_next_notification(
    client: &Arc<Mutex<LspClient>>,
) -> Result<lsp_server::Notification> {
    let mut c = client.lock().await;
    tokio::time::timeout(std::time::Duration::from_secs(5), c.read_notification())
        .await
        .context("timed out waiting for LSP notification")?
        .context("failed to read LSP notification")
}

#[tokio::test]
async fn spawns_rust_analyzer_and_gets_completions() -> Result<()> {
    if std::env::var("RUN_RA_INT").unwrap_or_default() != "1" {
        return Ok(());
    }
    // 1. Create a temp Rust project
    let dir = tempfile::tempdir()?;
    fs::create_dir_all(dir.path().join("src"))?;
    fs::write(
        dir.path().join("Cargo.toml"),
        r#"[package]
name = "ra-smoke"
version = "0.1.0"
edition = "2021"

[workspace]
members = []
"#,
    )?;

    let file_path = dir.path().join("src/main.rs");
    let source = "fn main() { let fo = 1; fo }";
    fs::write(&file_path, source)?;

    let root_uri = Url::from_directory_path(dir.path()).expect("root uri");
    let file_uri = Url::from_file_path(&file_path).expect("file uri");

    // 2. Spawn rust-analyzer
    let client = Arc::new(Mutex::new(LspClient::spawn("rust-analyzer", &[])?));

    // 3. Initialize/initialized handshake
    {
        let mut c = client.lock().await;
        c.initialize(root_uri.clone()).await?;
        c.initialized().await?;
    }

    // 4. Open document
    {
        let mut docs = DocumentManager::new(client.clone());
        docs.did_open(file_uri.clone(), "rust".into(), source.to_string())
            .await?;
    }

    // 5. Request completions at the end of "fo"
    let position = Position {
        line: 0,
        character: 24, // after "fo"
    };
    let params = CompletionParams {
        text_document_position: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier {
                uri: file_uri.clone(),
            },
            position,
        },
        work_done_progress_params: Default::default(),
        partial_result_params: Default::default(),
        context: None,
    };

    let resp_val = {
        let mut c = client.lock().await;
        c.request(Completion::METHOD, params).await?
    };

    // 6. Decode completion response and assert we got something
    let completion_resp: CompletionResponse = serde_json::from_value(resp_val)?;
    match completion_resp {
        CompletionResponse::Array(items) => {
            assert!(!items.is_empty(), "expected at least one completion item");
        }
        CompletionResponse::List(list) => {
            assert!(
                !list.items.is_empty(),
                "expected at least one completion item"
            );
        }
    }

    // 7. Shutdown cleanly
    {
        let mut c = client.lock().await;
        c.shutdown().await?;
    }

    Ok(())
}

#[tokio::test]
async fn hover_returns_type_info() -> Result<()> {
    if std::env::var("RUN_RA_INT").unwrap_or_default() != "1" {
        return Ok(());
    }

    let dir = tempfile::tempdir()?;
    fs::create_dir_all(dir.path().join("src"))?;
    fs::write(
        dir.path().join("Cargo.toml"),
        r#"[package]
name = "ra-hover"
version = "0.1.0"
edition = "2021"
"#,
    )?;

    let file_path = dir.path().join("src/main.rs");
    let source = "fn main() { let x = 1; x }";
    fs::write(&file_path, source)?;

    let root_uri = Url::from_directory_path(dir.path()).unwrap();
    let file_uri = Url::from_file_path(&file_path).unwrap();

    let client = Arc::new(Mutex::new(LspClient::spawn("rust-analyzer", &[])?));
    {
        let mut c = client.lock().await;
        c.initialize(root_uri.clone()).await?;
        c.initialized().await?;
    }

    let mut docs = DocumentManager::new(client.clone());
    docs.did_open(file_uri.clone(), "rust".into(), source.to_string())
        .await?;

    let hover_text = docs
        .hover_text(
            file_uri.clone(),
            Position {
                line: 0,
                character: 21,
            },
        )
        .await?;
    assert!(hover_text.is_some());
    let hover_text = hover_text.unwrap();
    assert!(hover_text.contains("i32") || hover_text.contains("i64"));

    {
        let mut c = client.lock().await;
        c.shutdown().await?;
    }

    Ok(())
}

#[tokio::test]
async fn goto_definition_resolves_function() -> Result<()> {
    if std::env::var("RUN_RA_INT").unwrap_or_default() != "1" {
        return Ok(());
    }

    let dir = tempfile::tempdir()?;
    fs::create_dir_all(dir.path().join("src"))?;
    fs::write(
        dir.path().join("Cargo.toml"),
        r#"[package]
name = "ra-goto"
version = "0.1.0"
edition = "2021"
"#,
    )?;

    let file_path = dir.path().join("src/main.rs");
    let source = "fn foo() -> i32 { 1 }\nfn main() { let x = foo(); x }";
    fs::write(&file_path, source)?;

    let root_uri = Url::from_directory_path(dir.path()).unwrap();
    let file_uri = Url::from_file_path(&file_path).unwrap();

    let client = Arc::new(Mutex::new(LspClient::spawn("rust-analyzer", &[])?));
    {
        let mut c = client.lock().await;
        c.initialize(root_uri.clone()).await?;
        c.initialized().await?;
    }

    let mut docs = DocumentManager::new(client.clone());
    docs.did_open(file_uri.clone(), "rust".into(), source.to_string())
        .await?;

    let locations = docs
        .goto_definition_locations(
            file_uri.clone(),
            Position {
                line: 1,
                character: 24,
            },
        )
        .await?;
    assert!(!locations.is_empty());
    assert_eq!(locations[0].uri, file_uri);
    assert_eq!(locations[0].range.start.line, 0);

    {
        let mut c = client.lock().await;
        c.shutdown().await?;
    }

    Ok(())
}

#[tokio::test]
async fn publish_diagnostics_notification_stored() -> Result<()> {
    if std::env::var("RUN_RA_INT").unwrap_or_default() != "1" {
        return Ok(());
    }

    let dir = tempfile::tempdir()?;
    fs::create_dir_all(dir.path().join("src"))?;
    fs::write(
        dir.path().join("Cargo.toml"),
        r#"[package]
name = "ra-diag"
version = "0.1.0"
edition = "2021"
"#,
    )?;

    let file_path = dir.path().join("src/main.rs");
    let source = "fn main() { let x = \"hello\"; x + 1; }";
    fs::write(&file_path, source)?;

    let root_uri = Url::from_directory_path(dir.path()).unwrap();
    let file_uri = Url::from_file_path(&file_path).unwrap();

    let client = Arc::new(Mutex::new(LspClient::spawn("rust-analyzer", &[])?));
    {
        let mut c = client.lock().await;
        c.initialize(root_uri.clone()).await?;
        c.initialized().await?;
    }

    let mut docs = DocumentManager::new(client.clone());
    docs.did_open(file_uri.clone(), "rust".into(), source.to_string())
        .await?;

    let mut seen_diagnostics = false;
    for _ in 0..30 {
        let notif = read_next_notification(&client).await?;
        if notif.method == "textDocument/publishDiagnostics" {
            let params: lsp_types::PublishDiagnosticsParams = serde_json::from_value(notif.params)?;
            docs.handle_publish_diagnostics(params);
            seen_diagnostics = true;
            break;
        }
    }

    assert!(seen_diagnostics, "expected publishDiagnostics notification");
    assert!(!docs.get_diagnostics(&file_uri).is_empty());

    {
        let mut c = client.lock().await;
        c.shutdown().await?;
    }

    Ok(())
}
