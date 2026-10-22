//! Cabeçalhos de função ("Para transferir a quantia da origem para o destino, ...")
//! e as chamadas que eles regem: a assinatura diz quantos objetos vêm, quais
//! preposições e em que ordem.

use super::Parser;
use crate::ast::*;
use crate::codes::*;
use crate::diagnostics::{Diagnostic, Result, err};
use crate::lexer::TokenKind;
use crate::lexicon;
use crate::morphology::{self, Gender, Number};
use crate::span::Span;

impl Parser {
    /// Acha os cabeçalhos do nível de fora antes de ler os corpos, para que uma função
    /// possa ser chamada antes de definida. Cabeçalho com erro vira assinatura quebrada
    /// (o que se leu até o erro), e o corpo é lido do mesmo jeito.
    pub(super) fn prescan(&mut self) {
        let mut depth = 0usize;
        let mut at_start = true;
        let mut k = 0;
        while k < self.toks.len() {
            let t = &self.toks[k];
            match t.kind {
                TokenKind::Indent => depth += 1,
                TokenKind::Dedent => depth = depth.saturating_sub(1),
                _ => {}
            }
            if depth == 0
                && at_start
                && t.is_word("para")
                && !self.toks.get(k + 1).is_some_and(|n| n.is_word("cada") || n.is_word("começar"))
            {
                self.pos = k;
                let mut draft = Draft::default();
                match self.header(&mut draft) {
                    Ok(sig) => {
                        let id = self.prog.sigs.len();
                        if let Err(d) = self.register(id, &sig) {
                            self.report(d);
                        }
                        self.prog.sigs.push(sig);
                        self.headers.insert(k, (id, self.pos));
                        k = self.pos;
                    }
                    Err(d) => {
                        let end = self.line_end_from(self.pos.max(k));
                        self.report_in(k, end, d);
                        match draft.into_sig(self.toks[end.saturating_sub(1).max(k)].span) {
                            Some(sig) => {
                                let id = self.prog.sigs.len();
                                if let Err(d) = self.register(id, &sig) {
                                    self.report(d);
                                }
                                self.prog.sigs.push(sig);
                                self.headers.insert(k, (id, end));
                            }
                            None => {
                                let e = self.definition_end(k);
                                self.skip_spans.insert(k, e);
                            }
                        }
                        k = end;
                    }
                }
                continue;
            }
            at_start = matches!(t.kind, TokenKind::Newline | TokenKind::Dedent | TokenKind::Indent);
            k += 1;
        }
        self.pos = 0;
    }

