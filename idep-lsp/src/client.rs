use anyhow::{bail, Context, Result};
use lsp_server::{Message, Notification, Request, RequestId};
use lsp_types::notification::{Exit, Initialized, Notification as LspNotification};
use lsp_types::request::{
    Completion, GotoDefinition, HoverRequest, Initialize, Request as LspRequest, Shutdown,
};
use lsp_types::{
    ClientCapabilities, CompletionContext, CompletionParams, CompletionResponse,
    GotoDefinitionParams, GotoDefinitionResponse, Hover, HoverParams, InitializeParams,
    InitializeResult, Position, TextDocumentIdentifier, TextDocumentPositionParams, Url,
    WorkDoneProgressParams,
};
use serde::Serialize;
use serde_json::Value;
use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::time::timeout;

/// Basic LSP client process wrapper.
///
/// Manages LSP server subprocess with stdio pipes, JSON-RPC transport,
/// and lifecycle management (initialize, shutdown, force-kill).
pub struct LspClient {
    pub process: Child,
    pub request_id: AtomicU64,
    writer: Arc<Mutex<ChildStdin>>,
    reader: Arc<Mutex<BufReader<ChildStdout>>>,
    pub stderr_output: Arc<Mutex<Vec<String>>>,
    pub shutdown_timeout: Duration,
    initialize_result: Arc<Mutex<Option<InitializeResult>>>,
}

impl LspClient {
    /// Spawn an LSP server process with stdio pipes.
    /// Captures stdout for LSP protocol and stderr separately for logging.
    pub fn spawn(command: &str, args: &[&str]) -> Result<Self> {
        Self::spawn_with_timeout(command, args, Duration::from_secs(5))
    }

    /// Spawn with configurable shutdown timeout.
    pub fn spawn_with_timeout(
        command: &str,
        args: &[&str],
        shutdown_timeout: Duration,
    ) -> Result<Self> {
        let mut child = Command::new(command)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .with_context(|| format!("Failed to spawn language server: {}", command))?;

        let child_stdin = child
            .stdin
            .take()
            .ok_or_else(|| anyhow::anyhow!("Failed to open child stdin"))?;
        let child_stdout = child
            .stdout
            .take()
            .ok_or_else(|| anyhow::anyhow!("Failed to open child stdout"))?;
        let child_stderr = child
            .stderr
            .take()
            .ok_or_else(|| anyhow::anyhow!("Failed to open child stderr"))?;

        let stderr_output = Arc::new(Mutex::new(Vec::new()));
        let stderr_output_clone = stderr_output.clone();

        std::thread::spawn(move || {
            let reader = BufReader::new(child_stderr);
            for line in reader.lines().map_while(Result::ok) {
                let stderr_vec = stderr_output_clone.blocking_lock();
                drop(stderr_vec);
                let mut stderr_vec = stderr_output_clone.blocking_lock();
                stderr_vec.push(line);
            }
        });

        Ok(Self {
            process: child,
            request_id: AtomicU64::new(1),
            writer: Arc::new(Mutex::new(child_stdin)),
            reader: Arc::new(Mutex::new(BufReader::new(child_stdout))),
            stderr_output,
            shutdown_timeout,
            initialize_result: Arc::new(Mutex::new(None)),
        })
    }

    /// Initialize handshake with the language server.
    pub async fn initialize(&mut self, root_uri: lsp_types::Url) -> Result<InitializeResult> {
        let id = self.next_id();
        let params = Self::build_initialize_params(root_uri.clone());

        let req = Request::new(
            RequestId::from(id as i32),
            Initialize::METHOD.to_string(),
            params,
        );
        self.send(Message::Request(req)).await?;
        let result = self.wait_for_response(id).await.and_then(|val| {
            let result: InitializeResult = serde_json::from_value(val.unwrap_or_default())
                .context("Failed to decode initialize result")?;
            Ok(result)
        })?;

        let mut stored = self.initialize_result.lock().await;
        *stored = Some(result.clone());

        Ok(result)
    }

    pub async fn initialized(&mut self) -> Result<()> {
        let notif = Notification::new(Initialized::METHOD.to_string(), serde_json::Value::Null);
        self.send(Message::Notification(notif)).await
    }

    pub async fn initialize_result(&self) -> Option<InitializeResult> {
        self.initialize_result.lock().await.clone()
    }

