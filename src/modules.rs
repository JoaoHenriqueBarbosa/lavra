//! Módulos: `Uso o arquivo "contas.lav".` traz para o programa as funções, os tipos e as
//! constantes de outro arquivo Lavra. O arquivo trazido é lido aqui, antes do parser: os
//! tokens dele entram na frente dos do programa, com as posições numa faixa própria (como
//! as bibliotecas), e daí em diante a análise é uma só. O "Para começar" de um arquivo
//! trazido não entra: cada arquivo continua podendo rodar sozinho.

use crate::codes::*;
use crate::diagnostics::{Diagnostic, Diagnostics, Style};
use crate::lexer::{self, Token, TokenKind};
use crate::span::{SourceFile, Span};
use std::path::{Component, Path, PathBuf};

/// O começo da faixa de posições dos arquivos trazidos: acima das bibliotecas, para que tudo
/// o que já trata "fora do arquivo do programa" valha também para eles.
pub const MODULE_BASE: u32 = crate::builtins::LIBRARY_BASE + (1 << 28);
/// O tamanho da faixa de cada arquivo trazido (4 MiB).
pub const MODULE_SIZE: u32 = 1 << 22;
/// Quantos arquivos um programa pode trazer, contando os que eles trazem.
const MAX_MODULES: usize = 256;

/// Quem lê um arquivo do disco (ou de onde o editor guarda os textos abertos).
pub type Reader<'a> = &'a dyn Fn(&Path) -> std::io::Result<String>;

/// A posição é de um arquivo trazido, e não do programa nem de uma biblioteca.
pub fn is_module(pos: u32) -> bool {
    pos >= MODULE_BASE
}

/// Os arquivos de uma análise: o do programa fica com quem chamou; aqui, os trazidos, cada
/// um com a base das posições dele.
#[derive(Default)]
pub struct Sources {
    pub modules: Vec<(u32, SourceFile)>,
    /// De cada arquivo trazido, o pedido no arquivo do programa por onde ele entrou (o
    /// próprio pedido, ou o do arquivo que o trouxe).
    pub origins: Vec<Span>,
}

impl Sources {
    /// O arquivo trazido que contém a posição.
    pub fn module_at(&self, pos: u32) -> Option<&(u32, SourceFile)> {
        if !is_module(pos) {
            return None;
        }
        self.modules.get(((pos - MODULE_BASE) / MODULE_SIZE) as usize)
    }

    /// Para o editor, que só mostra o arquivo aberto: o erro de um arquivo trazido aparece
    /// no pedido que o trouxe, dizendo o arquivo e a linha.
    pub fn at_request(&self, d: &Diagnostic) -> Option<Diagnostic> {
        let i = ((d.span.start.checked_sub(MODULE_BASE)?) / MODULE_SIZE) as usize;
        let (base, file) = self.modules.get(i)?;
        let line = file.line_col(d.span.start - base).0;
        let mut out = d.clone();
        out.message = format!("No arquivo \"{}\", linha {line}: {}", file.name(), d.message);
        out.span = *self.origins.get(i)?;
        out.primary = None;
        out.labels.clear();
        out.fixes.clear();
        Some(out)
    }

    /// A linha (a partir de 1) de uma posição e, se ela é de um arquivo trazido, o nome dele.
    pub fn line(&self, main: &str, lines: &crate::span::LineIndex, pos: u32) -> (usize, Option<String>) {
        match self.module_at(pos) {
            Some((base, file)) => (file.line_col(pos - base).0, Some(file.name())),
            None => (lines.line_col(main, pos as usize).0, None),
        }
    }

