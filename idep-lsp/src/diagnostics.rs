use lsp_types::{Diagnostic, PublishDiagnosticsParams, Url};
use std::collections::HashMap;

/// Stores diagnostics per document URI.
pub struct DiagnosticsManager {
    diagnostics: HashMap<Url, Vec<Diagnostic>>,
}

impl DiagnosticsManager {
    pub fn new() -> Self {
        Self {
            diagnostics: HashMap::new(),
        }
    }

    pub fn handle_publish_diagnostics(&mut self, params: PublishDiagnosticsParams) {
        self.diagnostics
            .insert(params.uri.clone(), params.diagnostics);
    }

    pub fn get_diagnostics(&self, uri: &Url) -> &[Diagnostic] {
        self.diagnostics
            .get(uri)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    pub fn clear(&mut self, uri: &Url) {
        self.diagnostics.remove(uri);
    }
}

impl Default for DiagnosticsManager {
    fn default() -> Self {
        Self::new()
    }
}
