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

/// Índice dos inícios de linha de um texto: converte um byte em linha e coluna (em
/// caracteres, para as mensagens; em unidades UTF-16, para o protocolo dos editores) e
/// de volta.
#[derive(Clone, Debug, Default)]
pub struct LineIndex {
    starts: Vec<usize>,
}

impl LineIndex {
    pub fn new(text: &str) -> LineIndex {
        let mut starts = vec![0];
        starts.extend(text.match_indices('\n').map(|(i, _)| i + 1));
        LineIndex { starts }
    }

    /// A linha (a partir de 0) do byte `offset`.
    pub fn line(&self, offset: usize) -> usize {
        match self.starts.binary_search(&offset) {
            Ok(i) => i,
            Err(i) => i - 1,
        }
    }

    /// O byte em que começa a linha (a partir de 0); depois da última, o fim do texto.
    pub fn line_start(&self, line: usize, text: &str) -> usize {
        self.starts.get(line).copied().unwrap_or(text.len())
    }

    pub fn line_count(&self) -> usize {
        self.starts.len()
    }

    /// Linha e coluna a partir de 1, a coluna em caracteres.
    pub fn line_col(&self, text: &str, offset: usize) -> (usize, usize) {
        let offset = floor_boundary(text, offset);
        let line = self.line(offset);
        (line + 1, text[self.starts[line]..offset].chars().count() + 1)
    }

    /// Linha e coluna a partir de 0, a coluna em unidades UTF-16 (como o LSP pede).
    pub fn utf16(&self, text: &str, offset: usize) -> (u32, u32) {
        let offset = floor_boundary(text, offset);
        let line = self.line(offset);
        let col: usize = text[self.starts[line]..offset].chars().map(char::len_utf16).sum();
        (line as u32, col as u32)
    }

    /// O byte da posição (linha, coluna UTF-16), saturado no fim da linha e do texto.
    pub fn offset_utf16(&self, text: &str, line: u32, col: u32) -> usize {
        let start = self.line_start(line as usize, text);
        let end = self.line_start(line as usize + 1, text);
        let mut units = 0u32;
        for (i, c) in text[start..end].char_indices() {
            if units >= col || c == '\n' || c == '\r' {
                return start + i;
            }
            units += c.len_utf16() as u32;
        }
        end.min(text.len())
    }
}

/// O maior limite de caractere que não passa de `offset`.
fn floor_boundary(text: &str, offset: usize) -> usize {
    let mut o = offset.min(text.len());
    while !text.is_char_boundary(o) {
        o -= 1;
    }
    o
}

/// O arquivo-fonte com o índice dos inícios de linha.
pub struct SourceFile {
    pub path: PathBuf,
    pub text: String,
    pub lines: LineIndex,
}

impl SourceFile {
    pub fn new(path: impl Into<PathBuf>, text: impl Into<String>) -> SourceFile {
        let text = text.into();
        let lines = LineIndex::new(&text);
        SourceFile { path: path.into(), text, lines }
    }

    /// Nome curto do arquivo para as mensagens.
    pub fn name(&self) -> String {
        self.path.display().to_string()
    }

    /// Linha (a partir de 1) e coluna em caracteres (a partir de 1).
    pub fn line_col(&self, offset: u32) -> (usize, usize) {
        self.lines.line_col(&self.text, offset as usize)
    }

    pub fn line_of(&self, offset: u32) -> usize {
        self.line_col(offset).0
    }

    /// Texto da linha `line` (a partir de 1), sem a quebra.
    pub fn line_text(&self, line: usize) -> &str {
        let start = self.lines.line_start(line - 1, &self.text);
        let end = self.lines.line_start(line, &self.text);
        self.text[start..end].trim_end_matches(['\n', '\r'])
    }

    pub fn line_count(&self) -> usize {
        self.lines.line_count()
    }

    pub fn slice(&self, span: Span) -> &str {
        &self.text[span.start as usize..span.end as usize]
    }
}

/// Os arquivos de uma compilação, cada um numa faixa própria de posições: o primeiro
/// começa em 0 e cada um seguinte começa depois do fim do anterior. Assim um `Span`
/// identifica o arquivo sem levar o arquivo junto (como no rustc).
#[derive(Default)]
pub struct SourceMap {
    files: Vec<(u32, SourceFile)>,
}

impl SourceMap {
    /// Acrescenta um arquivo e devolve a base dele (a posição global do primeiro byte).
    pub fn add(&mut self, path: impl Into<PathBuf>, text: impl Into<String>) -> u32 {
        let base = self.files.last().map_or(0, |(b, f)| b + f.text.len() as u32 + 1);
        self.files.push((base, SourceFile::new(path, text)));
        base
    }

    /// O arquivo de uma posição global e o trecho dentro dele.
    pub fn lookup(&self, span: Span) -> Option<(&SourceFile, Span)> {
        let i = self.files.partition_point(|(b, _)| *b <= span.start).checked_sub(1)?;
        let (base, file) = &self.files[i];
        Some((file, Span { start: span.start - base, end: span.end.saturating_sub(*base) }))
    }

    pub fn files(&self) -> impl Iterator<Item = &SourceFile> {
        self.files.iter().map(|(_, f)| f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn columns_in_chars_and_utf16() {
        let text = "Para começar:\n    Escrevo \"ação 𝄞\".\n";
        let idx = LineIndex::new(text);
        let at = text.find("𝄞").unwrap();
        assert_eq!(idx.line_col(text, at), (2, 19));
        // "ç" e "ã" valem uma unidade UTF-16; a clave de sol vale duas.
        assert_eq!(idx.utf16(text, at), (1, 18));
        assert_eq!(idx.utf16(text, at + "𝄞".len()), (1, 20));
        assert_eq!(idx.offset_utf16(text, 1, 18), at);
        assert_eq!(idx.offset_utf16(text, 1, 999), text.len() - 1);
        assert_eq!(idx.offset_utf16(text, 9, 0), text.len());
        // Posição no meio de um caractere cai no começo dele.
        assert_eq!(idx.line_col(text, text.find("ç").unwrap() + 1), (1, 10));
    }

    #[test]
    fn source_map_finds_the_file() {
        let mut map = SourceMap::default();
        let a = map.add("a.lav", "Escrevo 1.\n");
        let b = map.add("b.lav", "Escrevo 2.\n");
        assert_eq!(a, 0);
        let (f, local) = map.lookup(Span { start: b + 8, end: b + 9 }).unwrap();
        assert_eq!((f.name().as_str(), f.slice(local)), ("b.lav", "2"));
        let (f, local) = map.lookup(Span { start: 8, end: 9 }).unwrap();
        assert_eq!((f.name().as_str(), f.slice(local)), ("a.lav", "1"));
    }
}