    fn build_initialize_params(root_uri: Url) -> InitializeParams {
        #[allow(deprecated)]
        InitializeParams {
            process_id: None,
            client_info: Some(lsp_types::ClientInfo {
                name: "idep-lsp".into(),
                version: Some(env!("CARGO_PKG_VERSION").into()),
            }),
            root_path: None,
            root_uri: Some(root_uri.clone()),
            initialization_options: None,
            capabilities: ClientCapabilities::default(),
            trace: None,
            workspace_folders: Some(vec![lsp_types::WorkspaceFolder {
                uri: root_uri,
                name: "workspace".into(),
            }]),
            locale: None,
            work_done_progress_params: WorkDoneProgressParams {
                work_done_token: None,
            },
        }
    }

    pub async fn shutdown(&mut self) -> Result<()> {
        let id = self.next_id();
        let req = Request::new(
            RequestId::from(id as i32),
            Shutdown::METHOD.to_string(),
            serde_json::Value::Null,
        );
        self.send(Message::Request(req)).await?;

        if timeout(self.shutdown_timeout, self.wait_for_response(id))
            .await
            .is_err()
        {
            eprintln!("Shutdown timeout, force-killing process");
            let _ = self.process.kill();
            return Ok(());
        }

        // Send exit notification
        let exit_notif = Notification::new(Exit::METHOD.to_string(), serde_json::Value::Null);
        let _ = self.send(Message::Notification(exit_notif)).await;

        if timeout(Duration::from_secs(1), async {
            let _ = self.process.wait();
        })
        .await
        .is_err()
        {
            let _ = self.process.kill();
        }
        Ok(())
    }

    /// Send a JSON-RPC request.
    pub async fn request<P: Serialize>(&mut self, method: &str, params: P) -> Result<Value> {
        let id = self.next_id();
        let req = Request::new(RequestId::from(id as i32), method.to_string(), params);
        self.send(Message::Request(req)).await?;
        self.wait_for_response(id)
            .await
            .map(|v| v.unwrap_or(Value::Null))
    }

    /// Send a JSON-RPC notification (no response expected).
    pub async fn notify<P: Serialize>(&mut self, method: &str, params: P) -> Result<()> {
        let notif = Notification::new(method.to_string(), serde_json::to_value(params)?);
        self.send(Message::Notification(notif)).await
    }

