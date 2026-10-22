//! Léxico: quebra o texto em palavras, números, textos e pontuação; calcula os recuos
//! (INDENT/DEDENT); desfaz as contrações e junta as locuções.
//!
//! Comentário é a linha que começa com "Nota:" ou "--" (e
//! "--" no meio da linha corta o resto); a linha só termina o período se acabar em
//! ".", ":" ou ";", senão continua na de baixo com recuo livre (enjambement).

use crate::diagnostics::{Category, Diagnostic, Result, err};
use crate::morphology::split_contraction;
use crate::span::Span;

#[derive(Clone, Debug, PartialEq)]
pub enum TokenKind {
    /// Palavra em minúsculas (locuções vêm juntas, com espaço: "maior que").
    Word(String),
    Int(i128),
    Real(f64),
    Text(String),
    Char(char),
    Punct(char),
    Indent,
    Dedent,
    Newline,
    Eof,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Contraction {
    /// A forma escrita ("da").
    pub written: &'static str,
    /// 1 para a preposição, 2 para o artigo.
    pub part: u8,
}

#[derive(Clone, Debug)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
    /// Grafia original (com maiúsculas), para as mensagens.
    pub orig: String,
    pub contraction: Option<Contraction>,
    /// Primeiro token da linha física.
    pub line_start: bool,
}

impl Token {
    pub fn word(&self) -> Option<&str> {
        match &self.kind {
            TokenKind::Word(w) => Some(w),
            _ => None,
        }
    }

    pub fn is_word(&self, w: &str) -> bool {
        self.word() == Some(w)
    }

    pub fn is_punct(&self, c: char) -> bool {
        self.kind == TokenKind::Punct(c)
    }

    /// Descrição para mensagens ("a palavra \"seja\"", "o ponto final").
    pub fn describe(&self) -> String {
        match &self.kind {
            TokenKind::Word(_) => match self.contraction {
                Some(c) => format!("\"{}\"", c.written),
                None => format!("\"{}\"", self.orig),
            },
            TokenKind::Int(_) | TokenKind::Real(_) => format!("o número {}", self.orig),
            TokenKind::Text(_) => "um texto entre aspas".to_string(),
            TokenKind::Char(_) => "um caractere entre aspas simples".to_string(),
            TokenKind::Punct('.') => "o ponto final".to_string(),
            TokenKind::Punct(',') => "a vírgula".to_string(),
            TokenKind::Punct(':') => "os dois-pontos".to_string(),
            TokenKind::Punct(';') => "o ponto e vírgula".to_string(),
            TokenKind::Punct('(') => "o parêntese de abertura".to_string(),
            TokenKind::Punct(')') => "o parêntese de fechamento".to_string(),
            TokenKind::Punct(c) => format!("\"{c}\""),
            TokenKind::Indent => "um recuo".to_string(),
            TokenKind::Dedent => "o fim do bloco recuado".to_string(),
            TokenKind::Newline => "o fim da linha".to_string(),
            TokenKind::Eof => "o fim do texto".to_string(),
        }
    }
}

pub fn tokenize(src: &str) -> Result<Vec<Token>> {
    let raw = scan(src)?;
    let protected = fuse(raw, crate::lexicon::PROTECTED_LOCUTIONS, false);
    let split = split_contractions(protected);
    Ok(fuse(split, crate::lexicon::LOCUTIONS, true))
}

fn lex_error(msg: impl Into<String>, start: usize, end: usize) -> Box<Diagnostic> {
    Box::new(Diagnostic::new(Category::Lexical, msg, Span::new(start, end.max(start + 1))))
}

fn is_word_char(c: char) -> bool {
    c.is_alphabetic() || c.is_ascii_digit() || c == '_'
}

