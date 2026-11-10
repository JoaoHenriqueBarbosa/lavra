//! Léxico: quebra o texto em palavras, números, textos e pontuação; calcula os recuos
//! (INDENT/DEDENT); desfaz as contrações e junta as locuções.
//!
//! Comentário é a linha que começa com "Nota:" ou "--" (e
//! "--" no meio da linha corta o resto); a linha só termina o período se acabar em
//! ".", ":" ou ";", senão continua na de baixo com recuo livre (enjambement).

use crate::codes::*;
use crate::diagnostics::{Diagnostic, Diagnostics};
use crate::morphology::{Gender, split_contraction};
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
    /// Trecho que o léxico recusou (sinal inválido, número malformado, texto sem
    /// fechamento). O erro já foi relatado; a oração que o contém falha em silêncio.
    Error,
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
            TokenKind::Error => format!("\"{}\"", self.orig),
            TokenKind::Indent => "um recuo".to_string(),
            TokenKind::Dedent => "o fim do bloco recuado".to_string(),
            TokenKind::Newline => "o fim da linha".to_string(),
            TokenKind::Eof => "o fim do texto".to_string(),
        }
    }
}

/// Os tokens do texto. Um erro léxico vira diagnóstico e um token `Error` no lugar do
/// trecho recusado; a varredura segue.
pub fn tokenize(src: &str, diags: &mut Diagnostics) -> Vec<Token> {
    tokenize_at(src, 0, diags)
}

/// O papel de cada linha física do texto, para o formatador.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LineRole {
    Blank,
    /// Comentário de linha inteira ("Nota:" ou "--").
    Comment,
    /// Começo de linha lógica, na profundidade dada (quantos blocos abertos).
    Start(usize),
    /// Continuação de uma linha lógica que não terminou na linha de cima (enjambement).
    Continuation,
}

/// O papel de cada linha física (na ordem de `src.split_inclusive('\n')`).
pub fn layout(src: &str) -> Vec<LineRole> {
    let mut roles = Vec::new();
    scan_with(src, 0, &mut Diagnostics::default(), Some(&mut roles));
    roles
}

/// Como `tokenize`, com as posições a partir de `base` (a base do arquivo no
/// `SourceMap`, quando a compilação tiver vários arquivos).
pub fn tokenize_at(src: &str, base: usize, diags: &mut Diagnostics) -> Vec<Token> {
    let raw = scan(src, base, diags);
    let protected = fuse(raw, crate::lexicon::PROTECTED_LOCUTIONS, false);
    let split = split_contractions(protected);
    let fused = fuse(split, crate::lexicon::LOCUTIONS, true);
    numerals(fused, src, base, diags)
}

/// O numeral por extenso na posição `i`, se a palavra é um (sem contração).
fn numeral_word(tokens: &[Token], i: usize) -> Option<(u32, Option<Gender>)> {
    let t = tokens.get(i)?;
    if t.contraction.is_some() {
        return None;
    }
    crate::lexicon::numeral(t.word()?)
}

/// A palavra `w` na posição `i`, na mesma linha do numeral que ela continua.
fn same_line_word(tokens: &[Token], i: usize, w: &str) -> bool {
    tokens.get(i).is_some_and(|t| t.is_word(w) && t.contraction.is_none() && !t.line_start)
}

/// "um" ou "uma" depois do "e" de um composto ("cento e um"): só aí o artigo é numeral.
fn numeral_one(tokens: &[Token], i: usize) -> Option<(u32, Vec<Gender>, usize)> {
    let t = tokens.get(i)?;
    if t.contraction.is_some() {
        return None;
    }
    match t.word()? {
        "um" => Some((1, vec![Gender::Masculine], i + 1)),
        "uma" => Some((1, vec![Gender::Feminine], i + 1)),
        _ => None,
    }
}

/// De 0 a 99: "sete", "quinze", "vinte e cinco", "trinta e uma". Devolve o valor, os
/// gêneros das palavras que flexionam e a posição depois do numeral.
fn numeral_below_100(tokens: &[Token], i: usize) -> Option<(u32, Vec<Gender>, usize)> {
    let (v, g) = numeral_word(tokens, i)?;
    if v >= 100 {
        return None;
    }
    let mut genders: Vec<Gender> = g.into_iter().collect();
    if v >= 20 && same_line_word(tokens, i + 1, "e") && tokens.get(i + 2).is_some_and(|t| !t.line_start) {
        // A unidade: de "dois" a "nove", ou "um"/"uma", que só aqui é numeral.
        let unit = match numeral_word(tokens, i + 2) {
            Some((u, ug)) if (1..=9).contains(&u) => Some((u, ug.into_iter().collect::<Vec<_>>())),
            _ => numeral_one(tokens, i + 2).map(|(u, gs, _)| (u, gs)),
        };
        if let Some((u, ug)) = unit {
            genders.extend(ug);
            return Some((v + u, genders, i + 3));
        }
    }
    Some((v, genders, i + 1))
}

