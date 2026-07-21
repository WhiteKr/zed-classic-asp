use regex::Regex;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IncludeKind {
    File,
    Virtual,
}

#[derive(Debug, Clone)]
pub struct IncludeDirective {
    pub kind: IncludeKind,
    pub raw_path: String,
    pub resolved: Option<PathBuf>,
    pub line: u32,
    /// Byte range of the quoted path (without quotes) within the line.
    pub path_span: (usize, usize),
    /// Byte range of the whole directive within the line.
    pub directive_span: (usize, usize),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymbolKind {
    Sub,
    Function,
    Class,
    Property,
}

#[derive(Debug, Clone)]
pub struct SymbolDef {
    pub name: String,
    pub kind: SymbolKind,
    pub line: u32,
    /// Byte range of the name within the line.
    pub name_span: (usize, usize),
    /// The trimmed definition line, used for hover.
    pub signature: String,
}

#[derive(Debug, Default, Clone)]
pub struct FileIndex {
    pub includes: Vec<IncludeDirective>,
    pub symbols: Vec<SymbolDef>,
}

fn include_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"(?i)<!--\s*#include\s+(file|virtual)\s*=\s*"([^"]*)"\s*-->"#).unwrap()
    })
}

fn sub_fn_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        // `(?:<%=?\s*)?` — definitions can share a line with the opening ASP delimiter.
        Regex::new(r"(?i)^\s*(?:<%=?\s*)?(?:(?:public|private|default)\s+)*(sub|function)\s+([a-zA-Z_]\w*)")
            .unwrap()
    })
}

fn class_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(?i)^\s*(?:<%=?\s*)?class\s+([a-zA-Z_]\w*)").unwrap())
}

fn property_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"(?i)^\s*(?:<%=?\s*)?(?:(?:public|private|default)\s+)*property\s+(?:get|let|set)\s+([a-zA-Z_]\w*)")
            .unwrap()
    })
}

/// Lexically normalize a path (resolve `.` and `..` without touching the filesystem).
pub fn normalize(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for comp in path.components() {
        match comp {
            std::path::Component::ParentDir => {
                out.pop();
            }
            std::path::Component::CurDir => {}
            other => out.push(other),
        }
    }
    out
}

/// Resolve a path, falling back to a case-insensitive per-component search
/// (Classic ASP codebases written on Windows often have mismatched case).
fn resolve_existing(path: &Path) -> Option<PathBuf> {
    let path = normalize(path);
    if path.is_file() {
        return path.canonicalize().ok();
    }
    // Keep the root/drive prefix so the walk starts from the path's own root
    // (a hardcoded "/" is drive-relative on Windows).
    let mut current = PathBuf::new();
    for comp in path.components() {
        match comp {
            std::path::Component::RootDir | std::path::Component::Prefix(_) => {
                current.push(comp.as_os_str());
            }
            std::path::Component::Normal(name) => {
                let name = name.to_string_lossy();
                let entries = std::fs::read_dir(&current).ok()?;
                let found = entries.filter_map(|e| e.ok()).find(|e| {
                    e.file_name().to_string_lossy().eq_ignore_ascii_case(&name)
                })?;
                current = found.path();
            }
            _ => return None,
        }
    }
    current.is_file().then(|| current.canonicalize().unwrap_or(current))
}

pub fn resolve_include(
    from_file: &Path,
    web_root: &Path,
    kind: IncludeKind,
    raw: &str,
) -> Option<PathBuf> {
    let cleaned = raw.trim().replace('\\', "/");
    let candidate = match kind {
        IncludeKind::File => from_file.parent()?.join(&cleaned),
        IncludeKind::Virtual => web_root.join(cleaned.trim_start_matches('/')),
    };
    resolve_existing(&candidate)
}

pub fn parse_file(text: &str, path: &Path, web_root: &Path) -> FileIndex {
    let mut index = FileIndex::default();

    for (line_no, line) in text.lines().enumerate() {
        let line_no = line_no as u32;

        for caps in include_re().captures_iter(line) {
            let whole = caps.get(0).unwrap();
            let kind_m = caps.get(1).unwrap();
            let path_m = caps.get(2).unwrap();
            let kind = if kind_m.as_str().eq_ignore_ascii_case("file") {
                IncludeKind::File
            } else {
                IncludeKind::Virtual
            };
            let raw_path = path_m.as_str().to_string();
            let resolved = resolve_include(path, web_root, kind, &raw_path);
            index.includes.push(IncludeDirective {
                kind,
                raw_path,
                resolved,
                line: line_no,
                path_span: (path_m.start(), path_m.end()),
                directive_span: (whole.start(), whole.end()),
            });
        }

        let symbol = if let Some(caps) = sub_fn_re().captures(line) {
            let kind = if caps.get(1).unwrap().as_str().eq_ignore_ascii_case("sub") {
                SymbolKind::Sub
            } else {
                SymbolKind::Function
            };
            Some((kind, caps.get(2).unwrap()))
        } else if let Some(caps) = property_re().captures(line) {
            Some((SymbolKind::Property, caps.get(1).unwrap()))
        } else if let Some(caps) = class_re().captures(line) {
            Some((SymbolKind::Class, caps.get(1).unwrap()))
        } else {
            None
        };

        if let Some((kind, name_m)) = symbol {
            index.symbols.push(SymbolDef {
                name: name_m.as_str().to_string(),
                kind,
                line: line_no,
                name_span: (name_m.start(), name_m.end()),
                signature: line.trim().to_string(),
            });
        }
    }

    index
}
