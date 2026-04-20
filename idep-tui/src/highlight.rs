use ratatui::style::{Color, Modifier, Style};
use std::path::Path;
use tree_sitter::{Parser, Query, QueryCursor};

/// A highlighted span with byte range and highlight name
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HighlightedSpan {
    /// Byte start position in the source
    pub start: usize,
    /// Byte end position in the source
    pub end: usize,
    /// Highlight name (e.g., "keyword", "string", "comment")
    pub highlight: String,
}

impl HighlightedSpan {
    pub fn new(start: usize, end: usize, highlight: impl Into<String>) -> Self {
        Self {
            start,
            end,
            highlight: highlight.into(),
        }
    }
}

/// Supported programming languages
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    Rust,
    TypeScript,
    Python,
    Toml,
    Markdown,
    Unknown,
}

impl Language {
    /// Detect language from file extension
    pub fn from_path(path: &Path) -> Self {
        match path.extension().and_then(|e| e.to_str()) {
            Some("rs") => Language::Rust,
            Some("ts") | Some("tsx") | Some("js") | Some("jsx") => Language::TypeScript,
            Some("py") => Language::Python,
            Some("toml") => Language::Toml,
            Some("md") => Language::Markdown,
            _ => Language::Unknown,
        }
    }

    /// Get the tree-sitter language
    pub fn ts_language(&self) -> Option<tree_sitter::Language> {
        match self {
            Language::Rust => Some(tree_sitter_rust::language()),
            Language::TypeScript => Some(tree_sitter_typescript::language_typescript()),
            Language::Python => Some(tree_sitter_python::language()),
            Language::Toml => Some(tree_sitter_toml::language()),
            Language::Markdown => Some(tree_sitter_md::language()),
            Language::Unknown => None,
        }
    }

    /// Get highlight query for this language
    pub fn highlight_query(&self) -> Option<&'static str> {
        match self {
            Language::Rust => Some(RUST_HIGHLIGHT_QUERY),
            Language::TypeScript => Some(TYPESCRIPT_HIGHLIGHT_QUERY),
            Language::Python => Some(PYTHON_HIGHLIGHT_QUERY),
            Language::Toml => Some(TOML_HIGHLIGHT_QUERY),
            Language::Markdown => Some(MARKDOWN_HIGHLIGHT_QUERY),
            Language::Unknown => None,
        }
    }
}

/// Highlighter for syntax highlighting using tree-sitter
pub struct Highlighter {
    language: Language,
    query: Option<Query>,
}

impl Highlighter {
    /// Create a new highlighter for the given file path
    pub fn new(path: &Path) -> anyhow::Result<Self> {
        let language = Language::from_path(path);

        // Create query first (needs language), then set up parser
        let query = language
            .ts_language()
            .zip(language.highlight_query())
            .and_then(|(lang, q)| Query::new(lang, q).ok());

        // Parser is created fresh for each highlight call, don't store it
        Ok(Self { language, query })
    }

    /// Check if highlighting is available for this language
    pub fn has_highlighting(&self) -> bool {
        self.query.is_some()
    }

    /// Highlight source code and return spans
    pub fn highlight(&self, source: &str) -> Vec<HighlightedSpan> {
        if !self.has_highlighting() {
            return vec![];
        }

        // Create a new parser for this highlight call
        let ts_lang = self.language.ts_language();
        let mut parser = Parser::new();
        if let Some(lang) = ts_lang {
            let _ = parser.set_language(lang);
        }

        let tree = match parser.parse(source, None) {
            Some(tree) => tree,
            None => return vec![],
        };

        let query = match &self.query {
            Some(q) => q,
            None => return vec![],
        };

        let mut cursor = QueryCursor::new();
        let matches = cursor.matches(query, tree.root_node(), source.as_bytes());

        let mut spans = vec![];

        for m in matches {
            for capture in m.captures {
                let node = capture.node;
                let capture_name = &query.capture_names()[capture.index as usize];
                let highlight = capture_name_to_highlight(capture_name);

                spans.push(HighlightedSpan::new(
                    node.start_byte(),
                    node.end_byte(),
                    highlight,
                ));
            }
        }

        // Sort spans by start position and merge overlapping
        spans.sort_by_key(|a| a.start);
        merge_overlapping_spans(spans)
    }
}

/// Convert tree-sitter capture name to highlight name
fn capture_name_to_highlight(capture_name: &str) -> String {
    match capture_name {
        "keyword" | "keyword.control" | "keyword.function" | "keyword.operator" => "keyword",
        "string" | "string.quoted" | "string.raw" => "string",
        "comment" | "comment.line" | "comment.block" => "comment",
        "number" | "numeric" | "float" => "number",
        "boolean" => "boolean",
        "function" | "function.method" | "function.call" => "function",
        "variable" | "variable.parameter" | "variable.builtin" => "variable",
        "type" | "type.builtin" | "type.definition" => "type",
        "operator" => "operator",
        "punctuation" | "punctuation.bracket" | "punctuation.delimiter" => "punctuation",
        "constant" | "constant.builtin" => "constant",
        "property" | "property.attribute" => "property",
        "tag" => "tag",
        "attribute" => "attribute",
        "module" | "namespace" => "module",
        "lifetime" => "lifetime",
        "macro" => "macro",
        "label" => "label",
        _ => "text",
    }
    .to_string()
}

