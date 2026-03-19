use anyhow::Result;
use lsp_types::notification::{
    DidChangeTextDocument, DidCloseTextDocument, DidOpenTextDocument, DidSaveTextDocument,
    Notification,
};
use lsp_types::{
    Diagnostic, TextDocumentContentChangeEvent, TextDocumentIdentifier, TextDocumentItem, Url,
    VersionedTextDocumentIdentifier,
};
use lsp_types::{
    DidChangeTextDocumentParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams,
    DidSaveTextDocumentParams, PublishDiagnosticsParams,
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::client::LspClient;
use crate::diagnostics::DiagnosticsManager;
use crate::path::to_server_uri;

/// Tracks open documents, diagnostics, and proxies LSP textDocument notifications.
pub struct DocumentManager {
    pub client: Arc<Mutex<LspClient>>,
    pub open_documents: HashMap<Url, TextDocumentItem>,
    pub versions: HashMap<Url, i32>,
    diagnostics: DiagnosticsManager,
}

impl DocumentManager {
    pub fn new(client: Arc<Mutex<LspClient>>) -> Self {
        Self {
            client,
            open_documents: HashMap::new(),
            versions: HashMap::new(),
            diagnostics: DiagnosticsManager::new(),
        }
    }

    pub fn handle_publish_diagnostics(&mut self, params: PublishDiagnosticsParams) {
        self.diagnostics.handle_publish_diagnostics(params);
    }

    pub fn get_diagnostics(&self, uri: &Url) -> &[Diagnostic] {
        self.diagnostics.get_diagnostics(uri)
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
        self.diagnostics.clear(&_uri);

        let server_uri = to_server_uri(&_uri);

        let params = DidCloseTextDocumentParams {
            text_document: TextDocumentIdentifier { uri: server_uri },
        };

        let mut client = self.client.lock().await;
        client.notify(DidCloseTextDocument::METHOD, params).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lsp_types::{Diagnostic, PublishDiagnosticsParams, Url};
    use std::sync::Arc;
    use tokio::sync::Mutex;

    // This just tests diagnostics storage behavior independent of LSP I/O.
    #[tokio::test]
    async fn stores_and_retrieves_diagnostics_for_uri() {
        let dummy = LspClient::spawn("echo", &["test"]).expect("spawn dummy");
        let client = Arc::new(Mutex::new(dummy));
        let mut docs = DocumentManager::new(client);

        let uri = Url::parse("file:///tmp/test.rs").unwrap();
        let params = PublishDiagnosticsParams {
            uri: uri.clone(),
            version: None,
            diagnostics: vec![Diagnostic {
                range: lsp_types::Range {
                    start: lsp_types::Position {
                        line: 0,
                        character: 0,
                    },
                    end: lsp_types::Position {
                        line: 0,
                        character: 1,
                    },
                },
                severity: Some(lsp_types::DiagnosticSeverity::ERROR),
                code: None,
                code_description: None,
                source: Some("test".into()),
                message: "error".into(),
                related_information: None,
                tags: None,
                data: None,
            }],
        };

        docs.handle_publish_diagnostics(params);
        assert_eq!(docs.get_diagnostics(&uri).len(), 1);
    }
}
