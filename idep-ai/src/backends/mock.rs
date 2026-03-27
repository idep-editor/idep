// mock — simple backend for testing

use crate::backends::{Backend, BackendInfo};
use anyhow::Result;
use async_trait::async_trait;
use std::any::Any;

/// Mock backend for testing
pub struct MockBackend {
    pub response: String,
}

impl Default for MockBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl MockBackend {
    pub fn new() -> Self {
        Self {
            response: "Mock response".to_string(),
        }
    }

    pub fn with_response(response: impl Into<String>) -> Self {
        Self {
            response: response.into(),
        }
    }
}

#[async_trait]
impl Backend for MockBackend {
    async fn complete(&self, _prompt: &str, _max_tokens: u32) -> Result<String> {
        Ok(self.response.clone())
    }

    fn name(&self) -> &str {
        "Mock"
    }

    fn info(&self) -> BackendInfo {
        BackendInfo {
            name: "Mock",
            version: Some("1.0.0".to_string()),
            endpoint: "mock://test".to_string(),
            cloud_dependent: false,
            requires_auth: false,
        }
    }

    fn as_any(&self) -> &(dyn Any + Send + Sync) {
        self
    }
}
