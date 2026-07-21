mod builtins;
mod parse;
mod state;

use anyhow::Result;
use lsp_server::{Connection, Message, Notification, Request, Response};
use lsp_types::{
    Diagnostic, DiagnosticSeverity, DidChangeTextDocumentParams, DidCloseTextDocumentParams,
    DidOpenTextDocumentParams, DidSaveTextDocumentParams, GotoDefinitionParams,
    GotoDefinitionResponse, Hover, HoverContents, HoverParams, HoverProviderCapability,
    InitializeParams, Location, LocationLink, MarkupContent, MarkupKind, OneOf, Position,
    PublishDiagnosticsParams, Range, ReferenceParams, ServerCapabilities, SymbolInformation,
    TextDocumentSyncCapability, TextDocumentSyncKind, Url, WorkspaceSymbolParams,
    WorkspaceSymbolResponse,
};
use serde::Deserialize;
use std::path::PathBuf;

use crate::parse::SymbolKind;
use crate::state::{path_to_uri, span_to_range, symbol_location, word_at, State};

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct InitOptions {
    web_root: Option<String>,
}

fn main() -> Result<()> {
    let (connection, io_threads) = Connection::stdio();

    let capabilities = serde_json::to_value(ServerCapabilities {
        text_document_sync: Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL)),
        definition_provider: Some(OneOf::Left(true)),
        references_provider: Some(OneOf::Left(true)),
        hover_provider: Some(HoverProviderCapability::Simple(true)),
        workspace_symbol_provider: Some(OneOf::Left(true)),
        ..Default::default()
    })?;

    let init_value = connection.initialize(capabilities)?;
    let init_params: InitializeParams = serde_json::from_value(init_value)?;

    let root = init_params
        .workspace_folders
        .as_ref()
        .and_then(|f| f.first())
        .and_then(|f| f.uri.to_file_path().ok())
        .or_else(|| {
            #[allow(deprecated)]
            init_params.root_uri.as_ref().and_then(|u| u.to_file_path().ok())
        })
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/")));

    let options: InitOptions = init_params
        .initialization_options
        .and_then(|v| serde_json::from_value(v).ok())
        .unwrap_or_default();

    let mut state = State::new(State::canon(&root), options.web_root);
    state.definition_link_support = init_params
        .capabilities
        .text_document
        .as_ref()
        .and_then(|t| t.definition.as_ref())
        .and_then(|d| d.link_support)
        .unwrap_or(false);
    eprintln!(
        "asp-ls: root {:?}, web root {:?}",
        state.root, state.web_root
    );
    state.scan_workspace();
    eprintln!("asp-ls: indexed {} files", state.index.len());

    for msg in &connection.receiver {
        match msg {
            Message::Request(req) => {
                if connection.handle_shutdown(&req)? {
                    break;
                }
                let response = handle_request(&mut state, req);
                connection.sender.send(Message::Response(response))?;
            }
            Message::Notification(note) => {
                if let Some(uri) = handle_notification(&mut state, note) {
                    publish_diagnostics(&connection, &state, &uri)?;
                    // A change here can (in)validate cached include resolution
                    // in other files — refresh their diagnostics too.
                    if let Some(path) = uri_path(&uri) {
                        for dep in state.reindex_dependents(&path) {
                            if let Some(dep_uri) = path_to_uri(&dep) {
                                publish_diagnostics(&connection, &state, &dep_uri)?;
                            }
                        }
                    }
                }
            }
            Message::Response(_) => {}
        }
    }

    io_threads.join()?;
    Ok(())
}

fn handle_request(state: &mut State, req: Request) -> Response {
    let id = req.id.clone();
    let result = match req.method.as_str() {
        "textDocument/definition" => serde_json::from_value(req.params)
            .map_err(anyhow::Error::from)
            .and_then(|p| definition(state, p)),
        "textDocument/references" => serde_json::from_value(req.params)
            .map_err(anyhow::Error::from)
            .and_then(|p| references(state, p)),
        "textDocument/hover" => serde_json::from_value(req.params)
            .map_err(anyhow::Error::from)
            .and_then(|p| hover(state, p)),
        "workspace/symbol" => serde_json::from_value(req.params)
            .map_err(anyhow::Error::from)
            .and_then(|p| workspace_symbol(state, p)),
        _ => Ok(serde_json::Value::Null),
    };
    match result {
        Ok(value) => Response::new_ok(id, value),
        Err(err) => Response::new_err(id, -32603, err.to_string()),
    }
}