    /// "Uso a biblioteca de textos.", no nível de fora: os cabeçalhos da biblioteca entram
    /// no programa antes dos dele, e a frase sai da leitura do programa.
    pub(super) fn imports(&mut self) {
        let mut depth = 0usize;
        let mut at_start = true;
        let mut k = 0;
        let mut used: Vec<(&'static str, Span)> = Vec::new();
        while k < self.toks.len() {
            let t = &self.toks[k];
            match t.kind {
                TokenKind::Indent => depth += 1,
                TokenKind::Dedent => depth = depth.saturating_sub(1),
                _ => {}
            }
            let is_import = depth == 0
                && at_start
                && t.is_word("uso")
                && self.toks.get(k + 1).is_some_and(|x| x.is_word("a"))
                && self.toks.get(k + 2).is_some_and(|x| x.is_word("biblioteca"));
            if is_import {
                let end = self.line_end_from(k);
                self.skip_spans.insert(k, end);
                if let Err(d) = self.import_at(k, &mut used) {
                    self.report(d);
                }
                k = end;
                continue;
            }
            at_start = matches!(t.kind, TokenKind::Newline | TokenKind::Dedent | TokenKind::Indent);
            k += 1;
        }
        self.pos = 0;
    }

    /// Uma frase "Uso a biblioteca de X." que começa no token `k`.
    fn import_at(&mut self, k: usize, used: &mut Vec<(&'static str, Span)>) -> Result<()> {
        let tok = |i: usize| self.toks.get(k + i).cloned();
        let end = self.line_end_from(k);
        let start = self.toks[k].span.to(self.toks[end.saturating_sub(1).max(k)].span);
        let names: Vec<&str> = crate::builtins::LIBRARIES.iter().map(|l| l.name).collect();
        let form = || {
            Diagnostic::new(L0173, "A biblioteca se pede assim: \"Uso a biblioteca de textos.\"", start)
                .hint(format!("As bibliotecas são: {}.", names.join(", ")))
        };
        let (Some(de), Some(name_tok)) = (tok(3), tok(4)) else { return err(form()) };
        let Some(name) = name_tok.word().filter(|_| de.is_word("de") && de.contraction.is_none()).map(str::to_string) else {
            return err(form());
        };
        if !tok(5).is_some_and(|t| t.is_punct('.')) {
            return err(form());
        }
        let span = self.toks[k].span.to(name_tok.span);
        let Some((base, lib)) = crate::builtins::library(&name) else {
            let near = names.iter().find(|n| morphology::edit_distance(n, &name) <= 2);
            let d = Diagnostic::new(L0174, format!("Não há biblioteca de \"{}\".", name_tok.orig), name_tok.span)
                .hint(format!("As bibliotecas são: {}.", names.join(", ")));
            return err(match near {
                Some(n) => d.fix(format!("Escrever \"{n}\""), name_tok.span, *n),
                None => d,
            });
        };
        if let Some((_, first)) = used.iter().find(|(n, _)| *n == lib.name) {
            self.report(Box::new(
                Diagnostic::new(L0504, format!("A biblioteca de {} já foi trazida.", lib.name), span).label(*first, "trazida aqui"),
            ));
            return Ok(());
        }
        used.push((lib.name, span));
        // Os cabeçalhos da biblioteca, lidos pelo mesmo parser, com as posições na faixa dela.
        let mut lib_diags = crate::diagnostics::Diagnostics::default();
        let mut toks = crate::lexer::tokenize(lib.source, &mut lib_diags);
        for t in &mut toks {
            t.span = Span { start: t.span.start + base, end: t.span.end + base };
        }
        let saved = std::mem::replace(&mut self.toks, toks);
        self.library = Some(lib.name);
        self.library_headers();
        self.library = None;
        self.toks = saved;
        Ok(())
    }

    /// Os cabeçalhos de uma biblioteca: cada um vira assinatura do programa.
    fn library_headers(&mut self) {
        let mut at_start = true;
        let mut k = 0;
        while k < self.toks.len() {
            if at_start && self.toks[k].is_word("para") {
                self.pos = k;
                let mut draft = Draft::default();
                match self.header(&mut draft) {
                    Ok(sig) => {
                        let id = self.prog.sigs.len();
                        if let Err(d) = self.register(id, &sig) {
                            self.report(d);
                        }
                        self.prog.sigs.push(sig);
                        k = self.pos;
                    }
                    Err(d) => {
                        self.report(d);
                        k = self.line_end_from(k);
                    }
                }
                continue;
            }
            at_start = matches!(self.toks[k].kind, TokenKind::Newline | TokenKind::Dedent | TokenKind::Indent);
            k += 1;
        }
        self.pos = 0;
    }

    fn register(&mut self, id: usize, sig: &FuncSig) -> Result<()> {
        // O verbo do programa não pode ser o de uma biblioteca que ele trouxe.
        if sig.builtin.is_none()
            && let Some(o) = self.prog.sigs.iter().find(|o| {
                o.builtin.is_some()
                    && (o.infinitive == sig.infinitive
                        || (o.kind == SigKind::Action && sig.kind == SigKind::Action && o.first_person == sig.first_person))
            })
        {
            let (lib, _) = o.builtin.expect("de biblioteca").declared_as();
            return err(Diagnostic::new(
                L0175,
                format!("\"{}\" já vem da biblioteca de {lib}, que o programa usa.", o.infinitive),
                sig.name_span,
            )
            .hint(format!("Escolha outro verbo para a sua função, ou deixe de usar a biblioteca de {lib}.")));
        }
        if let SigKind::Adjective { .. } = sig.kind {
            return Ok(());
        }
        if sig.kind == SigKind::Predicate && sig.infinitive == "ir" {
            return err(Diagnostic::new(
                L0150,
                "Predicado com \"ir\" não pode: no subjuntivo ele dá \"for\", igual a \"ser\", que a língua usa nas comparações.",
                sig.name_span,
            )
            .hint("Escolha outro verbo: \"seguir\", \"partir\", \"chegar\"."));
        }
        if lexicon::is_core_infinitive(&sig.infinitive) {
            return err(Diagnostic::new(
                L0151,
                format!("\"{}\" é verbo da própria língua e não se redefine.", sig.infinitive),
                sig.name_span,
            )
            .hint("Escolha outro verbo para a sua função."));
        }
        if sig.kind == SigKind::Action {
            if Self::is_core_verb(&sig.first_person) {
                return err(Diagnostic::new(
                    L0153,
                    format!("A 1ª pessoa de \"{}\" é \"{}\", que já é verbo da língua.", sig.infinitive, sig.first_person),
                    sig.name_span,
                ));
            }
            if lexicon::reserved(&sig.first_person).is_some() {
                return err(Diagnostic::new(
                    L0152,
                    format!(
                        "A 1ª pessoa de \"{}\" é \"{}\", que já é palavra da língua; a oração ficaria ambígua.",
                        sig.infinitive, sig.first_person
                    ),
                    sig.name_span,
                )
                .hint("Escolha outro verbo."));
            }
            if let Some(&other) = self.vocab.by_first_person.get(&sig.first_person) {
                let o = &self.prog.sigs[other];
                if o.infinitive != sig.infinitive {
                    return err(Diagnostic::new(
                        L0154,
                        format!("\"{}\" e \"{}\" se conjugam igual: \"{}\".", o.infinitive, sig.infinitive, sig.first_person),
                        sig.name_span,
                    )
                    .label(o.name_span, "o outro verbo"));
                }
            }
            self.vocab.by_first_person.insert(sig.first_person.clone(), id);
        }
        let same: Vec<usize> = self.vocab.by_infinitive.get(&sig.infinitive).cloned().unwrap_or_default();
        for other in same {
            let o = &self.prog.sigs[other];
            if o.slots == sig.slots {
                return err(Diagnostic::new(
                    L0155,
                    format!("\"{}\" já foi definido com essa mesma regência.", sig.infinitive),
                    sig.name_span,
                )
                .label(o.name_span, "definido aqui"));
            }
        }
        self.vocab.by_infinitive.entry(sig.infinitive.clone()).or_default().push(id);
        Ok(())
    }

    /// Um parâmetro no cabeçalho: nome com artigo definido, ou símbolo. O "a" seguido
    /// de símbolo é preposição ("elevar x a n").
    fn param_start(&self) -> bool {
        let t = self.peek();
        if let Some(w) = t.word()
            && morphology::is_symbol(w)
        {
            return true;
        }
        self.definite_article_at(0).is_some() && !self.peek_at(1).word().is_some_and(morphology::is_symbol)
    }

    fn prep_at(&self) -> Option<String> {
        let t = self.peek();
        let w = t.word()?;
        if !morphology::is_preposition(w) {
            return None;
        }
        // "a" sozinho só é preposição antes de símbolo; antes de nome é artigo.
        if t.contraction.is_none() && morphology::article(w).is_some() && !self.peek_at(1).word().is_some_and(morphology::is_symbol) {
            return None;
        }
        Some(w.to_string())
    }

    pub(super) fn header(&mut self, draft: &mut Draft) -> Result<FuncSig> {
        let start = self.expect_word("para")?.span;
        draft.start = Some(start);
        let mut params: Vec<(NameRef, Slot)> = Vec::new();
        if self.param_start() {
            let subject = self.name_ref()?;
            draft.params.push(subject.clone());
            draft.kind = Some(SigKind::Predicate);
            params.push((subject, Slot::Subject));
        }
        let vt = self.bump();
        let Some(infinitive) = vt.word().map(str::to_string).filter(|w| morphology::verbs::is_infinitive(w)) else {
            return err(Diagnostic::new(
                L0156,
                format!("Depois de \"Para\" vem um verbo no infinitivo, mas encontrei {}.", vt.describe()),
                vt.span,
            )
            .hint("Exemplos: \"Para somar x e y\", \"Para medir a carta\"."));
        };
        let name_span = vt.span;
        draft.infinitive = Some(infinitive.clone());
        draft.name_span = name_span;
        let mut kind = if params.is_empty() { SigKind::Action } else { SigKind::Predicate };
        if !params.is_empty() && infinitive == "ser" {
            let adj = self.bump();
            let Some(a) = adj.word().map(str::to_string) else {
                return Err(self.expected("o adjetivo do predicado (\"Para o número ser par\")"));
            };
            let g = params[0].0.gender();
            let base = if params[0].0.number() == Number::Plural { a.trim_end_matches('s').to_string() } else { a };
            kind = SigKind::Adjective { adjective: base, base_gender: g, span: adj.span };
        }
        draft.kind = Some(kind.clone());
        let regular = morphology::first_person(&infinitive);
        let mut first_person = regular.clone();
        let mut first_person_span = None;
        if self.at_punct('(') {
            self.bump();
            let f = self.bump();
            let Some(w) = f.word() else { return Err(self.expected("a 1ª pessoa do verbo entre parênteses")) };
            first_person = Some(w.to_string());
            first_person_span = Some(f.span);
            draft.first_person = first_person.clone();
            draft.first_person_span = first_person_span;
            self.expect_punct(')', "o parêntese de fechamento")?;
        }
        let first_person = match first_person {
            Some(f) => f,
            None if kind == SigKind::Action => {
                return err(Diagnostic::new(
                    L0157,
                    format!("\"{infinitive}\" é defectivo: não tem 1ª pessoa do presente, e é ela que executa."),
                    name_span,
                )
                .hint(format!("Escolha outro verbo, ou declare a forma entre parênteses: \"Para {infinitive} (...) ...\".")));
            }
            None => infinitive.clone(),
        };
        let mut slots = Vec::new();
        if !params.is_empty() {
            slots.push(Slot::Subject);
        }
        // Objetos diretos.
        while self.param_start() {
            let p = self.name_ref()?;
            draft.params.push(p.clone());
            params.push((p, Slot::Object));
            slots.push(Slot::Object);
            if self.at_word("e")
                || (self.at_punct(',') && !self.peek_at(1).word().is_some_and(|w| w == "sendo" || w == "devolvendo" || w == "salvo"))
            {
                let save = self.pos;
                self.bump();
                if !self.param_start() {
                    self.pos = save;
                    break;
                }
            }
        }
        // Grupos regidos por preposição.
        while let Some(prep) = self.prep_at() {
            self.bump();
            let p = self.name_ref()?;
            draft.params.push(p.clone());
            let mut count = 1;
            params.push((p, Slot::Prep { prep: prep.clone(), count: 0 }));
            if self.at_word("e") {
                self.bump();
                let q = self.name_ref()?;
                draft.params.push(q.clone());
                params.push((q, Slot::Prep { prep: prep.clone(), count: 0 }));
                count = 2;
            }
            slots.push(Slot::Prep { prep, count });
        }
        // Tipagem e retorno.
        let mut typed: Vec<Typed> = Vec::new();
        let mut ret = None;
        let mut fallible = false;
        while self.eat_punct(',') {
            if self.eat_word("sendo") {
                loop {
                    let group = self.typing(&params)?;
                    typed.extend(group);
                    if self.at_word("e")
                        || (self.at_punct(',') && !self.peek_at(1).word().is_some_and(|w| w == "devolvendo" || w == "salvo"))
                    {
                        self.bump();
                        continue;
                    }
                    break;
                }
            } else if self.eat_word("devolvendo") {
                let at = self.bump();
                let Some(art) = at.word().and_then(morphology::article).filter(|a| !a.definite && a.number == Number::Singular) else {
                    return err(Diagnostic::new(
                        L0158,
                        format!("Esperava o tipo devolvido com artigo indefinido (\"um real\"), mas encontrei {}.", at.describe()),
                        at.span,
                    ));
                };
                let s0 = at.span;
                let ty = self.type_noun(art, at.span)?;
                ret = Some(TypeAnn { ty, span: s0.to(self.prev_span()) });
            } else if self.eat_word("salvo") {
                let a = self.bump();
                let f = self.bump();
                if !(a.is_word("uma") && f.is_word("falha")) {
                    return err(Diagnostic::new(L0159, "Esperava \"salvo uma falha\".", a.span.to(f.span)));
                }
                fallible = true;
            } else {
                return Err(self.expected("\"sendo\", \"devolvendo\" ou \"salvo uma falha\""));
            }
        }
        // Na biblioteca, o cabeçalho que termina em ponto é uma operação da própria língua.
        let library = self.library;
        let builtin = match library {
            Some(lib) if self.eat_punct('.') => {
                Some(crate::builtins::Builtin::find(lib, &infinitive).expect("toda operação de biblioteca tem implementação"))
            }
            _ => {
                self.expect_punct(':', "os dois-pontos que fecham o cabeçalho")?;
                None
            }
        };
        // Objeto sem tipo vira complemento fixo: parte do nome do verbo.
        let mut final_params = Vec::new();
        let mut final_slots = Vec::new();
        let mut pi = params.iter();
        for slot in slots {
            match slot {
                Slot::Object => {
                    let (n, _) = pi.next().unwrap();
                    match typed.iter().find(|t| t.word == n.word) {
                        Some(t) => {
                            final_params.push(t.param(n));
                            final_slots.push(Slot::Object);
                        }
                        None if !n.is_symbol() => final_slots.push(Slot::Fixed {
                            article: morphology::definite_article(n.gender(), n.number()).to_string(),
                            word: n.word.clone(),
                        }),
                        None => return err(missing_type(n)),
                    }
                }
                Slot::Subject => {
                    let (n, _) = pi.next().unwrap();
                    let t = typed.iter().find(|t| t.word == n.word).ok_or_else(|| Box::new(missing_type(n)))?;
                    final_params.push(t.param(n));
                    final_slots.push(Slot::Subject);
                }
                Slot::Prep { prep, count } => {
                    for _ in 0..count {
                        let (n, _) = pi.next().unwrap();
                        let t = typed.iter().find(|t| t.word == n.word).ok_or_else(|| Box::new(missing_type(n)))?;
                        final_params.push(t.param(n));
                    }
                    final_slots.push(Slot::Prep { prep, count });
                }
                Slot::Fixed { .. } => unreachable!(),
            }
        }
        for t in &typed {
            if !params.iter().any(|(n, _)| n.word == t.word) {
                return err(Diagnostic::new(L0160, format!("\"{}\" foi tipado mas não aparece no cabeçalho.", t.word), t.span));
            }
        }
        if !matches!(kind, SigKind::Action) && ret.as_ref().is_some_and(|r| !matches!(r.ty, TypeExpr::Bool)) {
            return err(Diagnostic::new(L0161, "Um predicado (sujeito antes do verbo) devolve um lógico.", ret.unwrap().span)
                .hint("Ele vira condição: \"Se o saldo exceder o teto:\"."));
        }
        if !matches!(kind, SigKind::Action) && ret.is_none() {
            return err(Diagnostic::new(L0161, "Um predicado (sujeito antes do verbo) devolve um lógico.", name_span)
                .hint("Acrescente \", devolvendo um lógico\" ao cabeçalho."));
        }
        Ok(FuncSig {
            infinitive,
            first_person,
            first_person_span,
            kind,
            slots: final_slots,
            params: final_params,
            ret,
            fallible,
            span: start.to(self.prev_span()),
            name_span,
            is_main: false,
            broken: false,
            builtin,
        })
    }

    /// Um grupo de tipagem depois de "sendo": "x um real", "p e q inteiros",
    /// "ambas datas", "a carta um texto emprestado".
    fn typing(&mut self, params: &[(NameRef, Slot)]) -> Result<Vec<Typed>> {
        // (palavra, trecho do nome ou do "ambos", o nome se repetiu)
        let mut names: Vec<(String, Span, bool)> = Vec::new();
        let t = self.peek().clone();
        if t.is_word("ambos") || t.is_word("ambas") {
            self.bump();
            // Com mais de dois nomes, "ambos" são os regidos (o objeto pode ser complemento fixo).
            let pair: Vec<&NameRef> = if params.len() == 2 {
                params.iter().map(|(n, _)| n).collect()
            } else {
                params.iter().filter(|(_, s)| !matches!(s, Slot::Object)).map(|(n, _)| n).collect()
            };
            if pair.len() != 2 {
                return err(Diagnostic::new(L0162, "\"Ambos\" só cabe quando o cabeçalho tem dois parâmetros.", t.span));
            }
            let fem = pair.iter().all(|n| n.gender() == Gender::Feminine && !n.is_symbol());
            if t.is_word("ambas") != fem {
                let right = if fem { "ambas" } else { "ambos" };
                return err(Diagnostic::new(L0163, format!("\"{}\" não concorda com os parâmetros: escreva \"{right}\".", t.orig), t.span)
                    .fix(format!("Escrever \"{right}\""), t.span, right));
            }
            for n in pair {
                names.push((n.word.clone(), t.span, false));
            }
        } else {
            loop {
                let n = self.name_ref()?;
                let Some((decl, _)) = params.iter().find(|(p, _)| p.word == n.word) else {
                    return err(Diagnostic::new(L0160, format!("\"{}\" não aparece no cabeçalho.", n.display()), n.span));
                };
                if let (Some(a), Some(b)) = (n.article, decl.article)
                    && (a.gender != b.gender || a.number != b.number)
                {
                    return err(Diagnostic::new(
                        L0164,
                        format!("\"{}\" não concorda com o cabeçalho, que diz \"{}\".", n.display(), decl.display()),
                        n.span,
                    )
                    .fix(format!("Escrever \"{}\"", decl.display()), n.span, decl.display()));
                }
                names.push((n.word.clone(), n.span, true));
                // Mais nomes antes do tipo: "p e q inteiros".
                let more = (self.at_word("e") || self.at_punct(',')) && self.continues_group();
                if more {
                    self.bump();
                    self.eat_word("e");
                    continue;
                }
                break;
            }
        }
        let start = self.peek().span;
        let (ty, ty_gender, plural) = if let Some(art) = self.peek().word().and_then(morphology::article).filter(|a| !a.definite) {
            let at = self.bump();
            if names.len() > 1 {
                return err(Diagnostic::new(
                    L0165,
                    "Vários parâmetros pedem o tipo no plural, sem artigo: \"sendo p e q inteiros\".",
                    at.span,
                ));
            }
            let (ty, g) = self.type_word(Number::Singular)?;
            if g != art.gender {
                let w = self.toks[self.pos - 1].orig.clone();
                return err(Diagnostic::new(
                    L0128,
                    format!("\"{w}\" é {}: \"{} {w}\".", g.describe(), morphology::indefinite_article(g, Number::Singular)),
                    at.span,
                )
                .fix("Trocar o artigo", at.span, morphology::indefinite_article(g, Number::Singular)));
            }
            (ty, g, false)
        } else {
            if names.len() < 2 {
                return Err(self.expected("o tipo com artigo indefinido (\"um real\")"));
            }
            let (ty, g) = self.type_word(Number::Plural)?;
            (ty, g, true)
        };
        let number = if plural { Number::Plural } else { Number::Singular };
        let mut mode = ParamMode::Owned;
        if let Some(w) = self.peek().word().map(str::to_string) {
            for (base, m) in [("emprestado", ParamMode::Borrowed), ("confiado", ParamMode::Lent)] {
                if let Some((g, n)) = morphology::adjective_form(&w, base) {
                    let pt = self.bump();
                    if g != ty_gender || n != number {
                        let right = morphology::inflect_adjective(base, Gender::Masculine, ty_gender, number);
                        return err(Diagnostic::new(
                            L0167,
                            format!("O particípio concorda com o tipo: \"{right}\", não \"{}\".", pt.orig),
                            pt.span,
                        )
                        .fix(format!("Escrever \"{right}\""), pt.span, right.clone()));
                    }
                    mode = m;
                }
            }
        }
        let ann = TypeAnn { ty, span: start.to(self.prev_span()) };
        Ok(names.into_iter().map(|(word, span, named)| Typed { word, ann: ann.clone(), mode, span, named }).collect())
    }

    /// Depois de "e"/"," na tipagem: o nome seguinte é do mesmo grupo plural
    /// ("p e q inteiros") ou abre um grupo novo ("x um real e n um natural")?
    fn continues_group(&self) -> bool {
        let mut k = 1;
        if self.peek_at(k).is_word("e") {
            k += 1;
        }
        let t = self.peek_at(k);
        let len = if t.word().is_some_and(morphology::is_symbol) {
            1
        } else if t.word().and_then(morphology::article).is_some_and(|a| a.definite) {
            2
        } else {
            return false;
        };
        !self.peek_at(k + len).word().and_then(morphology::article).is_some_and(|a| !a.definite)
    }

    /// Lê um substantivo de tipo no número pedido. Devolve o tipo e o gênero.
    pub(super) fn type_word(&mut self, number: Number) -> Result<(TypeExpr, Gender)> {
        self.expect_here(|| super::Expect::Type { plural: number == Number::Plural });
        let t = self.bump();
        let w = t.word().unwrap_or_default().to_string();
        let sing = match number {
            Number::Singular => w.clone(),
            Number::Plural => match lexicon::type_noun_by_plural(&w) {
                Some(t) => t.singular.to_string(),
                None if self.vocab.record_plurals.contains_key(&w) || self.vocab.enum_plurals.contains_key(&w) => w.clone(),
                None => {
                    let plurals: Vec<&str> = lexicon::TYPE_NOUNS.iter().filter(|t| t.singular != "lista").map(|t| t.plural).collect();
                    return err(Diagnostic::new(L0168, format!("Tipo desconhecido: \"{}\".", t.orig), t.span)
                        .hint(format!("No plural: {}.", plurals.join(", "))));
                }
            },
        };
        if number == Number::Singular
            && (lexicon::type_noun_by_plural(&w).is_some()
                || self.vocab.record_plurals.contains_key(&w)
                || self.vocab.enum_plurals.contains_key(&w))
        {
            return err(Diagnostic::new(L0169, format!("Depois do artigo singular, o tipo vai no singular: não \"{w}\"."), t.span));
        }
        if sing == "lista" {
            if number == Number::Plural && w != "listas" {
                unreachable!();
            }
            self.expect_word("de")?;
            let (elem, _) = self.type_word(Number::Plural)?;
            return Ok((TypeExpr::List(Box::new(elem)), Gender::Feminine));
        }
        if let Some((name, g)) = self.record_type_word(&w, number) {
            self.prog.type_uses.push(TypeUse { word: name.clone(), span: t.span, number });
            return Ok((TypeExpr::Named(name), g));
        }
        let ty = match sing.as_str() {
            "inteiro" | "natural" => TypeExpr::Int { bits: self.bits()?.unwrap_or(64), signed: sing == "inteiro" },
            "real" => {
                let bits = self.bits()?.unwrap_or(64);
                if bits != 64 && bits != 32 {
                    return err(Diagnostic::new(L0170, "Um real tem 32 ou 64 bits.", self.prev_span()));
                }
                TypeExpr::Real { bits }
            }
            "byte" => TypeExpr::Byte,
            "lógico" => TypeExpr::Bool,
            "caractere" => TypeExpr::Char,
            "texto" => TypeExpr::Text,
            _ => {
                let singulars: Vec<&str> = lexicon::TYPE_NOUNS.iter().filter(|t| t.singular != "lista").map(|t| t.singular).collect();
                let mut d = Diagnostic::new(L0168, format!("Tipo desconhecido: \"{}\".", t.orig), t.span).hint(format!(
                    "Tipos: {}, lista de ..., ou um registro definido com \"Uma conta é um registro de:\".",
                    singulars.join(", ")
                ));
                if let Some(fix) = ["lógico", "lógicos"].iter().find(|c| morphology::strip_accents(c) == w) {
                    d = d.hint(format!("Faltou acento? Escreva \"{fix}\"."));
                }
                return err(d);
            }
        };
        Ok((ty, Gender::Masculine))
    }

    // ------------------------------------------------------------- chamadas

    /// Lê os argumentos de uma chamada, guiado pela regência da assinatura. `skip_subject`
    /// quando o sujeito já foi lido (predicados).
    pub(super) fn call_args(&mut self, sig_id: usize, skip_subject: bool, verb_span: Span) -> Result<Vec<Expr>> {
        let sig = self.prog.sigs[sig_id].clone();
        let mut args = Vec::new();
        if sig.broken {
            // O cabeçalho já foi acusado; sem a regência, a chamada vai até o fim da oração.
            while !matches!(self.peek().kind, TokenKind::Punct('.' | ':') | TokenKind::Newline | TokenKind::Dedent | TokenKind::Eof) {
                self.bump();
            }
            return Ok(args);
        }
        let mut objects = 0;
        for slot in &sig.slots {
            match slot {
                Slot::Subject => {
                    if !skip_subject {
                        args.push(self.additive()?);
                    }
                }
                Slot::Object => {
                    self.expect_here(|| super::Expect::Value);
                    if objects > 0 && !(self.eat_word("e") || self.eat_punct(',')) {
                        return Err(self.regency_error(&sig, "faltou um objeto"));
                    }
                    if !self.starts_value(0) {
                        return Err(self.regency_error(&sig, "faltou um objeto"));
                    }
                    args.push(self.additive()?);
                    objects += 1;
                }
                Slot::Fixed { article, word } => {
                    let ok = self.peek().is_word(article) && self.peek_at(1).is_word(word);
                    if !ok {
                        return Err(self.regency_error(&sig, &format!("\"{article} {word}\" faz parte do verbo")));
                    }
                    self.bump();
                    self.bump();
                }
                Slot::Prep { prep, count } => {
                    if !self.at_word(prep) {
                        return Err(self.regency_error(&sig, &format!("aqui vem \"{prep}\"")));
                    }
                    self.bump();
                    args.push(self.additive()?);
                    if *count == 2 {
                        if !self.eat_word("e") {
                            return Err(self.regency_error(&sig, &format!("\"{prep}\" rege dois termos ligados por \"e\"")));
                        }
                        args.push(self.additive()?);
                    }
                }
            }
        }
        let _ = verb_span;
        Ok(args)
    }

    fn regency_error(&self, sig: &FuncSig, what: &str) -> Box<Diagnostic> {
        let t = self.peek();
        Box::new(
            Diagnostic::new(L0171, format!("A chamada não segue a regência de \"{}\": {what}.", sig.infinitive), t.span)
                .primary(format!("encontrei {}", t.describe()))
                .label(sig.name_span, "definido aqui")
                .hint(format!("Escreva: {}", sig.usage())),
        )
    }
}

fn missing_type(n: &NameRef) -> Diagnostic {
    Diagnostic::new(L0172, format!("Falta o tipo de \"{}\".", n.display()), n.span)
        .hint(format!("Depois do cabeçalho: \", sendo {} um inteiro\".", n.display()))
}

/// O que se leu de um cabeçalho até o erro, para a assinatura quebrada: com o verbo, a
/// função existe, as chamadas a ela se reconhecem e os parâmetros lidos são nomes no
/// corpo.
#[derive(Default)]
pub(super) struct Draft {
    start: Option<Span>,
    infinitive: Option<String>,
    first_person: Option<String>,
    first_person_span: Option<Span>,
    kind: Option<SigKind>,
    name_span: Span,
    params: Vec<NameRef>,
}

/// Um nome da tipagem do cabeçalho: o tipo e o modo que ele ganhou, e onde foi tipado
/// (o próprio nome, ou o "ambos" que o inclui).
struct Typed {
    word: String,
    ann: TypeAnn,
    mode: ParamMode,
    span: Span,
    named: bool,
}

impl Typed {
    fn param(&self, name: &NameRef) -> Param {
        Param { name: name.clone(), ty: self.ann.clone(), mode: self.mode, typing: self.named.then_some(self.span) }
    }
}

impl Draft {
    fn into_sig(self, end: Span) -> Option<FuncSig> {
        let infinitive = self.infinitive?;
        let kind = self.kind.unwrap_or(SigKind::Action);
        let first_person = self.first_person.or_else(|| morphology::first_person(&infinitive)).unwrap_or_else(|| infinitive.clone());
        // Tipo de erro, e confiados: o corpo pode ler e mudar sem gerar mais erros.
        let params = self
            .params
            .into_iter()
            .map(|n| {
                let span = n.span;
                Param { name: n, ty: TypeAnn { ty: TypeExpr::Error, span }, mode: ParamMode::Lent, typing: None }
            })
            .collect();
        Some(FuncSig {
            infinitive,
            first_person,
            first_person_span: self.first_person_span,
            kind,
            slots: Vec::new(),
            params,
            ret: Some(TypeAnn { ty: TypeExpr::Error, span: self.name_span }),
            fallible: true,
            span: self.start?.to(end),
            name_span: self.name_span,
            is_main: false,
            broken: true,
            builtin: None,
        })
    }
}
