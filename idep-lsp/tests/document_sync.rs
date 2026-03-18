use anyhow::Result;
use idep_lsp::client::LspClient;
use idep_lsp::document::DocumentManager;
use lsp_types::{TextDocumentContentChangeEvent, Url};
use tokio::sync::Mutex;

#[tokio::test]
#[ignore = "requires python3; run with --ignored if python3 is available"]
async fn sends_open_change_save_close_sequence() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let log_path = dir.path().join("log.txt");

    let script = r#"
import sys, json
log_path = sys.argv[1]

def read_msg():
    while True:
        line = sys.stdin.readline()
        if not line:
            return None
        if line.lower().startswith("content-length:"):
            length = int(line.split(":",1)[1].strip())
            sys.stdin.readline()
            body = sys.stdin.read(length)
            return json.loads(body)

with open(log_path, "w"):
    pass

while True:
    msg = read_msg()
    if msg is None:
        break
    method = msg.get("method")
    if method:
        with open(log_path, "a") as f:
            f.write(method + "\n")
"#;

    // Spawn python logger as the "server"
    let client = LspClient::spawn(
        "python3",
        &["-u", "-c", script, log_path.to_string_lossy().as_ref()],
    )?;
    let client = std::sync::Arc::new(Mutex::new(client));
    let mut docs = DocumentManager::new(client.clone());

    let uri = Url::parse("file:///tmp/doc.rs").unwrap();

    docs.did_open(uri.clone(), "rust".into(), "fn main() {}".into())
        .await?;

    let change = TextDocumentContentChangeEvent {
        range: None,
        range_length: None,
        text: "fn main() { let x = 1; }".into(),
    };
    docs.did_change(uri.clone(), vec![change]).await?;

    docs.did_save(uri.clone()).await?;
    docs.did_close(uri.clone()).await?;

    // Allow child to process and then terminate
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    let mut guard = client.lock().await;
    let _ = guard.process.kill();

    let log = std::fs::read_to_string(log_path)?;
    let methods: Vec<_> = log.lines().collect();
    assert_eq!(
        methods,
        vec![
            "textDocument/didOpen",
            "textDocument/didChange",
            "textDocument/didSave",
            "textDocument/didClose",
        ]
    );

    Ok(())
}