    /// O diagnóstico no arquivo em que ele está, com as posições contadas nesse arquivo. Os
    /// rótulos e as correções que caem em outro arquivo saem: cada arquivo se mostra sozinho.
    pub fn localize<'a>(&'a self, main: &'a SourceFile, d: &Diagnostic) -> (&'a SourceFile, Diagnostic) {
        let (base, file) = match self.module_at(d.span.start) {
            Some((base, file)) => (*base, file),
            None => (0, main),
        };
        let here = |s: Span| if base == 0 { !is_module(s.start) } else { self.module_at(s.start).is_some_and(|(b, _)| *b == base) };
        let shift = |s: Span| Span { start: s.start - base, end: s.end - base };
        let mut local = d.clone();
        local.span = shift(d.span);
        // O rótulo que cai em outro arquivo vira uma nota com o lugar.
        for l in d.labels.iter().filter(|l| !here(l.span)) {
            let place = match self.module_at(l.span.start) {
                Some((b, f)) => format!("{}:{}", f.name(), f.line_col(l.span.start - b).0),
                None => format!("{}:{}", main.name(), main.line_col(l.span.start).0),
            };
            local.notes.push(format!("{}: {place}", l.text));
        }
        local.labels.retain(|l| here(l.span));
        for l in &mut local.labels {
            l.span = shift(l.span);
        }
        local.fixes.retain(|f| f.edits.iter().all(|e| here(e.span)));
        for e in local.fixes.iter_mut().flat_map(|f| f.edits.iter_mut()) {
            e.span = shift(e.span);
        }
        (file, local)
    }

    /// O diagnóstico como o terminal o mostra, no arquivo em que ele está.
    pub fn render(&self, main: &SourceFile, d: &Diagnostic, style: Style) -> String {
        let (file, local) = self.localize(main, d);
        crate::diagnostics::render(&local, file, style)
    }

    /// Os arquivos trazidos num bloco só, para o executável nativo levar junto: de cada um,
    /// a base, o tamanho do caminho e o do texto (três inteiros de 32 bits), e os dois.
    pub fn blob(&self) -> Vec<u8> {
        let mut out = Vec::new();
        for (base, file) in &self.modules {
            let path = file.name();
            out.extend(base.to_le_bytes());
            out.extend((path.len() as u32).to_le_bytes());
            out.extend((file.text.len() as u32).to_le_bytes());
            out.extend(path.as_bytes());
            out.extend(file.text.as_bytes());
        }
        out
    }
}

/// Um pedido de arquivo no nível de fora: os tokens da frase e o caminho escrito.
struct Request {
    /// Os índices do primeiro token da frase e do que vem depois dela.
    tokens: (usize, usize),
    path: Option<(String, Span)>,
    span: Span,
}

/// Os pedidos `Uso o arquivo "...".` de um texto já em tokens. Só valem no nível de fora e
/// no começo da linha, como o pedido de biblioteca.
fn requests(tokens: &[Token]) -> Vec<Request> {
    let mut out = Vec::new();
    let mut depth = 0usize;
    let mut at_start = true;
    let mut k = 0;
    while k < tokens.len() {
        let t = &tokens[k];
        match t.kind {
            TokenKind::Indent => depth += 1,
            TokenKind::Dedent => depth = depth.saturating_sub(1),
            _ => {}
        }
        let word = |i: usize, w: &str| tokens.get(k + i).is_some_and(|x| x.is_word(w) && x.contraction.is_none());
        if depth == 0 && at_start && word(0, "uso") && word(1, "o") && word(2, "arquivo") {
            let mut end = k;
            while end < tokens.len() && !matches!(tokens[end].kind, TokenKind::Newline | TokenKind::Eof) {
                end += 1;
            }
            let path = match (tokens.get(k + 3).map(|x| (&x.kind, x.span)), tokens.get(k + 4)) {
                (Some((TokenKind::Text(p), span)), Some(dot)) if dot.is_punct('.') && k + 5 == end => Some((p.clone(), span)),
                _ => None,
            };
            let last = tokens[end.saturating_sub(1).max(k)].span;
            out.push(Request { tokens: (k, end), path, span: t.span.to(last) });
            k = end;
            continue;
        }
        at_start = matches!(t.kind, TokenKind::Newline | TokenKind::Dedent | TokenKind::Indent);
        k += 1;
    }
    out
}

/// O caminho sem "." e sem "pasta/..", para reconhecer o mesmo arquivo pedido de dois jeitos.
fn normalize(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for c in path.components() {
        match c {
            Component::CurDir => {}
            Component::ParentDir if out.file_name().is_some() => {
                out.pop();
            }
            other => out.push(other),
        }
    }
    out
}

