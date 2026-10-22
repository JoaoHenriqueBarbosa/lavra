//! Tokens semânticos: o que só o compilador sabe pintar. Verbos do programa (também nos
//! predicados e depois de "o resultado de"), parâmetros, variáveis, campos, tipos e casos,
//! com os modificadores de posse (e o `moved` no nome que entrega o valor). As palavras da
//! língua ficam com a gramática TextMate, que já as pinta pela forma.

use super::convert::Text;
use crate::Analysis;
use crate::ast::ParamMode;
use crate::lexer::TokenKind;
use crate::span::Span;
use crate::tables::{Def, DefKind, Use};
use lsp_types as lsp;
use std::collections::BTreeMap;

pub const TYPES: &[&str] = &["function", "parameter", "variable", "property", "type", "enumMember"];
pub const MODIFIERS: &[&str] = &["declaration", "readonly", "modification", "borrowed", "lent", "moved"];

const FUNCTION: u32 = 0;
const PARAMETER: u32 = 1;
const VARIABLE: u32 = 2;
const PROPERTY: u32 = 3;
const TYPE: u32 = 4;
const ENUM_MEMBER: u32 = 5;

const DECLARATION: u32 = 1 << 0;
const READONLY: u32 = 1 << 1;
const MODIFICATION: u32 = 1 << 2;
const BORROWED: u32 = 1 << 3;
const LENT: u32 = 1 << 4;
const MOVED: u32 = 1 << 5;

pub fn legend() -> lsp::SemanticTokensLegend {
    lsp::SemanticTokensLegend {
        token_types: TYPES.iter().map(|t| lsp::SemanticTokenType::new(t)).collect(),
        token_modifiers: MODIFIERS.iter().map(|m| lsp::SemanticTokenModifier::new(m)).collect(),
    }
}

fn classify(d: &Def) -> Option<(u32, u32)> {
    Some(match d.kind {
        DefKind::Function => (FUNCTION, 0),
        DefKind::Param => {
            let mods = match d.mode {
                Some(ParamMode::Borrowed) => BORROWED | READONLY,
                Some(ParamMode::Lent) => LENT,
                _ => 0,
            };
            (PARAMETER, mods)
        }
        DefKind::Local | DefKind::Constant => (VARIABLE, if d.mutable { 0 } else { READONLY }),
        DefKind::Field => (PROPERTY, 0),
        DefKind::Record | DefKind::Enum => (TYPE, 0),
        DefKind::Variant => (ENUM_MEMBER, 0),
        DefKind::Result => return None,
    })
}

/// O token de palavra que fecha o trecho: em "a carta" é "carta"; num verbo, ele mesmo.
fn word_span(a: &Analysis, span: Span) -> Option<Span> {
    a.tokens
        .iter()
        .filter(|t| matches!(t.kind, TokenKind::Word(_)) && t.span.start >= span.start && t.span.end <= span.end && !t.span.is_empty())
        .filter(|t| crate::morphology::article(t.word().unwrap_or_default()).is_none() || t.contraction.is_some())
        .max_by_key(|t| t.span.start)
        .map(|t| t.span)
}

/// Os tokens em ordem, sem sobreposição: (trecho, tipo, modificadores).
pub fn classify_all(a: &Analysis) -> Vec<(Span, u32, u32)> {
    let t = &a.tables;
    let mut out: BTreeMap<u32, (Span, u32, u32)> = BTreeMap::new();
    fn put(out: &mut BTreeMap<u32, (Span, u32, u32)>, span: Span, ty: u32, mods: u32) {
        out.entry(span.start).and_modify(|e| e.2 |= mods).or_insert((span, ty, mods));
    }
    for d in &t.defs {
        if let (Some((ty, mods)), Some(ws)) = (classify(d), word_span(a, d.span)) {
            put(&mut out, ws, ty, mods | DECLARATION);
        }
    }
    for r in &t.refs {
        let d = &t.defs[r.def];
        if let (Some((ty, mods)), Some(ws)) = (classify(d), word_span(a, r.span)) {
            let extra = if r.use_ == Use::Write { MODIFICATION } else { 0 };
            put(&mut out, ws, ty, mods | extra);
        }
    }
    // O nome no ponto em que entrega o valor: dali em diante, não se usa mais.
    for o in t.ownership.iter().filter(|o| o.moved) {
        if let Some(e) = word_span(a, o.span).and_then(|ws| out.get_mut(&ws.start)) {
            e.2 |= MOVED;
        }
    }
    // Tipos e casos citados (aposto, tipagem, construção) que não passaram por um uso.
    let v = &a.ast.vocab;
    for tok in &a.tokens {
        let Some(w) = tok.word() else { continue };
        if out.contains_key(&tok.span.start) || tok.contraction.is_some() {
            continue;
        }
        if v.records.contains_key(w) || v.record_plurals.contains_key(w) || v.enums.contains_key(w) || v.enum_plurals.contains_key(w) {
            put(&mut out, tok.span, TYPE, 0);
        } else if v.variants.contains_key(w) {
            put(&mut out, tok.span, ENUM_MEMBER, 0);
        }
    }
    let mut v: Vec<(Span, u32, u32)> = out.into_values().collect();
    // Sem sobreposição: um token que começa dentro do anterior sai.
    let mut last_end = 0;
    v.retain(|(s, _, _)| {
        let keep = s.start >= last_end;
        if keep {
            last_end = s.end;
        }
        keep
    });
    v
}

/// Os tokens no formato relativo do protocolo.
pub fn tokens(a: &Analysis, text: &Text) -> Vec<lsp::SemanticToken> {
    let mut out = Vec::new();
    let (mut prev_line, mut prev_col) = (0u32, 0u32);
    for (span, ty, mods) in classify_all(a) {
        let start = text.position(span.start);
        let end = text.position(span.end);
        if start.line != end.line {
            continue;
        }
        let delta_line = start.line - prev_line;
        let delta_start = if delta_line == 0 { start.character - prev_col } else { start.character };
        out.push(lsp::SemanticToken {
            delta_line,
            delta_start,
            length: end.character - start.character,
            token_type: ty,
            token_modifiers_bitset: mods,
        });
        prev_line = start.line;
        prev_col = start.character;
    }
    out
}
