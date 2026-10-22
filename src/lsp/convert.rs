//! Conversões entre o compilador e o protocolo: posições (bytes do texto contra linha e
//! coluna em UTF-8 ou UTF-16, conforme o cliente) e diagnósticos.

use crate::diagnostics::{Category, Diagnostic};
use crate::span::{LineIndex, Span};
use lsp_types as lsp;

/// Como o cliente conta as colunas.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Encoding {
    Utf8,
    Utf16,
}

/// Um documento aberto: o texto e o índice das linhas.
pub struct Text<'a> {
    pub text: &'a str,
    pub lines: &'a LineIndex,
    pub encoding: Encoding,
}

impl Text<'_> {
    pub fn position(&self, offset: u32) -> lsp::Position {
        let offset = offset as usize;
        match self.encoding {
            Encoding::Utf16 => {
                let (line, character) = self.lines.utf16(self.text, offset);
                lsp::Position { line, character }
            }
            Encoding::Utf8 => {
                let offset = offset.min(self.text.len());
                let line = self.lines.line(offset);
                let start = self.lines.line_start(line, self.text);
                lsp::Position { line: line as u32, character: (offset - start) as u32 }
            }
        }
    }

    pub fn range(&self, span: Span) -> lsp::Range {
        lsp::Range { start: self.position(span.start), end: self.position(span.end) }
    }

    pub fn offset(&self, pos: lsp::Position) -> u32 {
        let o = match self.encoding {
            Encoding::Utf16 => self.lines.offset_utf16(self.text, pos.line, pos.character),
            Encoding::Utf8 => {
                let start = self.lines.line_start(pos.line as usize, self.text);
                let end = self.lines.line_start(pos.line as usize + 1, self.text);
                let mut o = (start + pos.character as usize).min(end).min(self.text.len());
                while !self.text.is_char_boundary(o) {
                    o -= 1;
                }
                o
            }
        };
        o as u32
    }
}

/// O link da explicação de um código.
/// Uma edição de workspace sobre um documento só.
// O `lsp-types` define `changes` com `Uri` como chave, e o `Uri` guarda um cache interno
// (mutabilidade interior); a chave nunca muda depois de entrar no mapa.
#[allow(clippy::mutable_key_type)]
pub fn workspace_edit(uri: &lsp::Uri, edits: Vec<lsp::TextEdit>) -> lsp::WorkspaceEdit {
    let changes = std::collections::HashMap::from([(uri.clone(), edits)]);
    lsp::WorkspaceEdit { changes: Some(changes), ..Default::default() }
}

pub fn explain_url(code: &str) -> Option<lsp::Uri> {
    format!("https://github.com/JoaoHenriqueBarbosa/lavra/blob/main/src/explain/{code}.md").parse().ok()
}

/// Um diagnóstico do compilador no formato do protocolo. A dica e as notas vão junto da
/// mensagem; os rótulos viram informação relacionada; as correções viajam em `data`, para
/// as ações rápidas.
pub fn diagnostic(d: &Diagnostic, text: &Text, uri: &lsp::Uri) -> lsp::Diagnostic {
    let mut message = d.message.clone();
    if let Some(p) = &d.primary {
        message.push_str(&format!(" ({p})"));
    }
    for n in &d.notes {
        message.push_str(&format!("\n{n}"));
    }
    if let Some(h) = &d.hint {
        message.push_str(&format!("\nDica: {h}"));
    }
    let related: Vec<lsp::DiagnosticRelatedInformation> = d
        .labels
        .iter()
        .map(|l| lsp::DiagnosticRelatedInformation {
            location: lsp::Location { uri: uri.clone(), range: text.range(l.span) },
            message: l.text.clone(),
        })
        .collect();
    let unnecessary = matches!(d.code, Some("L0501" | "L0502" | "L0503"));
    let fixes: Vec<serde_json::Value> = d
        .fixes
        .iter()
        .map(|f| {
            serde_json::json!({
                "title": f.title,
                "safe": f.safe,
                "edits": f.edits.iter().map(|e| serde_json::json!({ "range": text.range(e.span), "text": e.text })).collect::<Vec<_>>(),
            })
        })
        .collect();
    lsp::Diagnostic {
        range: text.range(if d.span.is_empty() && d.span.start == 0 { Span::new(0, 0) } else { d.span }),
        severity: Some(if d.category == Category::Warning { lsp::DiagnosticSeverity::WARNING } else { lsp::DiagnosticSeverity::ERROR }),
        code: d.code.map(|c| lsp::NumberOrString::String(c.to_string())),
        code_description: d.code.and_then(explain_url).map(|href| lsp::CodeDescription { href }),
        source: Some("lavra".into()),
        message,
        related_information: (!related.is_empty()).then_some(related),
        tags: unnecessary.then(|| vec![lsp::DiagnosticTag::UNNECESSARY]),
        data: (!fixes.is_empty()).then(|| serde_json::json!({ "fixes": fixes })),
    }
}