struct Loader<'a> {
    read: Reader<'a>,
    diags: &'a mut Diagnostics,
    sources: Sources,
    /// Os arquivos já lidos (ou em leitura): cada um entra uma vez, e um ciclo se fecha sozinho.
    seen: Vec<PathBuf>,
    /// Os tokens dos arquivos trazidos, na ordem em que entram no programa.
    tokens: Vec<Token>,
}

impl Loader<'_> {
    /// Lê os arquivos que `tokens` pede (os de `dir`), cada um depois dos que ele mesmo
    /// pede, e tira as frases de pedido de `tokens`.
    fn load(&mut self, tokens: &mut Vec<Token>, dir: &Path, origin: Option<Span>) {
        let found = requests(tokens);
        for r in &found {
            let Some((written, path_span)) = &r.path else {
                self.diags.push(
                    Diagnostic::new(L0177, "O arquivo se pede assim: \"Uso o arquivo \\\"contas.lav\\\".\"", r.span)
                        .hint("O caminho vai entre aspas e conta a partir da pasta deste arquivo."),
                );
                continue;
            };
            let path = normalize(&dir.join(written));
            if self.seen.contains(&path) {
                continue;
            }
            self.seen.push(path.clone());
            if self.sources.modules.len() >= MAX_MODULES {
                self.diags.push(Diagnostic::new(L0178, format!("Arquivos demais: um programa traz até {MAX_MODULES}."), *path_span));
                continue;
            }
            let text = match (self.read)(&path) {
                Ok(text) if text.len() < MODULE_SIZE as usize => text,
                Ok(_) => {
                    self.diags.push(Diagnostic::new(
                        L0178,
                        format!("O arquivo \"{written}\" é grande demais para ser trazido."),
                        *path_span,
                    ));
                    continue;
                }
                Err(e) => {
                    let why = crate::textfmt::io_reason(e.kind());
                    self.diags.push(
                        Diagnostic::new(L0178, format!("Não consegui trazer o arquivo \"{written}\": {why}."), *path_span)
                            .hint(format!("Procurei em \"{}\". O caminho conta a partir da pasta deste arquivo.", path.display())),
                    );
                    continue;
                }
            };
            let base = MODULE_BASE + self.sources.modules.len() as u32 * MODULE_SIZE;
            let mut module = lexer::tokenize_at(&text, base as usize, self.diags);
            self.sources.modules.push((base, SourceFile::new(path.clone(), text)));
            self.sources.origins.push(origin.unwrap_or(r.span));
            self.load(&mut module, path.parent().unwrap_or(Path::new("")), Some(origin.unwrap_or(r.span)));
            // O fim de arquivo é um só, o do programa.
            module.retain(|t| !matches!(t.kind, TokenKind::Eof));
            self.tokens.extend(module);
        }
        // As frases de pedido saem: o parser lê só o que os arquivos definem.
        for r in found.iter().rev() {
            let end = (r.tokens.1 + 1).min(tokens.len());
            let keeps_eof = tokens.get(r.tokens.1).is_some_and(|t| matches!(t.kind, TokenKind::Eof));
            tokens.drain(r.tokens.0..if keeps_eof { r.tokens.1 } else { end });
        }
    }
}

/// Traz os arquivos que o programa pede. Devolve os tokens do programa inteiro (os dos
/// arquivos trazidos na frente, cada um depois dos que ele mesmo pede) e os arquivos lidos.
pub fn load(main: Vec<Token>, path: Option<&Path>, read: Reader, diags: &mut Diagnostics) -> (Vec<Token>, Sources) {
    let mut tokens = main;
    let dir = path.and_then(Path::parent).unwrap_or(Path::new("")).to_path_buf();
    let mut loader = Loader { read, diags, sources: Sources::default(), seen: Vec::new(), tokens: Vec::new() };
    if let Some(p) = path {
        loader.seen.push(normalize(p));
    }
    loader.load(&mut tokens, &dir, None);
    let mut all = std::mem::take(&mut loader.tokens);
    all.extend(tokens);
    (all, loader.sources)
}
