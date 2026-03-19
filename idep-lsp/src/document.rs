use anyhow::Result;
use lsp_types::notification::{
    DidChangeTextDocument, DidCloseTextDocument, DidOpenTextDocument, DidSaveTextDocument,
    Notification,
};
use lsp_types::{
    Diagnostic, Location, Position, TextDocumentContentChangeEvent, TextDocumentIdentifier,
    TextDocumentItem, Url, VersionedTextDocumentIdentifier,
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

    pub fn handle_publish_diagnostics(&mut self, mut params: PublishDiagnosticsParams) {
        params.uri = to_server_uri(&params.uri);
        self.diagnostics.handle_publish_diagnostics(params);
    }

    pub fn get_diagnostics(&self, uri: &Url) -> &[Diagnostic] {
        let server_uri = to_server_uri(uri);
        self.diagnostics.get_diagnostics(&server_uri)
    }

    pub async fn did_open(&mut self, uri: Url, language_id: String, text: String) -> Result<()> {
        let version = 1;
        let server_uri = to_server_uri(&uri);
        let item = TextDocumentItem {
            uri: server_uri.clone(),
            language_id: language_id.clone(),
            version,
            text: text.clone(),
        };

        self.open_documents.insert(uri.clone(), item.clone());
        self.versions.insert(uri.clone(), version);

        let params = DidOpenTextDocumentParams {
            text_document: item,
        };

        let mut client = self.client.lock().await;
        client.notify(DidOpenTextDocument::METHOD, params).await
    }

    pub async fn did_change(
        &mut self,
        uri: Url,
        changes: Vec<TextDocumentContentChangeEvent>,
    ) -> Result<()> {
        let next_version = self.versions.get(&uri).copied().unwrap_or(0) + 1;
        self.versions.insert(uri.clone(), next_version);

        let server_uri = to_server_uri(&uri);

        let identifier = VersionedTextDocumentIdentifier {
            uri: server_uri,
            version: next_version,
        };

        let params = DidChangeTextDocumentParams {
            text_document: identifier,
            content_changes: changes,
        };

        let mut client = self.client.lock().await;
        client.notify(DidChangeTextDocument::METHOD, params).await
    }

    pub async fn did_save(&mut self, uri: Url) -> Result<()> {
        let server_uri = to_server_uri(&uri);
        let params = DidSaveTextDocumentParams {
            text_document: TextDocumentIdentifier { uri: server_uri },
            text: None,
        };

        let mut client = self.client.lock().await;
        client.notify(DidSaveTextDocument::METHOD, params).await
    }

    pub async fn did_close(&mut self, uri: Url) -> Result<()> {
        self.open_documents.remove(&uri);
        self.versions.remove(&uri);
        self.diagnostics.clear(&to_server_uri(&uri));

        let server_uri = to_server_uri(&uri);

        let params = DidCloseTextDocumentParams {
            text_document: TextDocumentIdentifier { uri: server_uri },
        };

        let mut client = self.client.lock().await;
        client.notify(DidCloseTextDocument::METHOD, params).await
    }

    pub async fn hover_text(&self, uri: Url, position: Position) -> Result<Option<String>> {
        let server_uri = to_server_uri(&uri);
        let mut client = self.client.lock().await;
        client.hover_text(server_uri, position).await
    }

    pub async fn goto_definition_locations(
        &self,
        uri: Url,
        position: Position,
    ) -> Result<Vec<Location>> {
        let server_uri = to_server_uri(&uri);
        let mut client = self.client.lock().await;
        client.goto_definition_locations(server_uri, position).await
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

    #[tokio::test]
    async fn diagnostics_fields_are_preserved() {
        let dummy = LspClient::spawn("echo", &["test"]).expect("spawn dummy");
        let client = Arc::new(Mutex::new(dummy));
        let mut docs = DocumentManager::new(client);

        let uri = Url::parse("file:///tmp/test.rs").unwrap();
        let params = PublishDiagnosticsParams {
            uri: uri.clone(),
            version: Some(1),
            diagnostics: vec![Diagnostic {
                range: lsp_types::Range {
                    start: lsp_types::Position {
                        line: 2,
                        character: 4,
                    },
                    end: lsp_types::Position {
                        line: 2,
                        character: 9,
                    },
                },
                severity: Some(lsp_types::DiagnosticSeverity::WARNING),
                code: Some(lsp_types::NumberOrString::String("E100".into())),
                code_description: None,
                source: Some("test-lsp".into()),
                message: "unexpected token".into(),
                related_information: None,
                tags: None,
                data: None,
            }],
        };

        docs.handle_publish_diagnostics(params);
        let diags = docs.get_diagnostics(&uri);
        assert_eq!(diags.len(), 1);
        let diag = &diags[0];
        assert_eq!(diag.message, "unexpected token");
        assert_eq!(diag.severity, Some(lsp_types::DiagnosticSeverity::WARNING));
        assert_eq!(
            diag.code,
            Some(lsp_types::NumberOrString::String("E100".into()))
        );
        assert_eq!(diag.range.start.line, 2);
        assert_eq!(diag.range.start.character, 4);
        assert_eq!(diag.range.end.line, 2);
        assert_eq!(diag.range.end.character, 9);
    }

    #[tokio::test]
    async fn did_close_clears_diagnostics() {
        let dummy = LspClient::spawn("sleep", &["1"]).expect("spawn dummy");
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
                message: "err".into(),
                related_information: None,
                tags: None,
                data: None,
            }],
        };

        docs.handle_publish_diagnostics(params);
        assert_eq!(docs.get_diagnostics(&uri).len(), 1);

        docs.did_close(uri.clone()).await.expect("did_close");
        assert!(docs.get_diagnostics(&uri).is_empty());
    }
}
