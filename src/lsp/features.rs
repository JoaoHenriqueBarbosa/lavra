//! As consultas do editor sobre um documento: hover, definição, referências, destaque e
//! símbolos. Tudo sai das tabelas que a própria análise produziu.

use super::convert::Text;
use crate::Analysis;
use crate::ast::ParamMode;
use crate::lexicon;
use crate::tables::{Def, DefKind, Use};
use lsp_types as lsp;

/// A palavra da língua na posição, se houver (para o hover das palavras-chave).
fn word_at(a: &Analysis, offset: u32) -> Option<String> {
    a.tokens
        .iter()
        .find(|t| t.span.start <= offset && offset < t.span.end.max(t.span.start + 1))
        .and_then(|t| t.contraction.map(|c| c.written.to_string()).or_else(|| t.word().map(str::to_string)))
}

fn mode_word(mode: ParamMode, d: &Def) -> String {
    let base = match mode {
        ParamMode::Owned => return "recebido com a posse".to_string(),
        ParamMode::Borrowed => "emprestado",
        ParamMode::Lent => "confiado",
    };
    crate::morphology::inflect_adjective(base, crate::morphology::Gender::Masculine, d.gender, d.number)
}

/// O texto do hover de um nome declarado.
pub fn describe(a: &Analysis, text: &Text, def: usize) -> String {
    let t = &a.tables;
    let d = &t.defs[def];
    let line = text.position(d.span.start).line + 1;
    let ty = d.ty.as_ref().map(|t| t.describe()).unwrap_or_else(|| "de tipo desconhecido".into());
    match d.kind {
        DefKind::Function => {
            let sig = a.ast.sigs.iter().find(|s| s.name_span == d.span);
            let mut s = format!("```lavra\n{}\n```\n", d.display);
            if let Some(r) = d.ty.as_ref().filter(|r| !matches!(r, crate::types::Ty::Unit)) {
                s.push_str(&format!("Devolve {}.", r.describe()));
            } else {
                s.push_str("Não devolve valor.");
            }
            if sig.is_some_and(|s| s.fallible) {
                s.push_str(" Pode falhar (\"salvo uma falha\").");
            }
            s.push_str(&format!("\n\nDefinido na linha {line}."));
            s
        }
        DefKind::Param => {
            let func = d.owner.map(|o| t.defs[o].name.clone()).unwrap_or_default();
            let mode = d.mode.map(|m| mode_word(m, d)).unwrap_or_default();
            format!("**{}**: parâmetro de \"{func}\", {ty}, {mode}.", d.display)
        }
        DefKind::Local | DefKind::Constant => {
            let what = match (d.kind, d.mutable) {
                (DefKind::Constant, _) => "constante do programa",
                (_, true) => "variável",
                (_, false) => "fixo",
            };
            format!("**{}**: {what}, {ty}.\n\nDeclarado na linha {line}.", d.display)
        }
        DefKind::Field => {
            let owner = d.owner.map(|o| t.defs[o].display.clone()).unwrap_or_default();
            format!("**{}**: campo de {owner}, {ty}.", d.display)
        }
        DefKind::Record | DefKind::Enum => {
            let parts: Vec<String> = t.defs.iter().filter(|x| x.owner == Some(def)).map(|x| x.display.clone()).collect();
            let what = if d.kind == DefKind::Record { "registro com" } else { "enumeração com os casos" };
            format!("**{}**: {what} {}.", d.display, crate::textfmt::join_e(&parts))
        }
        DefKind::Variant => {
            let owner = d.owner.map(|o| t.defs[o].display.clone()).unwrap_or_default();
            format!("**{}**: caso de {owner}.", d.display)
        }
        DefKind::Result => format!("**o resultado**: {ty}."),
    }
}

pub fn hover(a: &Analysis, text: &Text, offset: u32) -> Option<lsp::Hover> {
    let value = if let Some(def) = a.tables.def_at(offset) {
        describe(a, text, def)
    } else if let Some((_, ty)) = a.tables.type_at(offset) {
        format!("{}.", crate::sema::capitalize(&ty.describe()))
    } else {
        let w = word_at(a, offset)?;
        let entries = lexicon::lookup(&w);
        let e = entries.first()?;
        format!("**{}** ({})\n\n{}", e.form, e.role.describe(), e.doc)
    };
    Some(lsp::Hover { contents: lsp::HoverContents::Markup(lsp::MarkupContent { kind: lsp::MarkupKind::Markdown, value }), range: None })
}