    /// textDocument/hover helper.
    pub async fn hover(&mut self, uri: Url, position: Position) -> Result<Option<Hover>> {
        let params = HoverParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri },
                position,
            },
            work_done_progress_params: WorkDoneProgressParams {
                work_done_token: None,
            },
        };
        let val = self.request(HoverRequest::METHOD, params).await?;
        if val.is_null() {
            return Ok(None);
        }
        let hover: Hover =
            serde_json::from_value(val).context("Failed to decode hover response")?;
        Ok(Some(hover))
    }

    /// textDocument/definition helper.
    pub async fn goto_definition(
        &mut self,
        uri: Url,
        position: Position,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let params = GotoDefinitionParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri },
                position,
            },
            work_done_progress_params: WorkDoneProgressParams {
                work_done_token: None,
            },
            partial_result_params: Default::default(),
        };
        let val = self.request(GotoDefinition::METHOD, params).await?;
        if val.is_null() {
            return Ok(None);
        }
        let resp: GotoDefinitionResponse =
            serde_json::from_value(val).context("Failed to decode goto definition response")?;
        Ok(Some(resp))
    }

    /// textDocument/completion helper.
    pub async fn completion(
        &mut self,
        uri: Url,
        position: Position,
        context: Option<CompletionContext>,
    ) -> Result<Option<CompletionResponse>> {
        let params = CompletionParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri },
                position,
            },
            work_done_progress_params: WorkDoneProgressParams {
                work_done_token: None,
            },
            partial_result_params: Default::default(),
            context,
        };

        let val = self.request(Completion::METHOD, params).await?;
        if val.is_null() {
            return Ok(None);
        }
        let resp: CompletionResponse =
            serde_json::from_value(val).context("Failed to decode completion response")?;
        Ok(Some(resp))
    }

    /// Simple ranking: shorter labels first, then lexicographic.
    pub fn rank_completions(
        items: Vec<lsp_types::CompletionItem>,
    ) -> Vec<lsp_types::CompletionItem> {
        let mut dedup: std::collections::HashMap<String, lsp_types::CompletionItem> =
            std::collections::HashMap::new();
        for item in items {
            dedup.entry(item.label.clone()).or_insert(item);
        }
        let mut vals: Vec<_> = dedup.into_values().collect();
        vals.sort_by(|a, b| {
            let la = a.label.len();
            let lb = b.label.len();
            la.cmp(&lb).then_with(|| a.label.cmp(&b.label))
        });
        vals
    }

    /// Attempt to restart the LSP server with exponential backoff.
    /// Max 3 retries with delays: 1s, 2s, 4s.
    pub async fn restart_with_backoff(command: &str, args: &[&str]) -> Result<Self> {
        let mut delay_ms = 1000u64;
        for attempt in 1..=3 {
            match Self::spawn(command, args) {
                Ok(client) => {
                    if attempt > 1 {
                        eprintln!("LSP server restarted successfully on attempt {}", attempt);
                    }
                    return Ok(client);
                }
                Err(e) => {
                    if attempt == 3 {
                        return Err(e).context("Failed to restart LSP server after 3 attempts");
                    }
                    eprintln!(
                        "Restart attempt {} failed: {}, retrying in {}ms",
                        attempt, e, delay_ms
                    );
                    tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                    delay_ms *= 2;
                }
            }
        }
        bail!("Failed to restart LSP server after 3 attempts")
    }

    fn next_id(&self) -> u64 {
        self.request_id.fetch_add(1, Ordering::SeqCst)
    }

    async fn send(&self, msg: Message) -> Result<()> {
        let json = serde_json::to_vec(&msg)?;
        let mut writer = self.writer.lock().await;
        write!(writer, "Content-Length: {}\r\n\r\n", json.len())?;
        writer.write_all(&json)?;
        writer.flush()?;
        Ok(())
    }

    async fn wait_for_response(&self, id: u64) -> Result<Option<Value>> {
        loop {
            let msg = self.read_message().await?;
            if let Message::Response(resp) = msg {
                if resp.id == RequestId::from(id as i32) {
                    if let Some(err) = resp.error {
                        bail!("LSP error {}: {:?}", id, err);
                    }
                    return Ok(resp.result);
                }
            }
        }
    }

    async fn read_message(&self) -> Result<Message> {
        let mut reader = self.reader.lock().await;
        let mut header = String::new();

        // Read headers until Content-Length then body
        loop {
            header.clear();
            let bytes = reader.read_line(&mut header)?;
            if bytes == 0 {
                bail!("LSP server closed the stream");
            }
            if header.trim().is_empty() {
                continue;
            }
            if header.to_lowercase().starts_with("content-length:") {
                let len_str = header[15..].trim();
                let len: usize = len_str.parse().context("Invalid Content-Length")?;
                // Consume the blank line
                let mut blank = String::new();
                reader.read_line(&mut blank)?;
                let mut buf = vec![0u8; len];
                reader.read_exact(&mut buf)?;
                let msg: Message =
                    serde_json::from_slice(&buf).context("Failed to parse LSP message")?;
                return Ok(msg);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Stdio;

    #[tokio::test]
    async fn test_spawn_with_timeout() {
        let result = LspClient::spawn("echo", &["test"]);
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_shutdown_timeout_force_kill() {
        let mut client = LspClient::spawn("sleep", &["10"]).expect("Failed to spawn sleep");
        client.shutdown_timeout = Duration::from_millis(100);

        let result = client.shutdown().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_restart_with_backoff() {
        let result = LspClient::restart_with_backoff("echo", &["test"]).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_stderr_capture() {
        let client = LspClient::spawn("bash", &["-c", "echo 'test error' >&2; sleep 0.1"])
            .expect("Failed to spawn bash");

        tokio::time::sleep(Duration::from_millis(200)).await;

        let stderr = client.stderr_output.lock().await;
        assert!(!stderr.is_empty(), "stderr should capture output");
    }

    #[tokio::test]
    async fn test_next_id_increments() {
        let client = LspClient::spawn("echo", &["test"]).expect("Failed to spawn");
        let id1 = client.next_id();
        let id2 = client.next_id();
        assert_eq!(id1 + 1, id2);
    }

    #[tokio::test]
    #[ignore = "uses subprocess pipes; keep ignored to avoid CI flake"]
    async fn test_initialize_stores_result_and_sends_initialized() {
        let script = r#"
import sys, json

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

msg = read_msg()

body = b'{"jsonrpc":"2.0","id":1,"result":{"capabilities":{}}}'
header = f"Content-Length: {len(body)}\r\n\r\n".encode()
sys.stdout.buffer.write(header)
sys.stdout.buffer.write(body)
sys.stdout.buffer.flush()

# Expect initialized notification next; just consume it to keep pipes clean
_ = read_msg()
"#;

        let mut child = std::process::Command::new("python3")
            .args(["-u", "-c", script])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .expect("failed to spawn python test server");

        let stdin = child.stdin.take().expect("stdin");
        let stdout = child.stdout.take().expect("stdout");

        let mut client = LspClient {
            process: child,
            request_id: AtomicU64::new(1),
            writer: Arc::new(Mutex::new(stdin)),
            reader: Arc::new(Mutex::new(BufReader::new(stdout))),
            stderr_output: Arc::new(Mutex::new(Vec::new())),
            shutdown_timeout: Duration::from_millis(100),
            initialize_result: Arc::new(Mutex::new(None)),
        };

        let root_uri = Url::parse("file:///tmp").unwrap();
        let result = client
            .initialize(root_uri.clone())
            .await
            .expect("initialize");
        assert!(
            result.capabilities.text_document_sync.is_none(),
            "capabilities should be parsed"
        );

        client.initialized().await.expect("initialized");

        let stored = client.initialize_result().await;
        assert!(stored.is_some(), "initialize result should be stored");

        let _ = client.process.kill();
    }

    #[tokio::test]
    async fn test_initialize_shutdown_sequence() {
        let script = r#"
import sys, json

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

# initialize
init_msg = read_msg()
init_resp = {"jsonrpc": "2.0", "id": init_msg.get("id", 1), "result": {"capabilities": {}}}
body = json.dumps(init_resp).encode()
header = f"Content-Length: {len(body)}\r\n\r\n".encode()
sys.stdout.buffer.write(header + body)
sys.stdout.buffer.flush()

# shutdown
shutdown_msg = read_msg()
shutdown_resp = {"jsonrpc": "2.0", "id": shutdown_msg.get("id", 2), "result": None}
body = json.dumps(shutdown_resp).encode()
header = f"Content-Length: {len(body)}\r\n\r\n".encode()
sys.stdout.buffer.write(header + body)
sys.stdout.buffer.flush()

# exit notification (ignore)
_ = read_msg()

"#;

        let mut child = std::process::Command::new("python3")
            .args(["-u", "-c", script])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .expect("failed to spawn python test server");

        let stdin = child.stdin.take().expect("stdin");
        let stdout = child.stdout.take().expect("stdout");

        let mut client = LspClient {
            process: child,
            request_id: AtomicU64::new(1),
            writer: Arc::new(Mutex::new(stdin)),
            reader: Arc::new(Mutex::new(BufReader::new(stdout))),
            stderr_output: Arc::new(Mutex::new(Vec::new())),
            shutdown_timeout: Duration::from_secs(2),
            initialize_result: Arc::new(Mutex::new(None)),
        };

        let root_uri = Url::parse("file:///tmp").unwrap();
        let init = client
            .initialize(root_uri.clone())
            .await
            .expect("initialize");
        assert!(init.capabilities.text_document_sync.is_none());

        client.initialized().await.expect("initialized");

        client.shutdown().await.expect("shutdown should succeed");
    }

    #[tokio::test]
    async fn test_completion_parses_response() {
        let script = r#"
import sys, json

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

msg = read_msg()

resp = {
    "jsonrpc": "2.0",
    "id": msg.get("id", 1),
    "result": [
        {"label": "foo", "insertText": "foo"}
    ]
}

body = json.dumps(resp).encode()
header = f"Content-Length: {len(body)}\r\n\r\n".encode()
sys.stdout.buffer.write(header + body)
sys.stdout.buffer.flush()
"#;

        let mut client = LspClient::spawn("python3", &["-u", "-c", script]).expect("spawn python");

        let uri = Url::parse("file:///tmp/test.rs").unwrap();
        let position = Position {
            line: 0,
            character: 0,
        };

        let resp = client
            .completion(uri, position, None)
            .await
            .expect("completion")
            .expect("expected response");

        match resp {
            CompletionResponse::Array(items) => {
                assert_eq!(items.len(), 1);
                assert_eq!(items[0].label, "foo");
            }
            CompletionResponse::List(list) => {
                assert_eq!(list.items.len(), 1);
                assert_eq!(list.items[0].label, "foo");
            }
        }

        let _ = client.process.kill();
    }

    #[test]
    fn test_rank_completions_sorts_and_dedups() {
        let items = vec![
            lsp_types::CompletionItem {
                label: "beta".into(),
                ..Default::default()
            },
            lsp_types::CompletionItem {
                label: "alpha".into(),
                ..Default::default()
            },
            lsp_types::CompletionItem {
                label: "alpha".into(),
                ..Default::default()
            },
            lsp_types::CompletionItem {
                label: "a".into(),
                ..Default::default()
            },
        ];

        let ranked = LspClient::rank_completions(items);
        let labels: Vec<_> = ranked.iter().map(|i| i.label.as_str()).collect();
        // After dedup: "a" (len 1), "alpha" (len 5), "beta" (len 4)
        // Sorted by length then lexicographic: "a", "beta", "alpha"
        assert_eq!(labels, vec!["a", "beta", "alpha"]);
    }
}