fn scan(src: &str) -> Result<Vec<Token>> {
    let mut out: Vec<Token> = Vec::new();
    let mut indents: Vec<usize> = vec![0];
    // Verdadeiro quando a próxima linha física começa uma linha lógica nova.
    let mut at_logical_start = true;
    let mut offset = 0usize;
    for raw_line in src.split_inclusive('\n') {
        let line_start = offset;
        offset += raw_line.len();
        let line = raw_line.trim_end_matches(['\n', '\r']);
        let content = line.trim_start_matches([' ', '\t']);
        if content.is_empty() {
            continue;
        }
        let lower = content.to_lowercase();
        if lower.starts_with("nota:") || content.starts_with("--") {
            continue;
        }
        let indent_bytes = line.len() - content.len();
        if at_logical_start {
            let width: usize = line[..indent_bytes].chars().map(|c| if c == '\t' { 4 } else { 1 }).sum();
            let top = *indents.last().unwrap();
            let pos = line_start + indent_bytes;
            if width > top {
                indents.push(width);
                out.push(Token {
                    kind: TokenKind::Indent,
                    span: Span::new(pos, pos),
                    orig: String::new(),
                    contraction: None,
                    line_start: false,
                });
            } else if width < top {
                while width < *indents.last().unwrap() {
                    indents.pop();
                    out.push(Token {
                        kind: TokenKind::Dedent,
                        span: Span::new(pos, pos),
                        orig: String::new(),
                        contraction: None,
                        line_start: false,
                    });
                }
                if width != *indents.last().unwrap() {
                    return err(Diagnostic::new(
                        Category::Lexical,
                        "Recuo inconsistente: esta linha não se alinha com nenhum bloco aberto.",
                        Span::new(line_start, pos.max(line_start + 1)),
                    )
                    .hint("Alinhe a linha com o começo de um dos blocos de cima."));
                }
            }
        }
        let before = out.len();
        scan_line(line, line_start, &mut out)?;
        if let Some(t) = out.get_mut(before) {
            t.line_start = true;
        }
        let ends_period = out.len() > before && matches!(out.last().map(|t| &t.kind), Some(TokenKind::Punct('.' | ':' | ';')));
        if ends_period {
            let end = out.last().unwrap().span.end as usize;
            out.push(Token {
                kind: TokenKind::Newline,
                span: Span::new(end, end),
                orig: String::new(),
                contraction: None,
                line_start: false,
            });
        }
        at_logical_start = ends_period || (out.len() == before && at_logical_start);
    }
    let end = src.len();
    if !matches!(out.last().map(|t| &t.kind), None | Some(TokenKind::Newline)) {
        out.push(Token { kind: TokenKind::Newline, span: Span::new(end, end), orig: String::new(), contraction: None, line_start: false });
    }
    while indents.len() > 1 {
        indents.pop();
        out.push(Token { kind: TokenKind::Dedent, span: Span::new(end, end), orig: String::new(), contraction: None, line_start: false });
    }
    out.push(Token { kind: TokenKind::Eof, span: Span::new(end, end), orig: String::new(), contraction: None, line_start: false });
    Ok(out)
}

