use anyhow::{anyhow, bail, Context, Result};
use lsp_server::{Message, Notification, Request, RequestId};
use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::process::{ChildStdin, ChildStdout};
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};
use tokio::sync::{broadcast, mpsc, oneshot, Mutex};

type PendingMap = Arc<Mutex<HashMap<RequestId, oneshot::Sender<Result<Option<Value>>>>>>;

/// Minimal JSON-RPC transport over stdio (LSP wire format).
/// Handles Content-Length framing and request ID generation.
pub struct JsonRpcTransport {
    writer: Arc<Mutex<ChildStdin>>,
    reader: Arc<Mutex<BufReader<ChildStdout>>>,
    request_id: Arc<AtomicU64>,
    pending: PendingMap,
    notifications: broadcast::Sender<Message>,
    outgoing: mpsc::Sender<Message>,
}

impl Clone for JsonRpcTransport {
    fn clone(&self) -> Self {
        Self {
            writer: self.writer.clone(),
            reader: self.reader.clone(),
            request_id: self.request_id.clone(),
            pending: self.pending.clone(),
            notifications: self.notifications.clone(),
            outgoing: self.outgoing.clone(),
        }
    }
}

impl JsonRpcTransport {
    pub fn new(stdin: ChildStdin, stdout: ChildStdout) -> Self {
        let (notifications, _) = broadcast::channel(32);
        let (outgoing, mut outgoing_rx) = mpsc::channel(32);

        let transport = Self {
            writer: Arc::new(Mutex::new(stdin)),
            reader: Arc::new(Mutex::new(BufReader::new(stdout))),
            request_id: Arc::new(AtomicU64::new(1)),
            pending: Arc::new(Mutex::new(HashMap::new())),
            notifications,
            outgoing,
        };

        // Writer loop: drain queued messages and write with framing
        let writer_clone = transport.clone();
        tokio::spawn(async move {
            while let Some(msg) = outgoing_rx.recv().await {
                let _ = writer_clone.write_framed(&msg).await;
            }
        });

        // Reader loop: decode messages and dispatch
        let reader_clone = transport.clone();
        tokio::spawn(async move {
            loop {
                match reader_clone.read_message().await {
                    Ok(msg) => match msg {
                        Message::Response(resp) => {
                            let mut pending = reader_clone.pending.lock().await;
                            if let Some(tx) = pending.remove(&resp.id) {
                                let result = if let Some(err) = resp.error {
                                    Err(anyhow!("LSP error {:?}: {:?}", resp.id, err))
                                } else {
                                    Ok(resp.result)
                                };
                                let _ = tx.send(result);
                            } else {
                                let _ = reader_clone.notifications.send(Message::Response(resp));
                            }
                        }
                        Message::Notification(_) | Message::Request(_) => {
                            let _ = reader_clone.notifications.send(msg);
                        }
                    },
                    Err(err) => {
                        let err_msg = err.to_string();
                        {
                            let mut pending = reader_clone.pending.lock().await;
                            for (_, tx) in pending.drain() {
                                let _ = tx.send(Err(anyhow!(err_msg.clone())));
                            }
                        }
                        let _ = reader_clone.notifications.send(Message::Notification(
                            Notification::new(
                                "transport/error".to_string(),
                                serde_json::json!({ "message": err_msg }),
                            ),
                        ));
                        break;
                    }
                }
            }
        });

        transport
    }

    pub fn next_id(&self) -> u64 {
        self.request_id.fetch_add(1, Ordering::SeqCst)
    }

    /// Enqueue a raw JSON-RPC message for sending.
    pub async fn send_message(&self, msg: Message) -> Result<()> {
        self.outgoing
            .send(msg)
            .await
            .map_err(|e| anyhow!("failed to enqueue message: {}", e))
    }