pub fn definition(a: &Analysis, text: &Text, uri: &lsp::Uri, offset: u32) -> Option<lsp::Location> {
    let def = a.tables.def_at(offset)?;
    Some(lsp::Location { uri: uri.clone(), range: text.range(a.tables.defs[def].span) })
}

pub fn references(a: &Analysis, text: &Text, uri: &lsp::Uri, offset: u32, declaration: bool) -> Vec<lsp::Location> {
    let Some(def) = a.tables.def_at(offset) else { return Vec::new() };
    let mut out = Vec::new();
    if declaration {
        out.push(lsp::Location { uri: uri.clone(), range: text.range(a.tables.defs[def].span) });
    }
    out.extend(a.tables.refs_of(def).iter().map(|r| lsp::Location { uri: uri.clone(), range: text.range(r.span) }));
    out
}

pub fn highlights(a: &Analysis, text: &Text, offset: u32) -> Vec<lsp::DocumentHighlight> {
    let Some(def) = a.tables.def_at(offset) else { return Vec::new() };
    let mut out =
        vec![lsp::DocumentHighlight { range: text.range(a.tables.defs[def].span), kind: Some(lsp::DocumentHighlightKind::WRITE) }];
    out.extend(a.tables.refs_of(def).iter().map(|r| lsp::DocumentHighlight {
        range: text.range(r.span),
        kind: Some(if r.use_ == Use::Write { lsp::DocumentHighlightKind::WRITE } else { lsp::DocumentHighlightKind::READ }),
    }));
    out
}

#[allow(deprecated)] // `DocumentSymbol::deprecated` é campo obrigatório do tipo.
fn symbol(
    name: String,
    detail: Option<String>,
    kind: lsp::SymbolKind,
    range: lsp::Range,
    sel: lsp::Range,
    children: Vec<lsp::DocumentSymbol>,
) -> lsp::DocumentSymbol {
    lsp::DocumentSymbol {
        name,
        detail,
        kind,
        tags: None,
        deprecated: None,
        range,
        selection_range: sel,
        children: (!children.is_empty()).then_some(children),
    }
}

/// Os símbolos do documento: funções (com a regência), registros com os campos,
/// enumerações com os casos e as constantes do programa.
pub fn document_symbols(a: &Analysis, text: &Text) -> Vec<lsp::DocumentSymbol> {
    let t = &a.tables;
    let mut out = Vec::new();
    for (i, d) in t.defs.iter().enumerate() {
        match d.kind {
            DefKind::Function => {
                let body = a
                    .ast
                    .funcs
                    .iter()
                    .find(|f| a.ast.sigs[f.sig].name_span == d.span)
                    .map(|f| a.ast.sigs[f.sig].span.to(f.body.span))
                    .unwrap_or(d.span);
                out.push(symbol(
                    d.name.clone(),
                    Some(d.display.clone()),
                    lsp::SymbolKind::FUNCTION,
                    text.range(body),
                    text.range(d.span),
                    Vec::new(),
                ));
            }
            DefKind::Record | DefKind::Enum => {
                let children: Vec<lsp::DocumentSymbol> = t
                    .defs
                    .iter()
                    .filter(|x| x.owner == Some(i))
                    .map(|x| {
                        let kind = if x.kind == DefKind::Variant { lsp::SymbolKind::ENUM_MEMBER } else { lsp::SymbolKind::FIELD };
                        let detail = x.ty.as_ref().filter(|_| x.kind == DefKind::Field).map(|t| t.describe());
                        symbol(x.display.clone(), detail, kind, text.range(x.span), text.range(x.span), Vec::new())
                    })
                    .collect();
                let kind = if d.kind == DefKind::Record { lsp::SymbolKind::STRUCT } else { lsp::SymbolKind::ENUM };
                let range = children.iter().fold(d.span, |acc, _| acc);
                out.push(symbol(d.name.clone(), None, kind, text.range(range), text.range(d.span), children));
            }
            DefKind::Constant => {
                let detail = d.ty.as_ref().map(|t| t.describe());
                out.push(symbol(d.display.clone(), detail, lsp::SymbolKind::CONSTANT, text.range(d.span), text.range(d.span), Vec::new()));
            }
            _ => {}
        }
    }
    out
}