/// De 0 a 999: o que cabe antes e depois de "mil".
fn numeral_below_1000(tokens: &[Token], i: usize) -> Option<(u32, Vec<Gender>, usize)> {
    let (v, g) = numeral_word(tokens, i)?;
    if v == 1000 {
        return None;
    }
    if v < 100 {
        return numeral_below_100(tokens, i);
    }
    let mut genders: Vec<Gender> = g.into_iter().collect();
    // "cem" é redondo; "cento" e as outras centenas continuam com "e".
    if !tokens[i].is_word("cem")
        && same_line_word(tokens, i + 1, "e")
        && tokens.get(i + 2).is_some_and(|t| !t.line_start)
        && let Some((rest, gs, end)) = numeral_below_100(tokens, i + 2).or_else(|| numeral_one(tokens, i + 2))
    {
        genders.extend(gs);
        return Some((v + rest, genders, end));
    }
    Some((v, genders, i + 1))
}

/// O numeral inteiro a partir de `i`, até 999 999: "mil", "dois mil e trezentos",
/// "mil duzentos e trinta".
fn numeral_at(tokens: &[Token], i: usize) -> Option<(u32, Vec<Gender>, usize)> {
    let (thousands, mut genders, after) = if tokens[i].is_word("mil") && tokens[i].contraction.is_none() {
        (1, Vec::new(), i + 1)
    } else {
        let (v, gs, end) = numeral_below_1000(tokens, i)?;
        if !(v >= 2 && same_line_word(tokens, end, "mil")) {
            return Some((v, gs, end));
        }
        (v, gs, end + 1)
    };
    let total = thousands * 1000;
    // Depois de "mil": com "e", qualquer resto ("mil e dois"); sem "e", só centena
    // ("mil duzentos e trinta").
    let with_e = same_line_word(tokens, after, "e");
    let at = if with_e { after + 1 } else { after };
    if tokens.get(at).is_some_and(|t| !t.line_start)
        && let Some((rest, gs, end)) = numeral_below_1000(tokens, at).or_else(|| numeral_one(tokens, at).filter(|_| with_e))
        && (with_e || rest >= 100)
    {
        genders.extend(gs);
        return Some((total + rest, genders, end));
    }
    Some((total, genders, after))
}

