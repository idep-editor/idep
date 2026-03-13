use std::fs;
use std::sync::Arc;

use anyhow::Result;
use idep_lsp::{client::LspClient, document::DocumentManager};
use lsp_types::request::{Completion, Request};
use lsp_types::{
    CompletionParams, CompletionResponse, Position, TextDocumentIdentifier,
    TextDocumentPositionParams, Url,
};
use tokio::sync::Mutex;

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
