//! O servidor de linguagem da Lavra (`lavrac lsp`): diagnósticos ao digitar, tokens
//! semânticos, hover com tipo e posse, definição, referências, destaque e símbolos.
//!
//! Síncrono, sobre stdio. Cada mudança reanalisa o documento inteiro (os programas são
//! pequenos, e `lavra::analyze` não para no primeiro erro); cada requisição roda isolada,
//! e um defeito numa delas vira erro só dela, sem derrubar o servidor.

mod convert;
mod features;
pub mod semantic;

use crate::span::LineIndex;
use crate::{Analysis, analyze};
use convert::{Encoding, Text};
use lsp_server::{Connection, ErrorCode, Message, Notification, Request, Response};
use lsp_types as lsp;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::error::Error;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};

type Fallible<T> = Result<T, Box<dyn Error + Send + Sync>>;

/// Roda o servidor sobre a entrada e a saída padrão.
pub fn run_stdio() -> Fallible<()> {
    let (conn, io) = Connection::stdio();
    run(&conn)?;
    io.join()?;
    Ok(())
}

/// Um documento aberto e a análise dele.
struct Doc {
    uri: lsp::Uri,
    text: String,
    lines: LineIndex,
    analysis: Analysis,
}

impl Doc {
    fn new(uri: lsp::Uri, text: String) -> Doc {
        let lines = LineIndex::new(&text);
        let analysis = analyze(&text);
        Doc { uri, text, lines, analysis }
    }
}

struct Server {
    encoding: Encoding,
    docs: HashMap<String, Doc>,
    roots: Vec<PathBuf>,
}

/// Roda o servidor numa conexão já aberta (os testes usam uma em memória).
pub fn run(conn: &Connection) -> Fallible<()> {
    let (id, params) = conn.initialize_start()?;
    let init: lsp::InitializeParams = serde_json::from_value(params)?;
    let utf8 = init
        .capabilities
        .general
        .as_ref()
        .and_then(|g| g.position_encodings.as_ref())
        .is_some_and(|e| e.contains(&lsp::PositionEncodingKind::UTF8));
    let encoding = if utf8 { Encoding::Utf8 } else { Encoding::Utf16 };
    #[allow(deprecated)] // `root_uri` ainda é o que muitos clientes mandam.
    let roots: Vec<PathBuf> = init
        .workspace_folders
        .unwrap_or_default()
        .iter()
        .map(|f| f.uri.clone())
        .chain(init.root_uri)
        .filter_map(|u| uri_to_path(u.as_str()))
        .collect();
    let caps = lsp::ServerCapabilities {
        position_encoding: Some(if utf8 { lsp::PositionEncodingKind::UTF8 } else { lsp::PositionEncodingKind::UTF16 }),
        text_document_sync: Some(lsp::TextDocumentSyncCapability::Kind(lsp::TextDocumentSyncKind::FULL)),
        hover_provider: Some(lsp::HoverProviderCapability::Simple(true)),
        definition_provider: Some(lsp::OneOf::Left(true)),
        references_provider: Some(lsp::OneOf::Left(true)),
        document_highlight_provider: Some(lsp::OneOf::Left(true)),
        document_symbol_provider: Some(lsp::OneOf::Left(true)),
        workspace_symbol_provider: Some(lsp::OneOf::Left(true)),
        semantic_tokens_provider: Some(lsp::SemanticTokensServerCapabilities::SemanticTokensOptions(lsp::SemanticTokensOptions {
            legend: semantic::legend(),
            full: Some(lsp::SemanticTokensFullOptions::Bool(true)),
            range: None,
            work_done_progress_options: Default::default(),
        })),
        ..Default::default()
    };
    conn.initialize_finish(id, json!({ "capabilities": caps, "serverInfo": { "name": "lavrac", "version": env!("CARGO_PKG_VERSION") } }))?;
    let mut server = Server { encoding, docs: HashMap::new(), roots };
    for msg in &conn.receiver {
        match msg {
            Message::Request(req) => {
                if conn.handle_shutdown(&req)? {
                    return Ok(());
                }
                let id = req.id.clone();
                let resp = match catch_unwind(AssertUnwindSafe(|| server.request(req))) {
                    Ok(Ok(v)) => Response::new_ok(id, v),
                    Ok(Err((code, msg))) => Response::new_err(id, code as i32, msg),
                    Err(_) => Response::new_err(id, ErrorCode::InternalError as i32, "defeito no servidor da Lavra".into()),
                };
                conn.sender.send(Message::Response(resp))?;
            }
            Message::Notification(n) => {
                let out = catch_unwind(AssertUnwindSafe(|| server.notification(n))).unwrap_or_default();
                for n in out {
                    conn.sender.send(Message::Notification(n))?;
                }
            }
            Message::Response(_) => {}
        }
    }
    Ok(())
}

