use std::{env, fs, path::PathBuf};

use anyhow::{Context, Result};
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum BackendKind {
    Ollama,
    Anthropic,
    Huggingface,
    Openai,
}

#[derive(Debug, Deserialize, Clone, PartialEq, Eq, Default)]
pub struct AiAuth {
    #[serde(default)]
    pub api_key: Option<String>,
}

#[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
pub struct AiConfig {
    pub backend: BackendKind,
    pub model: String,
    #[serde(default)]
    pub endpoint: Option<String>,
    #[serde(default)]
    pub auth: AiAuth,
}

#[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
pub struct CompletionConfig {
    #[serde(default = "default_debounce_ms")]
    pub debounce_ms: u64,
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
    #[serde(default = "default_stop_at_lines")]
    pub stop_at_lines: u32,
}

fn default_debounce_ms() -> u64 {
    300
}
fn default_max_tokens() -> u32 {
    128
}
fn default_stop_at_lines() -> u32 {
    10
}

#[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
pub struct IndexConfig {
    #[serde(default = "default_index_enabled")]
    pub enabled: bool,
    #[serde(default = "default_chunk_size")]
    pub chunk_size: u32,
    #[serde(default = "default_top_k")]
    pub top_k: u32,
    #[serde(default = "default_persist")]
    pub persist: bool,
}

fn default_index_enabled() -> bool {
    true
}
fn default_chunk_size() -> u32 {
    512
}
fn default_top_k() -> u32 {
    8
}
fn default_persist() -> bool {
    true
}

#[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
pub struct EditorConfig {
    #[serde(default = "default_tab_width")]
    pub tab_width: u8,
    #[serde(default = "default_insert_tabs")]
    pub insert_tabs: bool,
    #[serde(default = "default_theme")]
    pub theme: String,
}

fn default_tab_width() -> u8 {
    4
}
fn default_insert_tabs() -> bool {
    false
}
fn default_theme() -> String {
    "idep-dark".into()
}

#[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
pub struct Config {
    pub ai: AiConfig,
    #[serde(default)]
    pub completion: CompletionConfig,
    #[serde(default)]
    pub index: IndexConfig,
    #[serde(default)]
    pub editor: EditorConfig,
}

impl Default for CompletionConfig {
    fn default() -> Self {
        Self {
            debounce_ms: default_debounce_ms(),
            max_tokens: default_max_tokens(),
            stop_at_lines: default_stop_at_lines(),
        }
    }
}

impl Default for IndexConfig {
    fn default() -> Self {
        Self {
            enabled: default_index_enabled(),
            chunk_size: default_chunk_size(),
            top_k: default_top_k(),
            persist: default_persist(),
        }
    }
}

impl Default for EditorConfig {
    fn default() -> Self {
        Self {
            tab_width: default_tab_width(),
            insert_tabs: default_insert_tabs(),
            theme: default_theme(),
        }
    }
}

pub fn load_config(explicit_path: Option<PathBuf>) -> Result<Config> {
    let path = match explicit_path {
        Some(p) => p,
        None => default_config_path().context("failed to resolve config path")?,
    };

    let contents = fs::read_to_string(&path)
        .with_context(|| format!("failed to read config file at {}", path.display()))?;

    let mut cfg: Config = toml::from_str(&contents)
        .with_context(|| format!("failed to parse config file at {}", path.display()))?;

    // Env fallback for API key
    if cfg.ai.auth.api_key.is_none() {
        if let Ok(val) = env::var("IDEP_API_KEY") {
            if !val.is_empty() {
                cfg.ai.auth.api_key = Some(val);
            }
        }
    }

    Ok(cfg)
}

fn default_config_path() -> Option<PathBuf> {
    if let Some(mut p) = dirs::config_dir() {
        p.push("idep/config.toml");
        if p.exists() {
            return Some(p);
        }
    }

    if let Some(home) = dirs::home_dir() {
        let p = home.join(".idep/config.toml");
        if p.exists() {
            return Some(p);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir(prefix: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let mut p = env::temp_dir();
        p.push(format!("{prefix}-{nanos}"));
        fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn loads_with_env_api_key_fallback() {
        let dir = temp_dir("idep-config-explicit");
        let path = dir.join("config.toml");
        fs::write(
            &path,
            r#"
[ai]
backend = "openai"
model = "gpt-4o-mini"

[completion]
debounce_ms = 400
"#,
        )
        .unwrap();

        env::set_var("IDEP_API_KEY", "from-env");
        let cfg = load_config(Some(path)).expect("config should load");
        env::remove_var("IDEP_API_KEY");

        assert_eq!(cfg.ai.backend, BackendKind::Openai);
        assert_eq!(cfg.ai.model, "gpt-4o-mini");
        assert_eq!(cfg.ai.auth.api_key.as_deref(), Some("from-env"));
        assert_eq!(cfg.completion.debounce_ms, 400);
    }

    #[test]
    fn resolves_xdg_config_path() {
        let root = temp_dir("idep-config-xdg");
        let config_dir = root.join("idep");
        fs::create_dir_all(&config_dir).unwrap();
        let path = config_dir.join("config.toml");
        fs::write(
            &path,
            r#"
[ai]
backend = "ollama"
model = "codellama:13b"
endpoint = "http://localhost:11434"
"#,
        )
        .unwrap();

        env::set_var("XDG_CONFIG_HOME", &root);
        let cfg = load_config(None).expect("config should load via XDG_CONFIG_HOME");
        env::remove_var("XDG_CONFIG_HOME");

        assert_eq!(cfg.ai.backend, BackendKind::Ollama);
        assert_eq!(cfg.ai.model, "codellama:13b");
        assert_eq!(cfg.ai.endpoint.as_deref(), Some("http://localhost:11434"));
    }
}