fn scan_line(line: &str, base: usize, out: &mut Vec<Token>) -> Result<()> {
    let chars: Vec<(usize, char)> = line.char_indices().collect();
    let n = chars.len();
    let pos = |i: usize| -> usize { if i < n { base + chars[i].0 } else { base + line.len() } };
    let mut i = 0;
    while i < n {
        let c = chars[i].1;
        if c == ' ' || c == '\t' {
            i += 1;
            continue;
        }
        if c == '-' && i + 1 < n && chars[i + 1].1 == '-' {
            break;
        }
        let start = i;
        if c.is_ascii_digit() || (c == '-' && i + 1 < n && chars[i + 1].1.is_ascii_digit()) {
            let (tok, next) = scan_number(&chars, i, base, line)?;
            out.push(tok);
            i = next;
            continue;
        }
        if c.is_alphabetic() || c == '_' {
            let mut j = i;
            while j < n {
                let cj = chars[j].1;
                // Hífen interno junta palavras compostas ("segunda-feira").
                let hyphen = cj == '-' && j + 1 < n && chars[j + 1].1.is_alphabetic() && j > i;
                if is_word_char(cj) || hyphen {
                    j += 1;
                } else {
                    break;
                }
            }
            let orig = &line[chars[i].0..if j < n { chars[j].0 } else { line.len() }];
            out.push(Token {
                kind: TokenKind::Word(orig.to_lowercase()),
                span: Span::new(pos(i), pos(j)),
                orig: orig.to_string(),
                contraction: None,
                line_start: false,
            });
            i = j;
            continue;
        }
        if c == '"' || c == '“' || c == '”' {
            let mut j = i + 1;
            let mut s = String::new();
            loop {
                if j >= n {
                    return Err(lex_error("Texto sem aspas de fechamento.", pos(start), pos(n)));
                }
                let cj = chars[j].1;
                if cj == '"' || cj == '”' || cj == '“' {
                    break;
                }
                if cj == '\\' {
                    let (e, next) = escape(&chars, j, base)?;
                    s.push(e);
                    j = next;
                    continue;
                }
                s.push(cj);
                j += 1;
            }
            out.push(Token {
                kind: TokenKind::Text(s),
                span: Span::new(pos(start), pos(j + 1)),
                orig: line[chars[start].0..if j + 1 < n { chars[j + 1].0 } else { line.len() }].to_string(),
                contraction: None,
                line_start: false,
            });
            i = j + 1;
            continue;
        }
        if c == '\'' || c == '‘' || c == '’' {
            let mut j = i + 1;
            if j >= n {
                return Err(lex_error("Caractere sem aspas simples de fechamento.", pos(start), pos(n)));
            }
            let value = if chars[j].1 == '\\' {
                let (e, next) = escape(&chars, j, base)?;
                j = next;
                e
            } else {
                j += 1;
                chars[j - 1].1
            };
            if j >= n || !matches!(chars[j].1, '\'' | '’' | '‘') {
                return Err(Box::new(
                    Diagnostic::new(
                        Category::Lexical,
                        "Caractere entre aspas simples tem uma letra só.",
                        Span::new(pos(start), pos(j.min(n))),
                    )
                    .hint("Para mais de uma letra, use aspas duplas: é um texto."),
                ));
            }
            out.push(Token {
                kind: TokenKind::Char(value),
                span: Span::new(pos(start), pos(j + 1)),
                orig: line[chars[start].0..if j + 1 < n { chars[j + 1].0 } else { line.len() }].to_string(),
                contraction: None,
                line_start: false,
            });
            i = j + 1;
            continue;
        }
        if matches!(c, '.' | ',' | ':' | ';' | '(' | ')') {
            out.push(Token {
                kind: TokenKind::Punct(c),
                span: Span::new(pos(i), pos(i + 1)),
                orig: c.to_string(),
                contraction: None,
                line_start: false,
            });
            i += 1;
            continue;
        }
        let d = Diagnostic::new(Category::Lexical, format!("O sinal \"{c}\" não faz parte da Lavra."), Span::new(pos(i), pos(i + 1)));
        let d = match c {
            '+' | '-' | '*' | '/' | '%' => {
                d.hint("Operações se escrevem por extenso: \"mais\", \"menos\", \"vezes\", \"dividido por\", \"módulo\".")
            }
            '=' => d.hint("Para dar valor, use \"é\" ou \"começa em\"; para mudar, \"vira\"; para comparar, \"for igual a\"."),
            '<' | '>' => d.hint("Comparações se escrevem por extenso: \"for maior que\", \"for menor que\"."),
            '?' | '!' => d.hint("Pergunta e exclamação só dentro de texto entre aspas."),
            _ => d,
        };
        return err(d);
    }
    Ok(())
}

fn escape(chars: &[(usize, char)], j: usize, base: usize) -> Result<(char, usize)> {
    let at = |k: usize| base + chars.get(k).map(|c| c.0).unwrap_or(0);
    let Some(&(_, e)) = chars.get(j + 1) else {
        return Err(lex_error("Barra invertida no fim da linha.", at(j), at(j) + 1));
    };
    let v = match e {
        'n' => '\n',
        't' => '\t',
        '0' => '\0',
        '\\' => '\\',
        '"' => '"',
        '\'' => '\'',
        _ => {
            return Err(Box::new(
                Diagnostic::new(Category::Lexical, format!("Escape desconhecido: \"\\{e}\"."), Span::new(at(j), at(j + 1) + e.len_utf8()))
                    .hint("Escapes válidos: \\n (quebra de linha), \\t (tabulação), \\\" (aspas), \\\\ (barra), \\0."),
            ));
        }
    };
    Ok((v, j + 2))
}

