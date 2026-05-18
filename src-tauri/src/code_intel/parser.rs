use parking_lot::RwLock;
use std::collections::HashMap;
use tokio::fs;
use tree_sitter::{Language, Parser};

use super::SymbolDef;

pub const SUPPORTED_EXTENSIONS: &[&str] = &[
    "rs", "py", "js", "jsx", "ts", "tsx", "go", "java", "c", "cpp", "h", "hpp",
];

pub struct TreeSitterParser {
    parsers: RwLock<HashMap<String, Parser>>,
    languages: HashMap<String, Language>,
}

impl std::fmt::Debug for TreeSitterParser {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TreeSitterParser")
            .field("languages", &self.languages.keys().collect::<Vec<_>>())
            .finish_non_exhaustive()
    }
}

impl Default for TreeSitterParser {
    fn default() -> Self {
        Self::new()
    }
}

impl TreeSitterParser {
    pub fn new() -> Self {
        let mut languages = HashMap::new();
        languages.insert("rs".to_string(), tree_sitter_rust::LANGUAGE.into());
        languages.insert("py".to_string(), tree_sitter_python::LANGUAGE.into());
        languages.insert("js".to_string(), tree_sitter_javascript::LANGUAGE.into());
        languages.insert(
            "ts".to_string(),
            tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
        );
        languages.insert(
            "tsx".to_string(),
            tree_sitter_typescript::LANGUAGE_TSX.into(),
        );
        languages.insert("go".to_string(), tree_sitter_go::LANGUAGE.into());
        languages.insert("java".to_string(), tree_sitter_java::LANGUAGE.into());
        languages.insert("c".to_string(), tree_sitter_c::LANGUAGE.into());
        languages.insert("cpp".to_string(), tree_sitter_cpp::LANGUAGE.into());

        Self {
            parsers: RwLock::new(HashMap::new()),
            languages,
        }
    }

    pub async fn parse(&self, path: &str) -> anyhow::Result<Vec<SymbolDef>> {
        let ext = path.rsplit('.').next().unwrap_or("");
        let lang = self
            .languages
            .get(ext)
            .ok_or_else(|| anyhow::anyhow!("Unsupported language: {}", ext))?;

        let source = fs::read_to_string(path).await?;
        let mut parser = self.parsers.write();
        let parser = parser.entry(ext.to_string()).or_insert_with(|| {
            let mut p = Parser::new();
            p.set_language(lang).ok();
            p
        });

        let tree = parser
            .parse(&source, None)
            .ok_or_else(|| anyhow::anyhow!("Failed to parse {}", path))?;

        let root = tree.root_node();
        let mut symbols = Vec::new();
        self.extract_symbols(root, &source, &mut symbols, 0, ext);
        Ok(symbols)
    }

    fn extract_symbols(
        &self,
        node: tree_sitter::Node,
        source: &str,
        symbols: &mut Vec<SymbolDef>,
        _depth: usize,
        ext: &str,
    ) {
        let kind = node.kind();

        let is_definition = matches!(
            kind,
            "function_item"
                | "function_definition"
                | "method_definition"
                | "class_definition"
                | "struct_item"
                | "impl_item"
                | "trait_item"
                | "enum_item"
                | "module"
                | "interface_declaration"
                | "type_alias"
                | "const_item"
                | "static_item"
                | "macro_definition"
                | "function_declaration"
                | "method_signature"
        );

        if is_definition {
            let name_node = node.child_by_field_name("name");
            if let Some(name_node) = name_node {
                let name = name_node.utf8_text(source.as_bytes()).unwrap_or("");
                let start = name_node.start_position();
                let _end = name_node.end_position();

                symbols.push(SymbolDef {
                    name: name.to_string(),
                    kind: kind.to_string(),
                    file: String::new(),
                    line: start.row + 1,
                    column: start.column + 1,
                    signature: self.get_signature(node, source, ext),
                    doc: self.get_doc_comment(node, source),
                });
            }
        }

        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                self.extract_symbols(child, source, symbols, _depth + 1, ext);
            }
        }
    }

    fn get_signature(&self, node: tree_sitter::Node, source: &str, _ext: &str) -> Option<String> {
        let start = node.start_position();
        let end = node.end_position();
        if end.row == start.row {
            node.utf8_text(source.as_bytes())
                .ok()
                .map(|s| s.to_string())
        } else {
            // First line only for multi-line
            let line = source.lines().nth(start.row)?;
            Some(line.to_string())
        }
    }

    fn get_doc_comment(&self, node: tree_sitter::Node, source: &str) -> Option<String> {
        let prev = node.prev_sibling()?;
        let prev_text = prev.utf8_text(source.as_bytes()).ok()?;
        if prev_text.starts_with("///")
            || prev_text.starts_with("/**")
            || prev_text.starts_with("//")
        {
            Some(prev_text.to_string())
        } else {
            None
        }
    }
}