/// Troca o gênero das palavras de um numeral: "dois" e "duas", "um" e "uma", "duzentos" e
/// "duzentas".
fn regender_numeral(orig: &str, to: Gender) -> String {
    orig.split(' ')
        .map(|w| {
            let lower = w.to_lowercase();
            let swapped = match (lower.as_str(), to) {
                ("dois", Gender::Feminine) => "duas".to_string(),
                ("duas", Gender::Masculine) => "dois".to_string(),
                ("um", Gender::Feminine) => "uma".to_string(),
                ("uma", Gender::Masculine) => "um".to_string(),
                (l, Gender::Feminine) if l.ends_with("entos") => format!("{}entas", &l[..l.len() - 5]),
                (l, Gender::Masculine) if l.ends_with("entas") => format!("{}entos", &l[..l.len() - 5]),
                _ => return w.to_string(),
            };
            if w.chars().next().is_some_and(char::is_uppercase) {
                let mut c = swapped.chars();
                c.next().map(|f| f.to_uppercase().collect::<String>() + c.as_str()).unwrap_or_default()
            } else {
                swapped
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Junta cada numeral por extenso num número só ("vinte e cinco" vale 25) e confere o
/// gênero dos que flexionam: feminino antes de "vezes" ("duas vezes"), masculino no resto.
fn numerals(tokens: Vec<Token>, src: &str, base: usize, diags: &mut Diagnostics) -> Vec<Token> {
    let mut out: Vec<Token> = Vec::with_capacity(tokens.len());
    let mut i = 0;
    while i < tokens.len() {
        let Some((value, genders, end)) = numeral_at(&tokens, i) else {
            out.push(tokens[i].clone());
            i += 1;
            continue;
        };
        let span = tokens[i].span.to(tokens[end - 1].span);
        let orig = src.get(span.start as usize - base..span.end as usize - base).unwrap_or(&tokens[i].orig).to_string();
        let want = if tokens.get(end).is_some_and(|t| t.is_word("vezes")) { Gender::Feminine } else { Gender::Masculine };
        if genders.iter().any(|g| *g != want) && !orig.contains('\n') {
            let right = regender_numeral(&orig, want);
            let why = if want == Gender::Feminine {
                "\"vezes\" é feminino, e o numeral concorda com ela"
            } else {
                "sem um nome feminino depois, o numeral fica no masculino"
            };
            diags.push(Diagnostic::new(L0013, format!("O numeral não concorda: \"{right}\", não \"{orig}\"."), span).primary(why).fix(
                format!("Escrever \"{right}\""),
                span,
                right.clone(),
            ));
        }
        out.push(Token { kind: TokenKind::Int(value as i128), span, orig, contraction: None, line_start: tokens[i].line_start });
        i = end;
    }
    out
}

fn lex_error(code: Code, msg: impl Into<String>, start: usize, end: usize) -> Diagnostic {
    Diagnostic::new(code, msg, Span::new(start, end.max(start + 1)))
}

fn error_token(line: &str, base: usize, start: usize, end: usize) -> Token {
    let end = end.max(start);
    Token {
        kind: TokenKind::Error,
        span: Span::new(start, end),
        orig: line[start - base..end - base].to_string(),
        contraction: None,
        line_start: false,
    }
}

fn is_word_char(c: char) -> bool {
    c.is_alphabetic() || c.is_ascii_digit() || c == '_'
}

fn scan(src: &str, base: usize, diags: &mut Diagnostics) -> Vec<Token> {
    scan_with(src, base, diags, None)
}

fn scan_with(src: &str, base: usize, diags: &mut Diagnostics, mut roles: Option<&mut Vec<LineRole>>) -> Vec<Token> {
    let mut role = |r: LineRole| {
        if let Some(v) = roles.as_mut() {
            v.push(r);
        }
    };
    let mut out: Vec<Token> = Vec::new();
    let mut indents: Vec<usize> = vec![0];
    // Verdadeiro quando a próxima linha física começa uma linha lógica nova.
    let mut at_logical_start = true;
    let mut offset = base;
    for raw_line in src.split_inclusive('\n') {
        let line_start = offset;
        offset += raw_line.len();
        let line = raw_line.trim_end_matches(['\n', '\r']);
        let content = line.trim_start_matches([' ', '\t']);
        if content.is_empty() {
            role(LineRole::Blank);
            continue;
        }
        let lower = content.to_lowercase();
        if lower.starts_with("nota:") || content.starts_with("--") {
            role(LineRole::Comment);
            continue;
        }
        if !at_logical_start {
            role(LineRole::Continuation);
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
                    // Como no Python: a linha fica no bloco aberto mais próximo abaixo dela.
                    diags.push(
                        Diagnostic::new(
                            L0012,
                            "Recuo inconsistente: esta linha não se alinha com nenhum bloco aberto.",
                            Span::new(line_start, pos.max(line_start + 1)),
                        )
                        .hint("Alinhe a linha com o começo de um dos blocos de cima."),
                    );
                }
            }
        }
        if at_logical_start {
            role(LineRole::Start(indents.len() - 1));
        }
        let before = out.len();
        let cut = scan_line(line, line_start, &mut out, diags);
        if let Some(t) = out.get_mut(before) {
            t.line_start = true;
        }
        // Texto sem fechamento engoliu o fim da linha, ponto final inclusive: a linha
        // termina o período, para o erro não emendar na de baixo.
        let ends_period = out.len() > before && (cut || matches!(out.last().map(|t| &t.kind), Some(TokenKind::Punct('.' | ':' | ';'))));
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
    let end = base + src.len();
    if !matches!(out.last().map(|t| &t.kind), None | Some(TokenKind::Newline)) {
        out.push(Token { kind: TokenKind::Newline, span: Span::new(end, end), orig: String::new(), contraction: None, line_start: false });
    }
    while indents.len() > 1 {
        indents.pop();
        out.push(Token { kind: TokenKind::Dedent, span: Span::new(end, end), orig: String::new(), contraction: None, line_start: false });
    }
    out.push(Token { kind: TokenKind::Eof, span: Span::new(end, end), orig: String::new(), contraction: None, line_start: false });
    out
}

/// Os tokens de uma linha física. Devolve verdadeiro se um texto ou caractere ficou sem
/// fechamento e engoliu o fim da linha.
fn scan_line(line: &str, base: usize, out: &mut Vec<Token>, diags: &mut Diagnostics) -> bool {
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
            let (tok, next) = scan_number(&chars, i, base, line, diags);
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
                    // A barra no fim da linha já foi acusada (L0005), e é ela que deixou o texto aberto.
                    if !(n > start + 1 && chars[n - 1].1 == '\\') {
                        diags.push(lex_error(L0002, "Texto sem aspas de fechamento.", pos(start), pos(n)));
                    }
                    out.push(error_token(line, base, pos(start), pos(n)));
                    return true;
                }
                let cj = chars[j].1;
                if cj == '"' || cj == '”' || cj == '“' {
                    break;
                }
                if cj == '\\' {
                    let (e, next) = escape(&chars, j, base, diags);
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
                diags.push(lex_error(L0003, "Caractere sem aspas simples de fechamento.", pos(start), pos(n)));
                out.push(error_token(line, base, pos(start), pos(n)));
                return true;
            }
            let value = if chars[j].1 == '\\' {
                let (e, next) = escape(&chars, j, base, diags);
                j = next;
                e
            } else {
                j += 1;
                chars[j - 1].1
            };
            if j >= n && chars[n - 1].1 == '\\' {
                // A barra no fim da linha já foi acusada (L0005).
                out.push(error_token(line, base, pos(start), pos(n)));
                return true;
            }
            if j >= n || !matches!(chars[j].1, '\'' | '’' | '‘') {
                diags.push(
                    Diagnostic::new(L0004, "Caractere entre aspas simples tem uma letra só.", Span::new(pos(start), pos(j.min(n))))
                        .hint("Para mais de uma letra, use aspas duplas: é um texto."),
                );
                // Pula até a aspa de fechamento, se houver, ou até o fim da linha.
                let close = (j..n).find(|&k| matches!(chars[k].1, '\'' | '’' | '‘'));
                let end = close.map_or(n, |k| k + 1);
                out.push(error_token(line, base, pos(start), pos(end)));
                if close.is_none() {
                    return true;
                }
                i = end;
                continue;
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
        let d = Diagnostic::new(L0001, format!("O sinal \"{c}\" não faz parte da Lavra."), Span::new(pos(i), pos(i + 1)));
        let d = match c {
            '+' | '-' | '*' | '/' | '%' => {
                d.hint("Operações se escrevem por extenso: \"mais\", \"menos\", \"vezes\", \"dividido por\", \"módulo\".")
            }
            '=' => d.hint("Para dar valor, use \"é\" ou \"começa em\"; para mudar, \"vira\"; para comparar, \"for igual a\"."),
            '<' | '>' => d.hint("Comparações se escrevem por extenso: \"for maior que\", \"for menor que\"."),
            '?' | '!' => d.hint("Pergunta e exclamação só dentro de texto entre aspas."),
            _ => d,
        };
        diags.push(d);
        out.push(error_token(line, base, pos(i), pos(i + 1)));
        i += 1;
    }
    false
}

/// Um escape dentro de texto ou caractere. Com erro, relata e devolve a barra.
fn escape(chars: &[(usize, char)], j: usize, base: usize, diags: &mut Diagnostics) -> (char, usize) {
    let at = |k: usize| base + chars.get(k).map(|c| c.0).unwrap_or(0);
    let Some(&(_, e)) = chars.get(j + 1) else {
        diags.push(lex_error(L0005, "Barra invertida no fim da linha.", at(j), at(j) + 1));
        return ('\\', j + 1);
    };
    let v = match e {
        'n' => '\n',
        't' => '\t',
        '0' => '\0',
        '\\' => '\\',
        '"' => '"',
        '\'' => '\'',
        _ => {
            diags.push(
                Diagnostic::new(L0006, format!("Escape desconhecido: \"\\{e}\"."), Span::new(at(j), at(j + 1) + e.len_utf8()))
                    .hint("Escapes válidos: \\n (quebra de linha), \\t (tabulação), \\\" (aspas), \\\\ (barra), \\0."),
            );
            e
        }
    };
    (v, j + 2)
}

/// Um número: inteiro, real com vírgula, milhar com espaço ou hexadecimal. Malformado,
/// relata e devolve um token de erro.
fn scan_number(chars: &[(usize, char)], start: usize, base: usize, line: &str, diags: &mut Diagnostics) -> (Token, usize) {
    let n = chars.len();
    let pos = |i: usize| -> usize { if i < n { base + chars[i].0 } else { base + line.len() } };
    let text_of = |a: usize, b: usize| line[chars[a].0..if b < n { chars[b].0 } else { line.len() }].to_string();
    let mut fail = |d: Diagnostic, end: usize| -> (Token, usize) {
        diags.push(d);
        (error_token(line, base, pos(start), pos(end)), end)
    };
    let mut i = start;
    let negative = chars[i].1 == '-';
    if negative {
        i += 1;
    }
    if chars[i].1 == '0' && i + 1 < n && matches!(chars[i + 1].1, 'x' | 'X') {
        let mut j = i + 2;
        let mut v: Option<i128> = Some(0);
        while j < n && chars[j].1.is_ascii_hexdigit() {
            v = v
                .and_then(|v| v.checked_mul(16))
                .and_then(|v| v.checked_add(chars[j].1.to_digit(16).unwrap() as i128))
                .filter(|v| *v <= u64::MAX as i128);
            j += 1;
        }
        if j == i + 2 {
            return fail(lex_error(L0008, "Hexadecimal sem dígitos depois de \"0x\".", pos(start), pos(j)), j);
        }
        if negative {
            return fail(lex_error(L0009, "Hexadecimal não leva sinal.", pos(start), pos(j)), j);
        }
        let Some(v) = v else {
            return fail(lex_error(L0007, "Número grande demais.", pos(start), pos(j)), j);
        };
        let tok = Token {
            kind: TokenKind::Int(v),
            span: Span::new(pos(start), pos(j)),
            orig: text_of(start, j),
            contraction: None,
            line_start: false,
        };
        return (tok, j);
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
        return fail(
            Diagnostic::new(L0011, "Número colado numa palavra.", Span::new(pos(start), pos(j)))
                .hint("Separe o número da palavra com um espaço."),
            j,
        );
    }
    let span = Span::new(pos(start), pos(i));
    let sign = if negative { "-" } else { "" };
    let kind = if !frac.is_empty() {
        match format!("{sign}{digits}.{frac}").parse::<f64>() {
            Ok(v) if !v.is_finite() => {
                return fail(lex_error(L0007, "Número grande demais: nenhum tipo da Lavra guarda esse valor.", pos(start), pos(i)), i);
            }
            Ok(v) => TokenKind::Real(v),
            Err(_) => return fail(lex_error(L0010, "Número real malformado.", pos(start), pos(i)), i),
        }
    } else {
        match format!("{sign}{digits}").parse().ok().filter(|v: &i128| *v <= u64::MAX as i128 && *v >= i64::MIN as i128) {
            Some(v) => TokenKind::Int(v),
            None => return fail(lex_error(L0007, "Número grande demais: nenhum tipo da Lavra guarda esse valor.", pos(start), pos(i)), i),
        }
    };
    (Token { kind, span, orig: text_of(start, i), contraction: None, line_start: false }, i)
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
        let mut diags = Diagnostics::default();
        let toks = tokenize(src, &mut diags);
        assert!(diags.list.is_empty(), "erro léxico inesperado: {:?}", diags.list);
        toks.into_iter()
            .map(|t| match t.kind {
                TokenKind::Word(w) => w,
                TokenKind::Int(v) => format!("#{v}"),
                TokenKind::Real(v) => format!("#{v}"),
                TokenKind::Text(s) => format!("{s:?}"),
                TokenKind::Char(c) => format!("'{c}'"),
                TokenKind::Punct(c) => c.to_string(),
                TokenKind::Error => "ERRO".into(),
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
    fn rejects_symbols_and_goes_on() {
        let mut diags = Diagnostics::default();
        let toks = tokenize("Escrevo 1 + 2.\nEscrevo \"sem fim\nEscrevo 3.\n", &mut diags);
        assert_eq!(diags.list.iter().filter_map(|d| d.code).collect::<Vec<_>>(), ["L0001", "L0002"]);
        // O sinal vira um token de erro e o resto da linha segue; o texto sem fechamento
        // encerra a linha, e a de baixo é lida normalmente.
        let kinds: Vec<String> = toks.iter().map(|t| format!("{:?}", t.kind)).collect();
        assert!(kinds.contains(&"Error".to_string()));
        assert_eq!(toks.iter().filter(|t| t.is_word("escrevo")).count(), 3);
    }
}