type Answer = Result<Value, (ErrorCode, String)>;

fn params<P: serde::de::DeserializeOwned>(req: &Request) -> Result<P, (ErrorCode, String)> {
    serde_json::from_value(req.params.clone()).map_err(|e| (ErrorCode::InvalidParams, e.to_string()))
}

impl Server {
    fn text<'a>(&self, doc: &'a Doc) -> Text<'a> {
        Text { text: &doc.text, lines: &doc.lines, encoding: self.encoding }
    }

    fn doc(&self, uri: &lsp::Uri) -> Result<&Doc, (ErrorCode, String)> {
        self.docs.get(uri.as_str()).ok_or((ErrorCode::InvalidParams, "documento não está aberto".into()))
    }

    /// O documento e a posição em bytes de um parâmetro de posição.
    fn at(&self, p: &lsp::TextDocumentPositionParams) -> Result<(&Doc, u32), (ErrorCode, String)> {
        let doc = self.doc(&p.text_document.uri)?;
        let offset = self.text(doc).offset(p.position);
        Ok((doc, offset))
    }

    fn request(&mut self, req: Request) -> Answer {
        let v = match req.method.as_str() {
            "textDocument/hover" => {
                let p: lsp::HoverParams = params(&req)?;
                let (doc, offset) = self.at(&p.text_document_position_params)?;
                json!(features::hover(&doc.analysis, &self.text(doc), offset))
            }
            "textDocument/definition" => {
                let p: lsp::GotoDefinitionParams = params(&req)?;
                let (doc, offset) = self.at(&p.text_document_position_params)?;
                json!(features::definition(&doc.analysis, &self.text(doc), &doc.uri, offset))
            }
            "textDocument/references" => {
                let p: lsp::ReferenceParams = params(&req)?;
                let (doc, offset) = self.at(&p.text_document_position)?;
                json!(features::references(&doc.analysis, &self.text(doc), &doc.uri, offset, p.context.include_declaration))
            }
            "textDocument/documentHighlight" => {
                let p: lsp::DocumentHighlightParams = params(&req)?;
                let (doc, offset) = self.at(&p.text_document_position_params)?;
                json!(features::highlights(&doc.analysis, &self.text(doc), offset))
            }
            "textDocument/documentSymbol" => {
                let p: lsp::DocumentSymbolParams = params(&req)?;
                let doc = self.doc(&p.text_document.uri)?;
                json!(features::document_symbols(&doc.analysis, &self.text(doc)))
            }
            "textDocument/semanticTokens/full" => {
                let p: lsp::SemanticTokensParams = params(&req)?;
                let doc = self.doc(&p.text_document.uri)?;
                json!(lsp::SemanticTokens { result_id: None, data: semantic::tokens(&doc.analysis, &self.text(doc)) })
            }
            "workspace/symbol" => {
                let p: lsp::WorkspaceSymbolParams = params(&req)?;
                json!(self.workspace_symbols(&p.query))
            }
            other => return Err((ErrorCode::MethodNotFound, format!("método não suportado: {other}"))),
        };
        Ok(v)
    }

    fn notification(&mut self, n: Notification) -> Vec<Notification> {
        match n.method.as_str() {
            "textDocument/didOpen" => {
                let Ok(p) = serde_json::from_value::<lsp::DidOpenTextDocumentParams>(n.params) else { return Vec::new() };
                let key = p.text_document.uri.as_str().to_string();
                self.docs.insert(key.clone(), Doc::new(p.text_document.uri, p.text_document.text));
                self.publish(&key)
            }
            "textDocument/didChange" => {
                let Ok(p) = serde_json::from_value::<lsp::DidChangeTextDocumentParams>(n.params) else { return Vec::new() };
                let Some(change) = p.content_changes.into_iter().last() else { return Vec::new() };
                let key = p.text_document.uri.as_str().to_string();
                self.docs.insert(key.clone(), Doc::new(p.text_document.uri, change.text));
                self.publish(&key)
            }
            "textDocument/didClose" => {
                let Ok(p) = serde_json::from_value::<lsp::DidCloseTextDocumentParams>(n.params) else { return Vec::new() };
                self.docs.remove(p.text_document.uri.as_str());
                let params = lsp::PublishDiagnosticsParams { uri: p.text_document.uri, diagnostics: Vec::new(), version: None };
                vec![Notification::new("textDocument/publishDiagnostics".into(), params)]
            }
            _ => Vec::new(),
        }
    }

    fn publish(&self, key: &str) -> Vec<Notification> {
        let Some(doc) = self.docs.get(key) else { return Vec::new() };
        let text = self.text(doc);
        let diagnostics = doc.analysis.diagnostics.iter().map(|d| convert::diagnostic(d, &text, &doc.uri)).collect();
        let params = lsp::PublishDiagnosticsParams { uri: doc.uri.clone(), diagnostics, version: None };
        vec![Notification::new("textDocument/publishDiagnostics".into(), params)]
    }

    /// Funções, tipos e constantes dos documentos abertos e dos `.lav` das pastas do
    /// workspace, filtrados pela consulta.
    #[allow(deprecated)] // `SymbolInformation::deprecated` é campo obrigatório do tipo.
    fn workspace_symbols(&self, query: &str) -> Vec<lsp::SymbolInformation> {
        let query = query.to_lowercase();
        let mut out = Vec::new();
        let mut seen: Vec<String> = Vec::new();
        let mut add = |uri: &lsp::Uri, text: &str, a: &Analysis| {
            let lines = LineIndex::new(text);
            let t = Text { text, lines: &lines, encoding: self.encoding };
            for s in features::document_symbols(a, &t) {
                if !query.is_empty() && !s.name.to_lowercase().contains(&query) {
                    continue;
                }
                out.push(lsp::SymbolInformation {
                    name: s.name,
                    kind: s.kind,
                    tags: None,
                    deprecated: None,
                    location: lsp::Location { uri: uri.clone(), range: s.selection_range },
                    container_name: None,
                });
            }
        };
        for doc in self.docs.values() {
            seen.push(doc.uri.as_str().to_string());
            add(&doc.uri, &doc.text, &doc.analysis);
        }
        for root in &self.roots {
            for path in lav_files(root, 4) {
                let Some(uri) = path_to_uri(&path) else { continue };
                if seen.iter().any(|s| s == uri.as_str()) {
                    continue;
                }
                if let Ok(text) = std::fs::read_to_string(&path) {
                    let a = analyze(&text);
                    add(&uri, &text, &a);
                }
            }
        }
        out
    }
}

