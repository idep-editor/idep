use anyhow::{bail, Context, Result};
use lsp_server::Message;
use std::io::{BufRead, BufReader, Read, Write};
use std::process::{ChildStdin, ChildStdout};
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};
use tokio::sync::Mutex;

/// Minimal JSON-RPC transport over stdio (LSP wire format).
/// Handles Content-Length framing and request ID generation.
pub struct JsonRpcTransport {
    writer: Arc<Mutex<ChildStdin>>,
    reader: Arc<Mutex<BufReader<ChildStdout>>>,
    request_id: AtomicU64,
}

impl JsonRpcTransport {
    pub fn new(stdin: ChildStdin, stdout: ChildStdout) -> Self {
        Self {
            writer: Arc::new(Mutex::new(stdin)),
            reader: Arc::new(Mutex::new(BufReader::new(stdout))),
            request_id: AtomicU64::new(1),
        }
    }

    pub fn next_id(&self) -> u64 {
        self.request_id.fetch_add(1, Ordering::SeqCst)
    }

    pub async fn send(&self, msg: &Message) -> Result<()> {
        let json = serde_json::to_vec(msg)?;
        let mut writer = self.writer.lock().await;
        write!(writer, "Content-Length: {}\r\n\r\n", json.len())?;
        writer.write_all(&json)?;
        writer.flush()?;
        Ok(())
    }

    pub async fn read_message(&self) -> Result<Message> {
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