/// Handles a notification; returns a URI whose diagnostics should be republished.
fn handle_notification(state: &mut State, note: Notification) -> Option<Url> {
    match note.method.as_str() {
        "textDocument/didOpen" => {
            let params: DidOpenTextDocumentParams = serde_json::from_value(note.params).ok()?;
            let path = uri_path(&params.text_document.uri)?;
            state.overlays.insert(path.clone(), params.text_document.text);
            state.reindex(&path);
            Some(params.text_document.uri)
        }
        "textDocument/didChange" => {
            let params: DidChangeTextDocumentParams = serde_json::from_value(note.params).ok()?;
            let path = uri_path(&params.text_document.uri)?;
            let text = params.content_changes.into_iter().last()?.text;
            state.overlays.insert(path.clone(), text);
            state.reindex(&path);
            Some(params.text_document.uri)
        }
        "textDocument/didSave" => {
            let params: DidSaveTextDocumentParams = serde_json::from_value(note.params).ok()?;
            let path = uri_path(&params.text_document.uri)?;
            state.reindex(&path);
            Some(params.text_document.uri)
        }
        "textDocument/didClose" => {
            let params: DidCloseTextDocumentParams = serde_json::from_value(note.params).ok()?;
            let path = uri_path(&params.text_document.uri)?;
            state.overlays.remove(&path);
            state.reindex(&path);
            // Republish so diagnostics computed from the discarded overlay
            // don't stay stuck in the client.
            Some(params.text_document.uri)
        }
        _ => None,
    }
}

fn uri_path(uri: &Url) -> Option<PathBuf> {
    uri.to_file_path().ok().map(|p| State::canon(&p))
}

fn publish_diagnostics(connection: &Connection, state: &State, uri: &Url) -> Result<()> {
    let Some(path) = uri_path(uri) else { return Ok(()) };
    let mut diagnostics = Vec::new();
    if let (Some(index), Some(text)) = (state.index.get(&path), state.text_of(&path)) {
        let lines: Vec<&str> = text.lines().collect();
        for inc in &index.includes {
            if inc.resolved.is_none() {
                if let Some(line) = lines.get(inc.line as usize) {
                    diagnostics.push(Diagnostic {
                        range: span_to_range(inc.line, line, inc.path_span),
                        severity: Some(DiagnosticSeverity::WARNING),
                        source: Some("asp-ls".into()),
                        message: format!("Cannot resolve include \"{}\"", inc.raw_path),
                        ..Default::default()
                    });
                }
            }
        }
    }
    let params = PublishDiagnosticsParams {
        uri: uri.clone(),
        diagnostics,
        version: None,
    };
    connection.sender.send(Message::Notification(Notification {
        method: "textDocument/publishDiagnostics".into(),
        params: serde_json::to_value(params)?,
    }))?;
    Ok(())
}

fn position_context(state: &State, uri: &Url, position: Position) -> Option<(PathBuf, String, String)> {
    let path = uri_path(uri)?;
    let text = state.text_of(&path)?;
    let line = text.lines().nth(position.line as usize)?.to_string();
    Some((path, text, line))
}

fn definition(state: &mut State, params: GotoDefinitionParams) -> Result<serde_json::Value> {
    let pos = params.text_document_position_params.position;
    let uri = &params.text_document_position_params.text_document.uri;
    let Some((path, _text, line)) = position_context(state, uri, pos) else {
        return Ok(serde_json::Value::Null);
    };

    // On an include directive: jump to the included file.
    if let Some(index) = state.index.get(&path) {
        let byte_idx = state::utf16_col_to_byte(&line, pos.character);
        for inc in &index.includes {
            // Link-capable clients get the narrow span so only the quoted path
            // is underlined; others keep the whole directive as the trigger.
            let span = if state.definition_link_support {
                inc.path_span
            } else {
                inc.directive_span
            };
            if inc.line == pos.line && byte_idx >= span.0 && byte_idx <= span.1 {
                let Some(target) = &inc.resolved else {
                    return Ok(serde_json::Value::Null);
                };
                let uri = path_to_uri(target).ok_or_else(|| anyhow::anyhow!("bad path"))?;
                // With link support, underline the quoted path as one link
                // instead of letting the client linkify the word under the cursor.
                let response = if state.definition_link_support {
                    GotoDefinitionResponse::Link(vec![LocationLink {
                        origin_selection_range: Some(span_to_range(
                            inc.line,
                            &line,
                            inc.path_span,
                        )),
                        target_uri: uri,
                        target_range: Range::default(),
                        target_selection_range: Range::default(),
                    }])
                } else {
                    GotoDefinitionResponse::Scalar(Location {
                        uri,
                        range: Range::default(),
                    })
                };
                return Ok(serde_json::to_value(response)?);
            }
        }
    }

    // On an identifier: jump to Sub/Function/Class/Property definitions.
    let Some((word, _)) = word_at(&line, pos.character) else {
        return Ok(serde_json::Value::Null);
    };
    let mut locations = Vec::new();
    for (file, sym) in state.find_definitions(&path, &word) {
        // Skip the definition the cursor is already on.
        if file == path && sym.line == pos.line {
            continue;
        }
        if let Some(file_text) = state.text_of(&file) {
            if let Some(loc) = symbol_location(&file, &file_text, sym) {
                locations.push(loc);
            }
        }
    }
    if locations.is_empty() {
        Ok(serde_json::Value::Null)
    } else {
        Ok(serde_json::to_value(GotoDefinitionResponse::Array(locations))?)
    }
}