/// Os `.lav` de uma pasta, até a profundidade dada, sem entrar em pastas ocultas nem em
/// `target` e `node_modules`.
fn lav_files(dir: &Path, depth: usize) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else { return out };
    for e in entries.flatten() {
        let p = e.path();
        let name = e.file_name().to_string_lossy().to_string();
        if p.is_dir() {
            if depth > 0 && !name.starts_with('.') && name != "target" && name != "node_modules" {
                out.extend(lav_files(&p, depth - 1));
            }
        } else if name.ends_with(".lav") {
            out.push(p);
        }
    }
    out
}

/// `file:///home/x/a%20b.lav` vira o caminho, decodificado.
pub fn uri_to_path(uri: &str) -> Option<PathBuf> {
    let rest = uri.strip_prefix("file://")?;
    let bytes = rest.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).ok()?;
            out.push(u8::from_str_radix(hex, 16).ok()?);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    Some(PathBuf::from(String::from_utf8(out).ok()?))
}

/// O caminho como `file://`, com os bytes fora do conjunto seguro codificados.
pub fn path_to_uri(path: &Path) -> Option<lsp::Uri> {
    let p = path.to_str()?;
    let mut s = String::from("file://");
    for b in p.bytes() {
        if b.is_ascii_alphanumeric() || b"/-_.~".contains(&b) {
            s.push(b as char);
        } else {
            s.push_str(&format!("%{b:02X}"));
        }
    }
    s.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uris_and_paths() {
        let p = PathBuf::from("/home/ana/meus programas/ação.lav");
        let uri = path_to_uri(&p).unwrap();
        assert_eq!(uri.as_str(), "file:///home/ana/meus%20programas/a%C3%A7%C3%A3o.lav");
        assert_eq!(uri_to_path(uri.as_str()).unwrap(), p);
    }
}
