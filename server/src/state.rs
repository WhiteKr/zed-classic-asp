use crate::parse::{self, FileIndex, IncludeDirective, SymbolDef};
use lsp_types::{Location, Position, Range, Url};
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

const INDEXED_EXTENSIONS: &[&str] = &["asp", "asa", "inc", "vbs"];
const MAX_INDEXED_FILES: usize = 20_000;

pub struct State {
    pub root: PathBuf,
    pub web_root: PathBuf,
    /// Whether the client supports `LocationLink` responses for definitions.
    pub definition_link_support: bool,
    /// Open-buffer contents, keyed by canonical path.
    pub overlays: HashMap<PathBuf, String>,
    pub index: HashMap<PathBuf, FileIndex>,
}

impl State {
    pub fn new(root: PathBuf, web_root: Option<String>) -> Self {
        let web_root = match web_root {
            Some(w) => {
                let p = PathBuf::from(&w);
                if p.is_absolute() {
                    p
                } else {
                    root.join(p)
                }
            }
            None => root.clone(),
        };
        Self {
            root,
            web_root,
            definition_link_support: false,
            overlays: HashMap::new(),
            index: HashMap::new(),
        }
    }

    pub fn canon(path: &Path) -> PathBuf {
        path.canonicalize().unwrap_or_else(|_| parse::normalize(path))
    }

    pub fn scan_workspace(&mut self) {
        let mut count = 0;
        let root = self.root.clone();
        for entry in WalkDir::new(&root)
            .follow_links(false)
            .into_iter()
            .filter_entry(|e| {
                let name = e.file_name().to_string_lossy();
                !(name.starts_with('.') || name == "node_modules" || name == "target")
            })
            .filter_map(|e| e.ok())
        {
            if !entry.file_type().is_file() {
                continue;
            }
            let ext = entry
                .path()
                .extension()
                .map(|e| e.to_string_lossy().to_ascii_lowercase());
            let Some(ext) = ext else { continue };
            if !INDEXED_EXTENSIONS.contains(&ext.as_str()) {
                continue;
            }
            self.reindex(&Self::canon(entry.path()));
            count += 1;
            if count >= MAX_INDEXED_FILES {
                eprintln!("asp-ls: workspace scan stopped at {MAX_INDEXED_FILES} files");
                break;
            }
        }
    }

    pub fn text_of(&self, path: &Path) -> Option<String> {
        if let Some(text) = self.overlays.get(path) {
            return Some(text.clone());
        }
        // Legacy Classic ASP files are often CP949/EUC-KR; decode lossily
        // rather than dropping the file from the index.
        let bytes = std::fs::read(path).ok()?;
        Some(String::from_utf8_lossy(&bytes).into_owned())
    }

    pub fn reindex(&mut self, path: &Path) {
        if let Some(text) = self.text_of(path) {
            let index = parse::parse_file(&text, path, &self.web_root);
            self.index.insert(path.to_path_buf(), index);
        }
    }

    /// Re-resolves files whose cached include state may have been invalidated
    /// by a change elsewhere: any file with an include that is unresolved (the
    /// target may have just been created) or whose resolved target no longer
    /// exists (just deleted). Returns the reindexed paths.
    pub fn reindex_dependents(&mut self, changed: &Path) -> Vec<PathBuf> {
        let stale: Vec<PathBuf> = self
            .index
            .iter()
            .filter(|(file, index)| {
                file.as_path() != changed
                    && index.includes.iter().any(|inc| match &inc.resolved {
                        None => true,
                        Some(p) => !p.exists(),
                    })
            })
            .map(|(file, _)| file.clone())
            .collect();
        for file in &stale {
            self.reindex(file);
        }
        stale
    }

    /// Files reachable from `start` by following resolved includes (breadth-first,
    /// `start` itself first).
    pub fn include_closure(&self, start: &Path) -> Vec<PathBuf> {
        let mut seen: HashSet<PathBuf> = HashSet::new();
        let mut order = Vec::new();
        let mut queue = VecDeque::from([start.to_path_buf()]);
        while let Some(file) = queue.pop_front() {
            if !seen.insert(file.clone()) {
                continue;
            }
            order.push(file.clone());
            if let Some(index) = self.index.get(&file) {
                for inc in &index.includes {
                    if let Some(target) = &inc.resolved {
                        queue.push_back(target.clone());
                    }
                }
            }
        }
        order
    }

