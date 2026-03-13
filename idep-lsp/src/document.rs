use anyhow::Result;
use lsp_types::notification::{
    DidChangeTextDocument, DidCloseTextDocument, DidOpenTextDocument, DidSaveTextDocument,
    Notification,
};
use lsp_types::{
    DidChangeTextDocumentParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams,
    DidSaveTextDocumentParams,
};
use lsp_types::{
    TextDocumentContentChangeEvent, TextDocumentIdentifier, TextDocumentItem, Url,
    VersionedTextDocumentIdentifier,
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::client::LspClient;
use crate::path::to_server_uri;

/// Tracks open documents and proxies LSP textDocument notifications.
pub struct DocumentManager {
    pub client: Arc<Mutex<LspClient>>,
    pub open_documents: HashMap<Url, TextDocumentItem>,
    pub versions: HashMap<Url, i32>,
}

impl DocumentManager {
    pub fn new(client: Arc<Mutex<LspClient>>) -> Self {
        Self {
            client,
            open_documents: HashMap::new(),
            versions: HashMap::new(),
        }
    }

    pub async fn did_open(&mut self, _uri: Url, _language_id: String, _text: String) -> Result<()> {
        let version = 1;
        let server_uri = to_server_uri(&_uri);
        let item = TextDocumentItem {
            uri: server_uri.clone(),
            language_id: _language_id.clone(),
            version,
            text: _text.clone(),
        };

        self.open_documents.insert(_uri.clone(), item.clone());
        self.versions.insert(_uri.clone(), version);

        let params = DidOpenTextDocumentParams {
            text_document: item,
        };

        let mut client = self.client.lock().await;
        client.notify(DidOpenTextDocument::METHOD, params).await
    }

    pub async fn did_change(
        &mut self,
        _uri: Url,
        _changes: Vec<TextDocumentContentChangeEvent>,
    ) -> Result<()> {
        let next_version = self.versions.get(&_uri).copied().unwrap_or(0) + 1;
        self.versions.insert(_uri.clone(), next_version);

        let server_uri = to_server_uri(&_uri);

        let identifier = VersionedTextDocumentIdentifier {
            uri: server_uri,
            version: next_version,
        };

        let params = DidChangeTextDocumentParams {
            text_document: identifier,
            content_changes: _changes,
        };

        let mut client = self.client.lock().await;
        client.notify(DidChangeTextDocument::METHOD, params).await
    }

    pub async fn did_save(&mut self, _uri: Url) -> Result<()> {
        let server_uri = to_server_uri(&_uri);
        let params = DidSaveTextDocumentParams {
            text_document: TextDocumentIdentifier { uri: server_uri },
            text: None,
        };

        let mut client = self.client.lock().await;
        client.notify(DidSaveTextDocument::METHOD, params).await
    }

    pub async fn did_close(&mut self, _uri: Url) -> Result<()> {
        self.open_documents.remove(&_uri);
        self.versions.remove(&_uri);

        let server_uri = to_server_uri(&_uri);

        let params = DidCloseTextDocumentParams {
            text_document: TextDocumentIdentifier { uri: server_uri },
        };

        let mut client = self.client.lock().await;
        client.notify(DidCloseTextDocument::METHOD, params).await
    }
}