fn references(state: &mut State, params: ReferenceParams) -> Result<serde_json::Value> {
    let pos = params.text_document_position.position;
    let uri = &params.text_document_position.text_document.uri;
    let Some((path, _text, line)) = position_context(state, uri, pos) else {
        return Ok(serde_json::Value::Null);
    };

    // On an include directive, list every file that includes that same target;
    // anywhere else, list every file that includes the current file.
    let mut target = path.clone();
    if let Some(index) = state.index.get(&path) {
        let byte_idx = state::utf16_col_to_byte(&line, pos.character);
        for inc in &index.includes {
            if inc.line == pos.line
                && byte_idx >= inc.directive_span.0
                && byte_idx <= inc.directive_span.1
            {
                if let Some(resolved) = &inc.resolved {
                    target = resolved.clone();
                }
            }
        }
    }

    let mut locations = Vec::new();
    for (file, inc) in state.includers_of(&target) {
        let Some(text) = state.text_of(&file) else { continue };
        let Some(line) = text.lines().nth(inc.line as usize) else { continue };
        let Some(uri) = path_to_uri(&file) else { continue };
        locations.push(Location {
            uri,
            range: span_to_range(inc.line, line, inc.directive_span),
        });
    }
    Ok(serde_json::to_value(locations)?)
}

fn hover(state: &mut State, params: HoverParams) -> Result<serde_json::Value> {
    let pos = params.text_document_position_params.position;
    let uri = &params.text_document_position_params.text_document.uri;
    let Some((path, _text, line)) = position_context(state, uri, pos) else {
        return Ok(serde_json::Value::Null);
    };
    let Some((word, qualifier)) = word_at(&line, pos.character) else {
        return Ok(serde_json::Value::Null);
    };

    let doc = if let Some(q) = &qualifier {
        builtins::member_doc(q, &word)
            .or_else(|| builtins::ado_member_doc(&word))
    } else {
        builtins::object_doc(&word)
    }
    .or_else(|| {
        // User-defined symbol: show its definition line.
        state
            .find_definitions(&path, &word)
            .first()
            .map(|(file, sym)| {
                let origin = file
                    .strip_prefix(&state.root)
                    .unwrap_or(file)
                    .display();
                format!("```vbscript\n{}\n```\n\n_defined in {}_", sym.signature, origin)
            })
    });

    match doc {
        Some(value) => Ok(serde_json::to_value(Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value,
            }),
            range: None,
        })?),
        None => Ok(serde_json::Value::Null),
    }
}

fn workspace_symbol(state: &mut State, params: WorkspaceSymbolParams) -> Result<serde_json::Value> {
    let query = params.query.to_ascii_lowercase();
    let mut symbols = Vec::new();
    for (file, index) in &state.index {
        for sym in &index.symbols {
            if !query.is_empty() && !sym.name.to_ascii_lowercase().contains(&query) {
                continue;
            }
            let Some(text) = state.text_of(file) else { continue };
            let Some(location) = symbol_location(file, &text, sym) else { continue };
            #[allow(deprecated)]
            symbols.push(SymbolInformation {
                name: sym.name.clone(),
                kind: match sym.kind {
                    SymbolKind::Sub => lsp_types::SymbolKind::METHOD,
                    SymbolKind::Function => lsp_types::SymbolKind::FUNCTION,
                    SymbolKind::Class => lsp_types::SymbolKind::CLASS,
                    SymbolKind::Property => lsp_types::SymbolKind::PROPERTY,
                },
                tags: None,
                deprecated: None,
                location,
                container_name: None,
            });
            if symbols.len() >= 512 {
                return Ok(serde_json::to_value(WorkspaceSymbolResponse::Flat(symbols))?);
            }
        }
    }
    Ok(serde_json::to_value(WorkspaceSymbolResponse::Flat(symbols))?)
}