    /// All include directives across the workspace that resolve to `target`.
    pub fn includers_of(&self, target: &Path) -> Vec<(PathBuf, &IncludeDirective)> {
        let mut out = Vec::new();
        for (file, index) in &self.index {
            for inc in &index.includes {
                if inc.resolved.as_deref() == Some(target) {
                    out.push((file.clone(), inc));
                }
            }
        }
        out
    }

    /// Definitions of `name` (case-insensitive), searching the include closure of
    /// `from` first; falls back to the whole workspace index.
    pub fn find_definitions(&self, from: &Path, name: &str) -> Vec<(PathBuf, &SymbolDef)> {
        let mut hits = Vec::new();
        for file in self.include_closure(from) {
            if let Some(index) = self.index.get(&file) {
                for sym in &index.symbols {
                    if sym.name.eq_ignore_ascii_case(name) {
                        hits.push((file.clone(), sym));
                    }
                }
            }
        }
        if !hits.is_empty() {
            return hits;
        }
        for (file, index) in &self.index {
            for sym in &index.symbols {
                if sym.name.eq_ignore_ascii_case(name) {
                    hits.push((file.clone(), sym));
                }
            }
        }
        hits
    }
}

// --- position helpers (LSP positions are UTF-16 code units) ---

pub fn byte_to_utf16_col(line: &str, byte_idx: usize) -> u32 {
    // Indexed spans can be stale relative to the text on disk, so `byte_idx`
    // may not land on a char boundary — count chars instead of slicing.
    let mut units = 0u32;
    for (i, c) in line.char_indices() {
        if i >= byte_idx {
            break;
        }
        units += c.len_utf16() as u32;
    }
    units
}

pub fn utf16_col_to_byte(line: &str, utf16_col: u32) -> usize {
    let mut units = 0u32;
    for (byte_idx, c) in line.char_indices() {
        if units >= utf16_col {
            return byte_idx;
        }
        units += c.len_utf16() as u32;
    }
    line.len()
}

pub fn span_to_range(line_no: u32, line: &str, span: (usize, usize)) -> Range {
    Range {
        start: Position {
            line: line_no,
            character: byte_to_utf16_col(line, span.0),
        },
        end: Position {
            line: line_no,
            character: byte_to_utf16_col(line, span.1),
        },
    }
}

pub fn path_to_uri(path: &Path) -> Option<Url> {
    Url::from_file_path(path).ok()
}

pub fn symbol_location(file: &Path, text: &str, sym: &SymbolDef) -> Option<Location> {
    let line = text.lines().nth(sym.line as usize)?;
    Some(Location {
        uri: path_to_uri(file)?,
        range: span_to_range(sym.line, line, sym.name_span),
    })
}

/// The double-quoted string literal containing the cursor, if any.
/// Returns the content (without the quotes) and its byte span within the line.
pub fn string_at(line: &str, utf16_col: u32) -> Option<(String, (usize, usize))> {
    let byte_idx = utf16_col_to_byte(line, utf16_col);
    let bytes = line.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != b'"' {
            i += 1;
            continue;
        }
        let start = i + 1;
        let mut end = start;
        while end < bytes.len() && bytes[end] != b'"' {
            end += 1;
        }
        if end >= bytes.len() {
            return None; // unterminated string
        }
        if byte_idx >= start && byte_idx <= end {
            return Some((line[start..end].to_string(), (start, end)));
        }
        i = end + 1;
    }
    None
}

/// The identifier under the cursor plus, when it is a `qualifier.member`
/// access, the qualifier before the dot.
pub fn word_at(line: &str, utf16_col: u32) -> Option<(String, Option<String>)> {
    let byte_idx = utf16_col_to_byte(line, utf16_col);
    let bytes = line.as_bytes();
    let is_word = |b: u8| b.is_ascii_alphanumeric() || b == b'_';

    let mut start = byte_idx.min(line.len());
    while start > 0 && is_word(bytes[start - 1]) {
        start -= 1;
    }
    let mut end = start;
    while end < bytes.len() && is_word(bytes[end]) {
        end += 1;
    }
    if start == end {
        return None;
    }
    let word = line[start..end].to_string();

    let qualifier = if start > 0 && bytes[start - 1] == b'.' {
        let q_end = start - 1;
        let mut q_start = q_end;
        while q_start > 0 && is_word(bytes[q_start - 1]) {
            q_start -= 1;
        }
        (q_start < q_end).then(|| line[q_start..q_end].to_string())
    } else {
        None
    };

    Some((word, qualifier))
}
