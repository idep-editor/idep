use anyhow::{anyhow, Result};
use std::path::Path;
use tree_sitter::{Language, Node, Parser};

#[derive(Debug, Clone)]
pub struct Chunk {
    pub kind: String,
    pub name: Option<String>,
    pub start_byte: usize,
    pub end_byte: usize,
    pub text: String,
}

#[derive(Copy, Clone, Debug)]
enum Lang {
    Rust,
    TypeScript,
    Python,
}

pub struct AstChunker;

impl AstChunker {
    pub fn new() -> Self {
        Self
    }

    pub fn chunk(&self, path: &Path, source: &str) -> Result<Vec<Chunk>> {
        let lang = language_for_path(path)?;
        let language = language_for(lang)?;

        let mut parser = Parser::new();
        parser.set_language(&language)?;
        let tree = parser
            .parse(source, None)
            .ok_or_else(|| anyhow!("Failed to parse source"))?;

        let root = tree.root_node();
        let mut stack = Vec::new();
        stack.push(root);

        let mut chunks = Vec::new();
        while let Some(node) = stack.pop() {
            for child in node.children(&mut node.walk()) {
                stack.push(child);
            }

            if let Some(kind) = map_kind(lang, node) {
                if let Some(text) = node_text(node, source) {
                    let name = name_of(node, source);
                    chunks.push(Chunk {
                        kind,
                        name,
                        start_byte: node.start_byte(),
                        end_byte: node.end_byte(),
                        text,
                    });
                }
            }
        }

        Ok(chunks)
    }
}

fn language_for(lang: Lang) -> Result<Language> {
    match lang {
        Lang::Rust => Ok(tree_sitter_rust::language()),
        Lang::TypeScript => Ok(tree_sitter_typescript::language_tsx()),
        Lang::Python => Ok(tree_sitter_python::language()),
    }
}

fn language_for_path(path: &Path) -> Result<Lang> {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    match ext {
        "rs" => Ok(Lang::Rust),
        "ts" | "tsx" | "js" | "jsx" => Ok(Lang::TypeScript),
        "py" => Ok(Lang::Python),
        _ => Err(anyhow!("Unsupported language extension: {ext}")),
    }
}

fn map_kind(lang: Lang, node: Node) -> Option<String> {
    let k = node.kind();
    match lang {
        Lang::Rust => match k {
            "function_item" => Some("function".into()),
            "impl_item" => Some("impl".into()),
            "struct_item" => Some("struct".into()),
            "trait_item" => Some("trait".into()),
            "enum_item" => Some("enum".into()),
            _ => None,
        },
        Lang::TypeScript => match k {
            "function_declaration" => Some("function".into()),
            "class_declaration" => Some("class".into()),
            "interface_declaration" => Some("interface".into()),
            "type_alias_declaration" => Some("type".into()),
            _ => None,
        },
        Lang::Python => match k {
            "function_definition" => Some("function".into()),
            "class_definition" => Some("class".into()),
            _ => None,
        },
    }
}

fn name_of(node: Node, source: &str) -> Option<String> {
    // Standard name field
    if let Some(name) = node
        .child_by_field_name("name")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
    {
        return Some(name.to_string());
    }

    // Rust impl blocks expose type via field "type"
    if node.kind() == "impl_item" {
        if let Some(ty) = node
            .child_by_field_name("type")
            .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        {
            return Some(ty.to_string());
        }
    }

    None
}

fn node_text(node: Node, source: &str) -> Option<String> {
    let bytes = source.as_bytes();
    let start = node.start_byte();
    let end = node.end_byte();
    if start >= end || end > bytes.len() {
        return None;
    }
    std::str::from_utf8(&bytes[start..end])
        .ok()
        .map(|s| s.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn chunks_rust_items_with_names() {
        let source = r#"
struct Foo {
    x: i32,
}

impl Foo {
    fn new() -> Self {
        Foo { x: 0 }
    }

    fn bar(&self) {}
}

trait Bar {
    fn baz(&self);
}

enum E {
    A,
    B,
}

fn top() {}
"#;

        let path = PathBuf::from("dummy.rs");
        let chunker = AstChunker::new();
        let chunks = chunker.chunk(&path, source).expect("chunk");

        let mut by_kind = std::collections::HashMap::new();
        for c in chunks {
            by_kind
                .entry(c.kind.clone())
                .or_insert_with(Vec::new)
                .push(c);
        }

        let names = |kind: &str| -> Vec<String> {
            by_kind
                .get(kind)
                .unwrap_or(&Vec::new())
                .iter()
                .filter_map(|c| c.name.clone())
                .collect()
        };

        assert!(names("struct").contains(&"Foo".to_string()));
        assert!(names("impl").contains(&"Foo".to_string()));
        assert!(names("trait").contains(&"Bar".to_string()));
        assert!(names("enum").contains(&"E".to_string()));
        assert!(names("function").contains(&"top".to_string()));
        // method names appear as functions within impl
        let fn_names = names("function");
        assert!(fn_names.contains(&"new".to_string()));
        assert!(fn_names.contains(&"bar".to_string()));
    }

    #[test]
    fn chunks_typescript_items_with_names() {
        let source = r#"
function top() {}

class Foo {
  bar() {}
}

interface IFoo {
  baz(): void;
}

type Alias = { a: number };
"#;

        let path = PathBuf::from("dummy.ts");
        let chunker = AstChunker::new();
        let chunks = chunker.chunk(&path, source).expect("chunk");

        let mut by_kind = std::collections::HashMap::new();
        for c in chunks {
            by_kind
                .entry(c.kind.clone())
                .or_insert_with(Vec::new)
                .push(c);
        }

        let names = |kind: &str| -> Vec<String> {
            by_kind
                .get(kind)
                .unwrap_or(&Vec::new())
                .iter()
                .filter_map(|c| c.name.clone())
                .collect()
        };

        assert!(names("function").contains(&"top".to_string()));
        assert!(names("class").contains(&"Foo".to_string()));
        assert!(names("interface").contains(&"IFoo".to_string()));
        assert!(names("type").contains(&"Alias".to_string()));
    }
}