/// Merge overlapping highlight spans, prioritizing more specific highlights
fn merge_overlapping_spans(spans: Vec<HighlightedSpan>) -> Vec<HighlightedSpan> {
    if spans.is_empty() {
        return spans;
    }

    let mut result = vec![spans[0].clone()];

    for span in spans.into_iter().skip(1) {
        let last = result.last_mut().unwrap();

        if span.start <= last.end {
            // Overlapping spans - determine which highlight is more specific
            // Priority: non-"text" > longer name > later span
            let should_use_new_highlight = if last.highlight == "text" {
                true // Current is fallback, use new
            } else if span.highlight == "text" {
                false // New is fallback, keep current
            } else {
                // Both are specific - prefer longer name (more specific)
                span.highlight.len() > last.highlight.len()
            };

            if should_use_new_highlight {
                last.highlight = span.highlight;
            }

            // Extend range if needed
            if span.end > last.end {
                last.end = span.end;
            }
        } else {
            result.push(span);
        }
    }

    result
}

/// Map highlight name to ratatui Style
pub fn highlight_to_style(highlight: &str) -> Style {
    match highlight {
        "keyword" => Style::default()
            .fg(Color::Magenta)
            .add_modifier(Modifier::BOLD),
        "string" => Style::default().fg(Color::Green),
        "comment" => Style::default().fg(Color::DarkGray),
        "number" => Style::default().fg(Color::Cyan),
        "boolean" => Style::default().fg(Color::Yellow),
        "function" => Style::default().fg(Color::Blue),
        "variable" => Style::default().fg(Color::White),
        "type" => Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
        "operator" => Style::default().fg(Color::Magenta),
        "punctuation" => Style::default().fg(Color::White),
        "constant" => Style::default().fg(Color::Yellow),
        "property" => Style::default().fg(Color::LightBlue),
        "tag" => Style::default().fg(Color::Blue),
        "attribute" => Style::default().fg(Color::Yellow),
        "module" => Style::default().fg(Color::LightCyan),
        "lifetime" => Style::default().fg(Color::LightMagenta),
        "macro" => Style::default().fg(Color::LightYellow),
        "label" => Style::default().fg(Color::LightGreen),
        _ => Style::default().fg(Color::White),
    }
}

// Tree-sitter highlight queries

const RUST_HIGHLIGHT_QUERY: &str = include_str!("rust-highlight.scm");

const TYPESCRIPT_HIGHLIGHT_QUERY: &str = include_str!("typescript-highlight.scm");

const PYTHON_HIGHLIGHT_QUERY: &str = include_str!("python-highlight.scm");

const TOML_HIGHLIGHT_QUERY: &str = include_str!("toml-highlight.scm");

const MARKDOWN_HIGHLIGHT_QUERY: &str = include_str!("markdown-highlight.scm");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_language_from_path() {
        assert_eq!(Language::from_path(Path::new("test.rs")), Language::Rust);
        assert_eq!(
            Language::from_path(Path::new("test.ts")),
            Language::TypeScript
        );
        assert_eq!(Language::from_path(Path::new("test.py")), Language::Python);
        assert_eq!(Language::from_path(Path::new("test.toml")), Language::Toml);
        assert_eq!(
            Language::from_path(Path::new("test.md")),
            Language::Markdown
        );
        assert_eq!(
            Language::from_path(Path::new("test.unknown")),
            Language::Unknown
        );
    }

    #[test]
    fn test_highlighter_rust() {
        let source = "fn main() {\n    let x = \"hello\";\n    // comment\n}";

        let highlighter = Highlighter::new(Path::new("test.rs")).unwrap();
        let spans = highlighter.highlight(source);

        // Should have some highlights
        assert!(!spans.is_empty(), "Expected highlight spans for Rust code");

        // Check that we have keyword highlights
        let keyword_spans: Vec<_> = spans.iter().filter(|s| s.highlight == "keyword").collect();
        assert!(!keyword_spans.is_empty(), "Expected keyword highlights");
    }

    #[test]
    fn test_highlighter_unknown_language() {
        let highlighter = Highlighter::new(Path::new("test.unknown")).unwrap();
        assert!(!highlighter.has_highlighting());

        let spans = highlighter.highlight("some content");
        assert!(spans.is_empty());
    }

    #[test]
    fn test_merge_overlapping_spans() {
        let spans = vec![
            HighlightedSpan::new(0, 5, "keyword"),
            HighlightedSpan::new(3, 8, "type"),
            HighlightedSpan::new(10, 15, "string"),
        ];

        let merged = merge_overlapping_spans(spans);
        assert_eq!(merged.len(), 2);
        assert_eq!(merged[0].start, 0);
        assert_eq!(merged[0].end, 8);
    }
}