    /// Send a JSON-RPC request and wait for its response.
    pub async fn send_request<P: Serialize>(
        &self,
        method: &str,
        params: P,
    ) -> Result<Option<Value>> {
        let id = self.next_id();
        let req = Request::new(RequestId::from(id as i32), method.to_string(), params);
        let (tx, rx) = oneshot::channel();
        self.pending
            .lock()
            .await
            .insert(RequestId::from(id as i32), tx);
        self.send_message(Message::Request(req)).await?;
        rx.await?
    }

    /// Send a JSON-RPC notification (fire-and-forget).
    pub async fn send_notification<P: Serialize>(&self, method: &str, params: P) -> Result<()> {
        let notif = Notification::new(method.to_string(), serde_json::to_value(params)?);
        self.send_message(Message::Notification(notif)).await
    }

    /// Subscribe to incoming notifications/requests/responses not claimed by pending requests.
    pub fn subscribe_notifications(&self) -> broadcast::Receiver<Message> {
        self.notifications.subscribe()
    }

    async fn write_framed(&self, msg: &Message) -> Result<()> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::{Command, Stdio};

    fn spawn_echoing_server(script: &str) -> (JsonRpcTransport, std::process::Child) {
        let mut child = Command::new("python3")
            .args(["-u", "-c", script])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .expect("failed to spawn python test server");

        let stdin = child.stdin.take().expect("child stdin");
        let stdout = child.stdout.take().expect("child stdout");

        (JsonRpcTransport::new(stdin, stdout), child)
    }

    #[tokio::test]
    async fn round_trip_request_response_and_notification() {
        let script = r#"
import sys, json

def read_msg():
    while True:
        line = sys.stdin.readline()
        if not line:
            return None
        if not line.lower().startswith("content-length:"):
            continue
        length = int(line.split(":",1)[1].strip())
        sys.stdin.readline()
        body = sys.stdin.read(length)
        return json.loads(body)

msg = read_msg()

notif = {"jsonrpc":"2.0","method":"test/notify","params":{"note":1}}
notif_str = json.dumps(notif)
sys.stdout.write(f"Content-Length: {len(notif_str)}\r\n\r\n{notif_str}")

resp = {"jsonrpc":"2.0","id": msg["id"], "result": {"echo": msg.get("params")}}
resp_str = json.dumps(resp)
sys.stdout.write(f"Content-Length: {len(resp_str)}\r\n\r\n{resp_str}")
sys.stdout.flush()
"#;

        let (transport, mut child) = spawn_echoing_server(script);
        let mut notif_rx = transport.subscribe_notifications();

        let response = transport
            .send_request("test/echo", serde_json::json!({"foo": "bar"}))
            .await
            .expect("request should succeed")
            .expect("response result should exist");

        assert_eq!(response["echo"]["foo"], "bar");

        // Notification should be dispatched fire-and-forget
        let notif_msg = notif_rx.recv().await.expect("notification");
        match notif_msg {
            Message::Notification(notif) => {
                assert_eq!(notif.method, "test/notify");
            }
            _ => panic!("expected notification"),
        }

        let _ = child.kill();
        let _ = child.wait();
    }

    #[tokio::test]
    async fn handles_malformed_message_gracefully() {
        let script = r#"
import sys

def read_msg():
    while True:
        line = sys.stdin.readline()
        if not line:
            return None
        if line.lower().startswith("content-length:"):
            length = int(line.split(":",1)[1].strip())
            sys.stdin.readline()
            return sys.stdin.read(length)

_ = read_msg()
# respond with invalid JSON body but correct framing
bad_body = "not-json"
sys.stdout.write(f"Content-Length: {len(bad_body)}\r\n\r\n{bad_body}")
sys.stdout.flush()
"#;

        let (transport, mut child) = spawn_echoing_server(script);

        let result = transport
            .send_request("test/bad", serde_json::json!({}))
            .await;

        assert!(result.is_err(), "malformed response should produce error");

        let _ = child.kill();
        let _ = child.wait();
    }
}
