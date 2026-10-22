//! A ajuda à escrita: completar (guiado pelo próprio parser), ajuda de assinatura, inlay
//! hints, ações rápidas, dobra e seleção.

use super::convert::Text;
use crate::Analysis;
use crate::ast::{FuncSig, ParamMode};
use crate::diagnostics::Diagnostics;
use crate::lexer::{self, LineRole, TokenKind};
use crate::lexicon::{self, Role};
use crate::morphology::{Gender, Number};
use crate::parser::{self, CURSOR, Expect};
use crate::span::Span;
use crate::tables::DefKind;
use lsp_types as lsp;
use std::collections::HashSet;

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    c.next().map(|f| f.to_uppercase().collect::<String>() + c.as_str()).unwrap_or_default()
}

/// A regência de uma assinatura como snippet: "transfiro ${1:quantia} de ${2:origem}".
fn usage_snippet(sig: &FuncSig) -> String {
    let mut n = 0;
    sig.usage()
        .split(' ')
        .map(|w| {
            if w.chars().any(|c| c.is_alphabetic()) && w.chars().all(|c| !c.is_alphabetic() || c.is_uppercase()) {
                n += 1;
                format!("${{{n}:{}}}", w.to_lowercase())
            } else {
                w.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn keyword_item(word: &str, capital: bool) -> lsp::CompletionItem {
    let doc = lexicon::lookup(word).first().map(|e| e.doc.clone());
    lsp::CompletionItem {
        label: if capital { capitalize(word) } else { word.to_string() },
        kind: Some(lsp::CompletionItemKind::KEYWORD),
        documentation: doc.map(|d| lsp::Documentation::MarkupContent(lsp::MarkupContent { kind: lsp::MarkupKind::Markdown, value: d })),
        ..Default::default()
    }
}

/// O que caberia no cursor, pelo parser, preenchido com o vocabulário e os nomes visíveis.
pub fn completion(a: &Analysis, src: &str, offset: u32) -> Vec<lsp::CompletionItem> {
    let off = (offset as usize).min(src.len());
    // O marcador ocupa a palavra inteira em que está o cursor (antes e depois dele).
    let word = |c: &char| c.is_alphanumeric() || *c == '_' || *c == '-';
    let prefix: usize = src[..off].chars().rev().take_while(word).map(char::len_utf8).sum();
    let suffix: usize = src[off..].chars().take_while(word).map(char::len_utf8).sum();
    let start = off - prefix;
    let probe = format!("{}{CURSOR}{}", &src[..start], &src[off + suffix..]);
    let tokens = lexer::tokenize(&probe, &mut Diagnostics::default());
    let Some(idx) = tokens.iter().position(|t| t.is_word(CURSOR)) else { return Vec::new() };
    let capital = idx == 0 || matches!(tokens[idx - 1].kind, TokenKind::Newline | TokenKind::Indent | TokenKind::Dedent);
    let expects = parser::expectations(tokens, idx);
    let t = &a.tables;
    let visible = t.visible_at(offset);
    let mut out: Vec<lsp::CompletionItem> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    let mut push = |item: lsp::CompletionItem, out: &mut Vec<lsp::CompletionItem>| {
        if seen.insert(item.label.clone()) {
            out.push(item);
        }
    };
    for e in expects {
        match e {
            Expect::Word(w) if w.chars().all(|c| c.is_alphabetic() || c == ' ') => push(keyword_item(&w, capital), &mut out),
            Expect::Word(_) => {}
            Expect::ClauseStart => {
                for f in lexicon::clause_verb_forms() {
                    push(keyword_item(f, capital), &mut out);
                }
                for (first, &sig) in &a.ast.vocab.by_first_person {
                    let s = &a.ast.sigs[sig];
                    let snippet = usage_snippet(s);
                    push(
                        lsp::CompletionItem {
                            label: if capital { capitalize(first) } else { first.clone() },
                            kind: Some(lsp::CompletionItemKind::FUNCTION),
                            detail: Some(s.usage()),
                            insert_text: Some(if capital { capitalize(&snippet) } else { snippet }),
                            insert_text_format: Some(lsp::InsertTextFormat::SNIPPET),
                            ..Default::default()
                        },
                        &mut out,
                    );
                }
                // O dado como sujeito: "O saldo vira ...".
                for &i in &visible {
                    let d = &t.defs[i];
                    if matches!(d.kind, DefKind::Local | DefKind::Param) && d.mutable {
                        push(
                            lsp::CompletionItem {
                                label: if capital { capitalize(&d.display) } else { d.display.clone() },
                                kind: Some(lsp::CompletionItemKind::VARIABLE),
                                detail: d.ty.as_ref().map(|t| t.describe()),
                                ..Default::default()
                            },
                            &mut out,
                        );
                    }
                }
            }
            Expect::Name { gender, number } => {
                for &i in &visible {
                    let d = &t.defs[i];
                    let fits = d.gender == gender && d.number == number;
                    let kind = match d.kind {
                        DefKind::Local | DefKind::Param | DefKind::Constant => lsp::CompletionItemKind::VARIABLE,
                        DefKind::Field => lsp::CompletionItemKind::FIELD,
                        _ => continue,
                    };
                    if fits {
                        push(
                            lsp::CompletionItem {
                                label: d.name.clone(),
                                kind: Some(kind),
                                detail: d.ty.as_ref().map(|t| t.describe()),
                                ..Default::default()
                            },
                            &mut out,
                        );
                    }
                }
                // Depois do artigo também vêm as operações por extenso, as posições e "o resultado".
                for n in lexicon::NOMINALS {
                    if n.gender == gender && number == Number::Singular {
                        push(keyword_item(n.word, false), &mut out);
                    }
                }
                for &(stem, _) in lexicon::ORDINALS {
                    let end = match (gender, number) {
                        (Gender::Masculine, Number::Singular) => "o",
                        (Gender::Feminine, Number::Singular) => "a",
                        (Gender::Masculine, Number::Plural) => "os",
                        (Gender::Feminine, Number::Plural) => "as",
                    };
                    push(keyword_item(&format!("{stem}{end}"), false), &mut out);
                }
                if gender == Gender::Masculine && number == Number::Singular {
                    for w in ["item", "resultado", "fato"] {
                        push(keyword_item(w, false), &mut out);
                    }
                }
            }
            Expect::Value => {
                for &i in &visible {
                    let d = &t.defs[i];
                    if matches!(d.kind, DefKind::Local | DefKind::Param | DefKind::Constant) {
                        push(
                            lsp::CompletionItem {
                                label: d.display.clone(),
                                kind: Some(lsp::CompletionItemKind::VARIABLE),
                                detail: d.ty.as_ref().map(|t| t.describe()),
                                ..Default::default()
                            },
                            &mut out,
                        );
                    }
                }
            }
            Expect::Type { plural } => {
                for ty in lexicon::TYPE_NOUNS {
                    let w = if plural { ty.plural } else { ty.singular };
                    let mut item = keyword_item(w, false);
                    item.kind = Some(lsp::CompletionItemKind::CLASS);
                    push(item, &mut out);
                }
                let v = &a.ast.vocab;
                let names = if plural { [&v.record_plurals, &v.enum_plurals] } else { [&v.records, &v.enums] };
                for map in names {
                    for w in map.keys() {
                        push(
                            lsp::CompletionItem { label: w.clone(), kind: Some(lsp::CompletionItemKind::STRUCT), ..Default::default() },
                            &mut out,
                        );
                    }
                }
            }
        }
    }
    out
}

/// A regência do verbo da oração em que está o cursor, com o parâmetro ativo.
pub fn signature_help(a: &Analysis, offset: u32) -> Option<lsp::SignatureHelp> {
    let toks: Vec<&lexer::Token> = a.tokens.iter().filter(|t| t.span.start < offset).collect();
    // O começo da linha lógica (ou do período) em que está o cursor.
    let begin = toks
        .iter()
        .rposition(|t| matches!(t.kind, TokenKind::Newline | TokenKind::Indent | TokenKind::Dedent) || t.is_punct('.'))
        .map_or(0, |i| i + 1);
    let clause = &toks[begin..];
    let v = &a.ast.vocab;
    let (at, sig) = clause.iter().enumerate().rev().find_map(|(i, t)| {
        let w = t.word()?;
        if let Some(&s) = v.by_first_person.get(w) {
            return Some((i, s));
        }
        // "o resultado de <infinitivo>"
        if i > 0 && clause[i - 1].is_word("de") {
            return v.by_infinitive.get(w).and_then(|s| s.first()).map(|&s| (i, s));
        }
        None
    })?;
    let sig = &a.ast.sigs[sig];
    let usage = sig.usage();
    // Os parâmetros são as palavras em maiúsculas da regência.
    let mut params = Vec::new();
    let mut pos = 0u32;
    for w in usage.split(' ') {
        let len = w.chars().count() as u32;
        if w.chars().any(|c| c.is_alphabetic()) && w.chars().all(|c| !c.is_alphabetic() || c.is_uppercase()) {
            params.push(lsp::ParameterInformation { label: lsp::ParameterLabel::LabelOffsets([pos, pos + len]), documentation: None });
        }
        pos += len + 1;
    }
    // O parâmetro ativo: o objeto, até aparecer a preposição de um grupo regido.
    let mut active = 0u32;
    let mut index = 0u32;
    let preps: Vec<(String, u32)> = sig
        .slots
        .iter()
        .filter_map(|s| match s {
            crate::ast::Slot::Object | crate::ast::Slot::Subject => {
                index += 1;
                None
            }
            crate::ast::Slot::Prep { prep, count } => {
                let first = index;
                index += *count as u32;
                Some((prep.clone(), first))
            }
            crate::ast::Slot::Fixed { .. } => None,
        })
        .collect();
    for t in &clause[at + 1..] {
        if let Some(w) = t.word()
            && let Some((_, first)) = preps.iter().find(|(p, _)| p == w)
        {
            active = *first;
        }
    }
    Some(lsp::SignatureHelp {
        signatures: vec![lsp::SignatureInformation {
            label: usage,
            documentation: None,
            parameters: Some(params),
            active_parameter: Some(active),
        }],
        active_signature: Some(0),
        active_parameter: Some(active),
    })
}

/// Dicas no texto: o tipo das declarações sem aposto, o modo dos argumentos emprestados e
/// confiados, e o tipo de "o resultado".
/// Quais inlay hints o editor quer: o tipo das declarações sem aposto, o modo dos
/// argumentos e o tipo de "o resultado". Todos começam ligados; o editor desliga pela
/// configuração "inlayHints" (nas opções de inicialização ou em
/// `workspace/didChangeConfiguration`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InlayOptions {
    pub types: bool,
    pub modes: bool,
    pub result: bool,
}

impl Default for InlayOptions {
    fn default() -> Self {
        InlayOptions { types: true, modes: true, result: true }
    }
}

impl InlayOptions {
    /// Lê `{ "types": false, ... }`; o que não vier fica como estava.
    pub fn update(&mut self, v: &serde_json::Value) {
        for (key, field) in [("types", &mut self.types), ("modes", &mut self.modes), ("result", &mut self.result)] {
            if let Some(on) = v.get(key).and_then(serde_json::Value::as_bool) {
                *field = on;
            }
        }
    }
}

pub fn inlay_hints(a: &Analysis, src: &str, text: &Text, range: lsp::Range, options: InlayOptions) -> Vec<lsp::InlayHint> {
    let (from, to) = (text.offset(range.start), text.offset(range.end));
    let inside = |s: Span| s.start >= from && s.start <= to;
    let mut out = Vec::new();
    // O preenchimento só onde o texto não tem espaço: o tipo da declaração e o de "o
    // resultado" encostam no nome; o modo do argumento vem antes dele, com um espaço.
    let hint = |at: u32, label: String, right: bool, kind| lsp::InlayHint {
        position: text.position(at),
        label: lsp::InlayHintLabel::String(label),
        kind: Some(kind),
        text_edits: None,
        tooltip: None,
        padding_left: Some(false),
        padding_right: Some(right),
        data: None,
    };
    let declares = |w: &str| lexicon::lookup(w).iter().any(|e| e.role == Role::Declaration);
    for d in a.tables.defs.iter().filter(|_| options.types) {
        if !matches!(d.kind, DefKind::Local | DefKind::Constant) || !inside(d.span) {
            continue;
        }
        // Só na declaração sem aposto: "O saldo começa em 0." (com aposto, já tem o tipo).
        let next = a.tokens.iter().find(|t| t.span.start >= d.span.end && !t.span.is_empty());
        if let (Some(ty), Some(n)) = (&d.ty, next)
            && n.word().is_some_and(declares)
            && !ty.is_error()
        {
            out.push(hint(d.span.end, format!(", {},", ty.describe()), false, lsp::InlayHintKind::TYPE));
        }
    }
    for &(span, mode) in a.tables.args.iter().filter(|_| options.modes) {
        if !inside(span) {
            continue;
        }
        let label = match mode {
            ParamMode::Lent => "confiado:",
            ParamMode::Borrowed => "emprestado:",
            ParamMode::Owned => continue,
        };
        out.push(hint(span.start, label.into(), true, lsp::InlayHintKind::PARAMETER));
    }
    for t in a.tokens.iter().filter(|_| options.result) {
        if t.is_word("resultado")
            && inside(t.span)
            && !src[t.span.end as usize..].trim_start().starts_with("de ")
            && let Some((_, ty)) = a.tables.type_at(t.span.start).filter(|(_, ty)| !ty.is_error())
        {
            out.push(hint(t.span.end, format!(": {}", ty.describe()), false, lsp::InlayHintKind::TYPE));
        }
    }
    out
}

/// As correções que o compilador sabe fazer, dos diagnósticos que o editor manda.
pub fn code_actions(uri: &lsp::Uri, diagnostics: &[lsp::Diagnostic]) -> Vec<lsp::CodeActionOrCommand> {
    let mut out = Vec::new();
    for d in diagnostics {
        let Some(fixes) = d.data.as_ref().and_then(|v| v.get("fixes")).and_then(|f| f.as_array()) else { continue };
        for f in fixes {
            let edits: Vec<lsp::TextEdit> = f["edits"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|e| {
                    Some(lsp::TextEdit {
                        range: serde_json::from_value(e["range"].clone()).ok()?,
                        new_text: e["text"].as_str()?.to_string(),
                    })
                })
                .collect();
            out.push(lsp::CodeActionOrCommand::CodeAction(lsp::CodeAction {
                title: f["title"].as_str().unwrap_or("Corrigir").to_string(),
                kind: Some(lsp::CodeActionKind::QUICKFIX),
                diagnostics: Some(vec![d.clone()]),
                edit: Some(super::convert::workspace_edit(uri, edits)),
                is_preferred: f["safe"].as_bool(),
                ..Default::default()
            }));
        }
    }
    out
}

/// "Corrigir tudo": as correções seguras do documento, volta após volta (as mesmas do
/// `lavrac fix`), numa ação só. Sem correção segura no documento, não há ação.
pub fn fix_all(a: &Analysis, src: &str, text: &Text, uri: &lsp::Uri) -> Option<lsp::CodeActionOrCommand> {
    if !a.diagnostics.iter().any(|d| d.fixes.iter().any(|f| f.safe)) {
        return None;
    }
    let (fixed, _) = crate::fix::fix_all(src);
    if fixed.is_empty() {
        return None;
    }
    let edits = fixed.edits().iter().map(|e| lsp::TextEdit { range: text.range(e.span), new_text: e.text.clone() }).collect();
    Some(lsp::CodeActionOrCommand::CodeAction(lsp::CodeAction {
        title: "Corrigir tudo o que é seguro".into(),
        kind: Some(lsp::CodeActionKind::SOURCE_FIX_ALL),
        edit: Some(super::convert::workspace_edit(uri, edits)),
        ..Default::default()
    }))
}

/// A última linha que pertence ao bloco aberto na linha `i` (de profundidade `depth`).
fn block_end(roles: &[LineRole], i: usize, depth: usize) -> usize {
    let mut end = i;
    for (j, r) in roles.iter().enumerate().skip(i + 1) {
        match r {
            LineRole::Start(d) if *d <= depth => break,
            LineRole::Blank => {}
            _ => end = j,
        }
    }
    end
}

/// Dobra: cada bloco recuado e cada sequência de comentários.
pub fn folding(src: &str) -> Vec<lsp::FoldingRange> {
    let roles = lexer::layout(src);
    let mut out = Vec::new();
    for (i, r) in roles.iter().enumerate() {
        if let LineRole::Start(d) = r {
            let end = block_end(&roles, i, *d);
            if end > i {
                out.push(lsp::FoldingRange { start_line: i as u32, end_line: end as u32, ..Default::default() });
            }
        }
        if *r == LineRole::Comment && (i == 0 || roles[i - 1] != LineRole::Comment) {
            let end = roles[i..].iter().take_while(|r| **r == LineRole::Comment).count() + i - 1;
            if end > i {
                out.push(lsp::FoldingRange {
                    start_line: i as u32,
                    end_line: end as u32,
                    kind: Some(lsp::FoldingRangeKind::Comment),
                    ..Default::default()
                });
            }
        }
    }
    out
}

/// Seleção crescente: a palavra, a oração (a linha lógica), cada bloco em volta, o texto.
pub fn selection(a: &Analysis, src: &str, text: &Text, offset: u32) -> lsp::SelectionRange {
    let roles = lexer::layout(src);
    let lines = text.lines;
    let line_span = |first: usize, last: usize| {
        let start = lines.line_start(first, src);
        let end = lines.line_start(last + 1, src).min(src.len());
        let body = &src[start..end];
        let lead = body.len() - body.trim_start().len();
        Span::new(start + lead, start + body.trim_end().len())
    };
    let mut ranges: Vec<Span> = Vec::new();
    if let Some(t) = a.tokens.iter().find(|t| t.span.start <= offset && offset <= t.span.end && !t.span.is_empty()) {
        ranges.push(t.span);
    }
    let line = lines.line(offset as usize);
    // A oração: da linha que a abriu até a última continuação.
    if let Some(start) = (0..=line.min(roles.len().saturating_sub(1))).rev().find(|&i| matches!(roles[i], LineRole::Start(_))) {
        let mut end = start;
        while end + 1 < roles.len() && roles[end + 1] == LineRole::Continuation {
            end += 1;
        }
        ranges.push(line_span(start, end));
        // Os blocos em volta, de dentro para fora.
        let mut depth = match roles[start] {
            LineRole::Start(d) => d,
            _ => 0,
        };
        let mut at = start;
        while depth > 0 {
            if let Some(head) = (0..at).rev().find(|&i| matches!(roles[i], LineRole::Start(d) if d < depth)) {
                let LineRole::Start(d) = roles[head] else { break };
                ranges.push(line_span(head, block_end(&roles, head, d)));
                depth = d;
                at = head;
            } else {
                break;
            }
        }
    }
    ranges.push(Span::new(0, src.len()));
    ranges.dedup();
    let mut parent: Option<Box<lsp::SelectionRange>> = None;
    for s in ranges.iter().rev() {
        parent = Some(Box::new(lsp::SelectionRange { range: text.range(*s), parent }));
    }
    *parent.unwrap_or_else(|| Box::new(lsp::SelectionRange { range: text.range(Span::new(0, 0)), parent: None }))
}