fn scan_number(chars: &[(usize, char)], start: usize, base: usize, line: &str) -> Result<(Token, usize)> {
    let n = chars.len();
    let pos = |i: usize| -> usize { if i < n { base + chars[i].0 } else { base + line.len() } };
    let text_of = |a: usize, b: usize| line[chars[a].0..if b < n { chars[b].0 } else { line.len() }].to_string();
    let mut i = start;
    let negative = chars[i].1 == '-';
    if negative {
        i += 1;
    }
    if chars[i].1 == '0' && i + 1 < n && matches!(chars[i + 1].1, 'x' | 'X') {
        let mut j = i + 2;
        let mut v: i128 = 0;
        while j < n && chars[j].1.is_ascii_hexdigit() {
            v = v
                .checked_mul(16)
                .and_then(|v| v.checked_add(chars[j].1.to_digit(16).unwrap() as i128))
                .filter(|v| *v <= u64::MAX as i128)
                .ok_or_else(|| lex_error("Número grande demais.", pos(start), pos(j)))?;
            j += 1;
        }
        if j == i + 2 {
            return Err(lex_error("Hexadecimal sem dígitos depois de \"0x\".", pos(start), pos(j)));
        }
        if negative {
            return Err(lex_error("Hexadecimal não leva sinal.", pos(start), pos(j)));
        }
        return Ok((
            Token {
                kind: TokenKind::Int(v),
                span: Span::new(pos(start), pos(j)),
                orig: text_of(start, j),
                contraction: None,
                line_start: false,
            },
            j,
        ));
    }
    let mut digits = String::new();
    while i < n && chars[i].1.is_ascii_digit() {
        digits.push(chars[i].1);
        i += 1;
    }
    // Milhar com espaço: o número continua se vier espaço e exatamente três dígitos.
    while i + 3 < n
        && chars[i].1 == ' '
        && (1..=3).all(|k| chars[i + k].1.is_ascii_digit())
        && (i + 4 >= n || !chars[i + 4].1.is_ascii_digit())
    {
        for k in 1..=3 {
            digits.push(chars[i + k].1);
        }
        i += 4;
    }
    let mut frac = String::new();
    if i + 1 < n && chars[i].1 == ',' && chars[i + 1].1.is_ascii_digit() {
        i += 1;
        while i < n && chars[i].1.is_ascii_digit() {
            frac.push(chars[i].1);
            i += 1;
        }
    }
    if i < n && is_word_char(chars[i].1) {
        let mut j = i;
        while j < n && is_word_char(chars[j].1) {
            j += 1;
        }
        return Err(Box::new(
            Diagnostic::new(Category::Lexical, "Número colado numa palavra.", Span::new(pos(start), pos(j)))
                .hint("Separe o número da palavra com um espaço."),
        ));
    }
    let span = Span::new(pos(start), pos(i));
    let sign = if negative { "-" } else { "" };
    let kind = if !frac.is_empty() {
        TokenKind::Real(format!("{sign}{digits}.{frac}").parse().map_err(|_| lex_error("Número real malformado.", pos(start), pos(i)))?)
    } else {
        let v: i128 = format!("{sign}{digits}")
            .parse()
            .ok()
            .filter(|v: &i128| *v <= u64::MAX as i128 && *v >= i64::MIN as i128)
            .ok_or_else(|| lex_error("Número grande demais: nenhum tipo da Lavra guarda esse valor.", pos(start), pos(i)))?;
        TokenKind::Int(v)
    };
    Ok((Token { kind, span, orig: text_of(start, i), contraction: None, line_start: false }, i))
}

