//! Posições no texto-fonte: intervalos em bytes e conversão para linha e coluna.

use std::path::PathBuf;

/// Intervalo `[start, end)` em bytes dentro do arquivo-fonte.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Span {
    pub start: u32,
    pub end: u32,
}

impl Span {
    pub fn new(start: usize, end: usize) -> Span {
        Span { start: start as u32, end: end as u32 }
    }

    /// O menor intervalo que cobre os dois.
    pub fn to(self, other: Span) -> Span {
        Span { start: self.start.min(other.start), end: self.end.max(other.end) }
    }

    pub fn is_empty(self) -> bool {
        self.start >= self.end
    }
}

/// O arquivo-fonte com o índice dos inícios de linha.
pub struct SourceFile {
    pub path: PathBuf,
    pub text: String,
    line_starts: Vec<usize>,
}

impl SourceFile {
    pub fn new(path: impl Into<PathBuf>, text: impl Into<String>) -> SourceFile {
        let text = text.into();
        let mut line_starts = vec![0];
        for (i, b) in text.bytes().enumerate() {
            if b == b'\n' {
                line_starts.push(i + 1);
            }
        }
        SourceFile { path: path.into(), text, line_starts }
    }

    /// Nome curto do arquivo para as mensagens.
    pub fn name(&self) -> String {
        self.path.display().to_string()
    }

    /// Linha (a partir de 1) e coluna em caracteres (a partir de 1).
    pub fn line_col(&self, offset: u32) -> (usize, usize) {
        let offset = (offset as usize).min(self.text.len());
        let line = match self.line_starts.binary_search(&offset) {
            Ok(i) => i,
            Err(i) => i - 1,
        };
        let start = self.line_starts[line];
        let col = self.text[start..offset].chars().count() + 1;
        (line + 1, col)
    }

    pub fn line_of(&self, offset: u32) -> usize {
        self.line_col(offset).0
    }

    /// Texto da linha `line` (a partir de 1), sem a quebra.
    pub fn line_text(&self, line: usize) -> &str {
        let start = self.line_starts[line - 1];
        let end = self.line_starts.get(line).copied().unwrap_or(self.text.len());
        self.text[start..end].trim_end_matches(['\n', '\r'])
    }

    pub fn line_count(&self) -> usize {
        self.line_starts.len()
    }

    pub fn slice(&self, span: Span) -> &str {
        &self.text[span.start as usize..span.end as usize]
    }
}