fn fuse(tokens: Vec<Token>, table: &[(&str, &str)], allow_contracted_second: bool) -> Vec<Token> {
    let mut out: Vec<Token> = Vec::with_capacity(tokens.len());
    let mut i = 0;
    while i < tokens.len() {
        let t = &tokens[i];
        if let (Some(w1), Some(t2)) = (t.word(), tokens.get(i + 1))
            && t.contraction.is_none_or(|c| c.part == 2)
            && let Some(w2) = t2.word()
            && let Some((a, b)) = table.iter().find(|(a, b)| *a == w1 && *b == w2)
        {
            let second_contracted = t2.contraction.is_some();
            if second_contracted && !allow_contracted_second {
                out.push(t.clone());
                i += 1;
                continue;
            }
            let span = if second_contracted { t.span.to(Span { start: t2.span.start, end: t2.span.start }) } else { t.span.to(t2.span) };
            let orig = if second_contracted { t.orig.clone() } else { format!("{} {}", t.orig, t2.orig) };
            out.push(Token { kind: TokenKind::Word(format!("{a} {b}")), span, orig, contraction: None, line_start: t.line_start });
            i += 2;
            continue;
        }
        out.push(t.clone());
        i += 1;
    }
    out
}

fn split_contractions(tokens: Vec<Token>) -> Vec<Token> {
    let mut out = Vec::with_capacity(tokens.len() + tokens.len() / 4);
    for t in tokens {
        if let TokenKind::Word(w) = &t.kind
            && let Some((written, prep, art)) = split_contraction(w)
        {
            out.push(Token {
                kind: TokenKind::Word(prep.to_string()),
                span: t.span,
                orig: t.orig.clone(),
                contraction: Some(Contraction { written, part: 1 }),
                line_start: t.line_start,
            });
            out.push(Token {
                kind: TokenKind::Word(art.to_string()),
                span: t.span,
                orig: t.orig.clone(),
                contraction: Some(Contraction { written, part: 2 }),
                line_start: false,
            });
            continue;
        }
        out.push(t);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words(src: &str) -> Vec<String> {
        tokenize(src)
            .unwrap()
            .into_iter()
            .map(|t| match t.kind {
                TokenKind::Word(w) => w,
                TokenKind::Int(v) => format!("#{v}"),
                TokenKind::Real(v) => format!("#{v}"),
                TokenKind::Text(s) => format!("{s:?}"),
                TokenKind::Char(c) => format!("'{c}'"),
                TokenKind::Punct(c) => c.to_string(),
                TokenKind::Indent => "INDENT".into(),
                TokenKind::Dedent => "DEDENT".into(),
                TokenKind::Newline => "NL".into(),
                TokenKind::Eof => "EOF".into(),
            })
            .collect()
    }

    #[test]
    fn contractions_and_locutions() {
        assert_eq!(
            words("O saldo da conta for igual ao teto."),
            ["o", "saldo", "de", "a", "conta", "for", "igual a", "o", "teto", ".", "NL", "EOF"]
        );
        assert_eq!(words("pelo menos 3."), ["pelo menos", "#3", ".", "NL", "EOF"]);
    }

    #[test]
    fn numbers() {
        assert_eq!(words("1 000 000 e 8,5 e 0xFF."), ["#1000000", "e", "#8.5", "e", "#255", ".", "NL", "EOF"]);
        assert_eq!(words("7, 8."), ["#7", ",", "#8", ".", "NL", "EOF"]);
    }

    #[test]
    fn blocks_and_enjambement() {
        let src = "Se x:\n    Escrevo 1\n        mais 2.\nEscrevo 3.\n";
        assert_eq!(
            words(src),
            ["se", "x", ":", "NL", "INDENT", "escrevo", "#1", "mais", "#2", ".", "NL", "DEDENT", "escrevo", "#3", ".", "NL", "EOF"]
        );
    }

    #[test]
    fn comments() {
        assert_eq!(words("Nota: nada aqui.\nEscrevo 1. -- fim\n"), ["escrevo", "#1", ".", "NL", "EOF"]);
        assert_eq!(words("Escrevo \"a -- b\"."), ["escrevo", "\"a -- b\"", ".", "NL", "EOF"]);
    }

    #[test]
    fn rejects_symbols() {
        assert!(tokenize("Escrevo 1 + 2.").is_err());
    }
}
