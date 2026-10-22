//! Análise sintática por descida recursiva. Confere a gramática local (contrações,
//! concordância verbal, ordem das palavras); escopo, tipos e posse ficam na `sema`.

use crate::ast::*;
use crate::diagnostics::{Category, Diagnostic, Result, err};
use crate::lexer::{Token, TokenKind};
use crate::lexicon;
use crate::morphology::{self, Gender, Number};
use crate::span::Span;

mod data;
mod header;

use std::collections::HashMap;

pub fn parse(tokens: Vec<Token>) -> Result<Program> {
    let mut p = Parser {
        toks: tokens,
        pos: 0,
        prog: Program::default(),
        headers: HashMap::new(),
        vocab: Vocabulary::default(),
        allow_uncontracted: false,
        skip_spans: HashMap::new(),
        last_nominal: false,
    };
    p.prescan_records()?;
    p.prescan()?;
    p.program()?;
    p.prog.vocab = p.vocab;
    Ok(p.prog)
}

struct Parser {
    toks: Vec<Token>,
    pos: usize,
    prog: Program,
    /// Cabeçalhos lidos na pré-varredura: posição do "Para" → (assinatura, fim do cabeçalho).
    headers: HashMap<usize, (usize, usize)>,
    /// Os nomes do programa, das pré-varreduras; vai para `Program::vocab` no fim.
    vocab: Vocabulary,
    /// "o fato de o valor ser ...": antes do sujeito de infinitivo não se contrai.
    allow_uncontracted: bool,
    /// Definições já lidas na pré-varredura: início → fim.
    skip_spans: HashMap<usize, usize>,
    /// O último termo lido foi uma operação por extenso (para a regra dos parênteses).
    last_nominal: bool,
}

impl Parser {
    // ------------------------------------------------------------------ tokens

    fn peek(&self) -> &Token {
        &self.toks[self.pos]
    }

    fn peek_at(&self, k: usize) -> &Token {
        &self.toks[(self.pos + k).min(self.toks.len() - 1)]
    }

    fn bump(&mut self) -> Token {
        let t = self.toks[self.pos].clone();
        if self.pos < self.toks.len() - 1 {
            self.pos += 1;
        }
        t
    }

    fn prev_span(&self) -> Span {
        self.toks[self.pos.saturating_sub(1)].span
    }

    /// O último caractere do token anterior, inteiro mesmo quando acentuado.
    fn last_char_span(&self) -> Span {
        let t = &self.toks[self.pos.saturating_sub(1)];
        let end = t.span.end as usize;
        let width = t.orig.chars().last().map_or(1, char::len_utf8);
        Span::new(end.saturating_sub(width).max(t.span.start as usize), end)
    }

    fn at_word(&self, w: &str) -> bool {
        self.peek().is_word(w)
    }

    fn eat_word(&mut self, w: &str) -> bool {
        if self.at_word(w) {
            self.bump();
            true
        } else {
            false
        }
    }

    fn at_punct(&self, c: char) -> bool {
        self.peek().is_punct(c)
    }

    fn eat_punct(&mut self, c: char) -> bool {
        if self.at_punct(c) {
            self.bump();
            true
        } else {
            false
        }
    }

    /// Erro de "esperava X, encontrei Y", com o caso especial do ponto final esquecido.
    fn expected(&self, what: &str) -> Box<Diagnostic> {
        let t = self.peek();
        if what == "o ponto final" && (matches!(t.kind, TokenKind::Newline | TokenKind::Eof | TokenKind::Dedent) || t.line_start) {
            return Box::new(
                Diagnostic::new(Category::Syntax, "Faltou o ponto final.", self.last_char_span())
                    .hint("Todo período termina em ponto; uma linha sem ponto continua na de baixo."),
            );
        }
        Box::new(Diagnostic::new(Category::Syntax, format!("Esperava {what}, mas encontrei {}.", t.describe()), t.span))
    }

    fn expect_punct(&mut self, c: char, what: &str) -> Result<Token> {
        if self.at_punct(c) { Ok(self.bump()) } else { Err(self.expected(what)) }
    }

    fn expect_word(&mut self, w: &str) -> Result<Token> {
        if self.at_word(w) { Ok(self.bump()) } else { Err(self.expected(&format!("\"{w}\""))) }
    }

    fn skip_newlines(&mut self) {
        while matches!(self.peek().kind, TokenKind::Newline) {
            self.bump();
        }
    }

    /// Artigo definido (não a preposição de uma contração).
    fn definite_article_at(&self, k: usize) -> Option<morphology::Article> {
        let t = self.peek_at(k);
        if t.contraction.is_some_and(|c| c.part == 1) {
            return None;
        }
        t.word().and_then(morphology::article).filter(|a| a.definite)
    }

    // ----------------------------------------------------------------- programa

    fn program(&mut self) -> Result<()> {
        self.skip_newlines();
        while !matches!(self.peek().kind, TokenKind::Eof) {
            if self.at_word("para") && self.peek_at(1).is_word("começar") {
                let start = self.bump().span;
                let name_span = self.bump().span;
                self.expect_punct(':', "os dois-pontos depois de \"Para começar\"")?;
                if self.prog.main.is_some() {
                    return err(Diagnostic::new(Category::Syntax, "O programa já tem um \"Para começar\".", start.to(name_span)));
                }
                let sig = self.prog.sigs.len();
                self.prog.sigs.push(FuncSig {
                    infinitive: "começar".into(),
                    first_person: "começo".into(),
                    kind: SigKind::Action,
                    slots: vec![],
                    params: vec![],
                    ret: None,
                    fallible: true,
                    span: start.to(name_span),
                    name_span,
                    is_main: true,
                });
                let body = self.block()?;
                self.prog.funcs.push(FuncDef { sig, body });
                self.prog.main = Some(sig);
            } else if let Some(&end) = self.skip_spans.get(&self.pos) {
                self.pos = end;
            } else if let Some(&(sig, end)) = self.headers.get(&self.pos) {
                self.pos = end;
                let body = self.block()?;
                self.prog.funcs.push(FuncDef { sig, body });
            } else {
                let s = self.stmt()?;
                self.prog.top.push(s);
            }
            self.skip_newlines();
        }
        Ok(())
    }

    /// Bloco depois de dois-pontos: recuado na linha de baixo, ou um período na mesma linha.
    fn block(&mut self) -> Result<Block> {
        if matches!(self.peek().kind, TokenKind::Newline) {
            self.skip_newlines();
            if !matches!(self.peek().kind, TokenKind::Indent) {
                return Err(Box::new(
                    Diagnostic::new(Category::Syntax, "Depois dos dois-pontos vem um bloco recuado.", self.peek().span)
                        .hint("Recue as linhas do bloco (quatro espaços), ou escreva um período na mesma linha."),
                ));
            }
            self.bump();
            let mut stmts = Vec::new();
            self.skip_newlines();
            while !matches!(self.peek().kind, TokenKind::Dedent | TokenKind::Eof) {
                stmts.push(self.stmt()?);
                self.skip_newlines();
            }
            if matches!(self.peek().kind, TokenKind::Dedent) {
                self.bump();
            }
            Ok(Block { stmts })
        } else {
            let p = self.period()?;
            Ok(Block { stmts: vec![Stmt::Period(p)] })
        }
    }

    fn stmt(&mut self) -> Result<Stmt> {
        let start = self.peek().span;
        if self.at_word("se") {
            return self.if_stmt();
        }
        if self.eat_word("enquanto") {
            let cond = self.cond()?;
            self.expect_punct(':', "os dois-pontos depois da condição")?;
            let body = self.block()?;
            return Ok(Stmt::While { cond, body, span: start.to(self.prev_span()) });
        }
        if self.eat_word("repito") {
            let count = if self.eat_word("sempre") {
                None
            } else {
                let n = self.expr()?;
                if !self.eat_word("vezes") {
                    if self.at_word("vez") {
                        let t = self.peek().clone();
                        return err(Diagnostic::new(Category::Agreement, "\"Repito\" conta em \"vezes\", no plural.", t.span)
                            .hint("Para uma vez só, não precisa de laço."));
                    }
                    return Err(self.expected("\"vezes\" (ou \"Repito sempre:\")"));
                }
                Some(n)
            };
            self.expect_punct(':', "os dois-pontos")?;
            let body = self.block()?;
            return Ok(Stmt::Repeat { count, body, span: start.to(self.prev_span()) });
        }
        if self.at_word("para") && self.peek_at(1).is_word("cada") {
            self.bump();
            self.bump();
            let var = self.foreach_var()?;
            let has_until = {
                let mut k = 0;
                loop {
                    let t = self.peek_at(k);
                    if t.is_word("até") {
                        break true;
                    }
                    if matches!(t.kind, TokenKind::Punct(':') | TokenKind::Newline | TokenKind::Eof) {
                        break false;
                    }
                    k += 1;
                }
            };
            if !has_until {
                let source = if self.eat_word("de") {
                    self.primary()?
                } else if var.article.is_none() {
                    return Err(self.expected("\"de\" com a coleção ou o intervalo (\"Para cada i de 1 até 10:\")"));
                } else {
                    // "Para cada nota:" acha "as notas" pelo plural.
                    let plural = morphology::pluralize(&var.word);
                    if plural == var.word {
                        return err(Diagnostic::new(
                            Category::Morphology,
                            format!("\"{}\" é igual no singular e no plural: diga de onde vêm os itens.", var.word),
                            var.span,
                        )
                        .hint(format!(
                            "Escreva \"Para cada {} {} ...:\".",
                            var.word,
                            morphology::join_prep("de", morphology::definite_article(var.gender(), Number::Plural))
                        )));
                    }
                    let a = var.article.expect("o nome sem símbolo tem artigo");
                    let name = NameRef {
                        word: plural,
                        span: var.span,
                        word_span: var.word_span,
                        article: Some(ArticleUse { gender: a.gender, number: Number::Plural, span: a.span, contraction: None }),
                    };
                    let place = Place { span: var.span, root: name, proj: vec![] };
                    Expr { span: var.span, kind: ExprKind::Place(place) }
                };
                self.expect_punct(':', "os dois-pontos")?;
                let body = self.block()?;
                return Ok(Stmt::ForEach { item: var, source, body, span: start.to(self.prev_span()) });
            }
            if self.eat_word("de") {
                let from = self.expr()?;
                self.expect_word("até")?;
                let to = self.expr()?;
                self.expect_punct(':', "os dois-pontos")?;
                let body = self.block()?;
                return Ok(Stmt::ForRange { var, from, to, body, span: start.to(self.prev_span()) });
            }
            return Err(self.expected("\"de\" com o intervalo (\"Para cada i de 1 até 10:\")"));
        }
        if self.eat_word("conforme") {
            return self.match_stmt(start);
        }
        if self.at_word("senão") {
            return err(Diagnostic::new(Category::Syntax, "\"Senão\" sem um \"Se\" antes.", self.peek().span)
                .hint("O \"Senão\" fica alinhado com o \"Se\" a que responde."));
        }
        Ok(Stmt::Period(self.period()?))
    }

    /// A variável de "Para cada": símbolo ("i") ou substantivo sem artigo ("nota"); o
    /// gênero sai da palavra e é conferido contra a coleção na análise.
    fn foreach_var(&mut self) -> Result<NameRef> {
        let t = self.bump();
        let Some(w) = t.word().map(str::to_string) else {
            return err(Diagnostic::new(
                Category::Syntax,
                format!("Depois de \"Para cada\" vem o nome do item, mas encontrei {}.", t.describe()),
                t.span,
            ));
        };
        if morphology::is_symbol(&w) {
            return Ok(NameRef { word: w, span: t.span, word_span: t.span, article: None });
        }
        if morphology::article(&w).is_some() {
            return err(Diagnostic::new(Category::Syntax, "\"Cada\" já determina o nome: \"Para cada nota\", sem artigo.", t.span));
        }
        let g = morphology::guess_gender(&w);
        Ok(NameRef {
            word: w,
            span: t.span,
            word_span: t.span,
            article: Some(ArticleUse { gender: g, number: Number::Singular, span: t.span, contraction: None }),
        })
    }

    /// "Conforme a figura:" seguido de casos "Sendo um círculo:" e, opcional, "Senão:".
    fn match_stmt(&mut self, start: Span) -> Result<Stmt> {
        let scrutinee = self.expr()?;
        self.expect_punct(':', "os dois-pontos depois de \"Conforme ...\"")?;
        self.skip_newlines();
        if !matches!(self.peek().kind, TokenKind::Indent) {
            return Err(self.expected("os casos recuados (\"Sendo um círculo: ...\")"));
        }
        self.bump();
        let mut arms = Vec::new();
        let mut otherwise = None;
        self.skip_newlines();
        while !matches!(self.peek().kind, TokenKind::Dedent | TokenKind::Eof) {
            let t = self.peek().clone();
            if self.eat_word("senão") {
                self.expect_punct(':', "os dois-pontos depois de \"Senão\"")?;
                otherwise = Some(self.block()?);
                self.skip_newlines();
                break;
            }
            if !self.eat_word("sendo") {
                return err(Diagnostic::new(
                    Category::Syntax,
                    format!("Dentro de \"Conforme\", cada caso começa com \"Sendo\", mas encontrei {}.", t.describe()),
                    t.span,
                )
                .hint("Escreva \"Sendo um círculo: ...\", ou \"Senão: ...\" para o resto."));
            }
            let mut patterns = vec![self.variant_pattern()?];
            while self.eat_word("ou") {
                patterns.push(self.variant_pattern()?);
            }
            self.expect_punct(':', "os dois-pontos depois do caso")?;
            let body = self.block()?;
            arms.push(MatchArm { patterns, body, span: t.span.to(self.prev_span()) });
            self.skip_newlines();
        }
        if matches!(self.peek().kind, TokenKind::Dedent) {
            self.bump();
        }
        Ok(Stmt::Match { scrutinee, arms, otherwise, span: start.to(self.prev_span()) })
    }

    fn if_stmt(&mut self) -> Result<Stmt> {
        let start = self.expect_word("se")?.span;
        let mut arms = Vec::new();
        let cond = self.cond()?;
        self.expect_punct(':', "os dois-pontos depois da condição")?;
        arms.push((cond, self.block()?));
        let mut otherwise = None;
        loop {
            let save = self.pos;
            self.skip_newlines();
            if !self.eat_word("senão") {
                self.pos = save;
                break;
            }
            if self.eat_punct(',') {
                self.expect_word("se")?;
                let cond = self.cond()?;
                self.expect_punct(':', "os dois-pontos depois da condição")?;
                arms.push((cond, self.block()?));
                continue;
            }
            if self.at_word("se") {
                return err(Diagnostic::new(Category::Syntax, "Entre \"senão\" e \"se\" vai vírgula.", self.peek().span)
                    .hint("Escreva \"Senão, se ...:\"."));
            }
            self.expect_punct(':', "os dois-pontos depois de \"Senão\"")?;
            otherwise = Some(self.block()?);
            break;
        }
        Ok(Stmt::If { arms, otherwise, span: start.to(self.prev_span()) })
    }

    // ---------------------------------------------------------------- condições

    fn cond(&mut self) -> Result<Cond> {
        self.cond_mood(Mood::Subjunctive)
    }

    fn cond_mood(&mut self, mood: Mood) -> Result<Cond> {
        let mut ors = vec![self.cond_and(mood)?];
        while self.eat_word("ou") {
            ors.push(self.cond_and(mood)?);
        }
        Ok(if ors.len() == 1 { ors.pop().unwrap() } else { Cond::Or(ors) })
    }

    fn cond_and(&mut self, mood: Mood) -> Result<Cond> {
        let mut ands = vec![self.cond_atom(mood)?];
        while self.eat_word("e") {
            ands.push(self.cond_atom(mood)?);
        }
        Ok(if ands.len() == 1 { ands.pop().unwrap() } else { Cond::And(ands) })
    }

    /// Forma de "ser" no modo e número pedidos: "for"/"forem" ou "ser"/"serem".
    fn be_form(mood: Mood, n: Number) -> String {
        match mood {
            Mood::Subjunctive => morphology::future_subjunctive("ser", n),
            Mood::Infinitive => morphology::personal_infinitive("ser", n),
        }
    }

    fn verb_form(mood: Mood, inf: &str, n: Number) -> String {
        match mood {
            Mood::Subjunctive => morphology::future_subjunctive(inf, n),
            Mood::Infinitive => morphology::personal_infinitive(inf, n),
        }
    }

    fn cond_atom(&mut self, mood: Mood) -> Result<Cond> {
        let start = self.peek().span;
        // "Se houver apelido", "Se não houver apelido".
        let neg_exists = self.at_word("não") && self.peek_at(1).is_word("houver");
        if self.at_word("houver") || neg_exists {
            if neg_exists {
                self.bump();
            }
            self.bump();
            let t = self.bump();
            let Some(word) = t.word().map(str::to_string) else {
                return Err(Box::new(Diagnostic::new(
                    Category::Syntax,
                    format!("Depois de \"houver\" vem o nome, sem artigo: \"Se houver apelido\". Encontrei {}.", t.describe()),
                    t.span,
                )));
            };
            if morphology::article(&word).is_some() {
                return err(Diagnostic::new(
                    Category::Syntax,
                    "Depois de \"houver\" o nome vem sem artigo: \"Se houver apelido\".",
                    t.span,
                ));
            }
            return Ok(Cond::Atom {
                negated: neg_exists,
                kind: Box::new(CondKind::Exists { word, word_span: t.span }),
                span: start.to(t.span),
            });
        }
        if mood == Mood::Subjunctive && self.at_word("há") {
            let t = self.peek().clone();
            return err(Diagnostic::new(
                Category::Morphology,
                "Em condição, o verbo vai no futuro do subjuntivo: \"houver\", não \"há\".",
                t.span,
            ));
        }
        let subject = self.expr()?;
        self.allow_uncontracted = false;
        let number = expr_number(&subject);
        let gender = expr_gender(&subject);
        let negated = self.eat_word("não");
        let vt = self.peek().clone();
        let w = vt.word().unwrap_or_default().to_string();
        // Predicado definido pelo usuário: "Se o saldo exceder o teto".
        for n in [Number::Singular, Number::Plural] {
            let found = self.prog.sigs.iter().position(|s| s.kind == SigKind::Predicate && Self::verb_form(mood, &s.infinitive, n) == w);
            if let Some(sig) = found {
                if n != number {
                    let right = Self::verb_form(mood, &self.prog.sigs[sig].infinitive, number);
                    return err(Diagnostic::new(
                        Category::Agreement,
                        format!("O verbo não concorda com o sujeito, que é {}: \"{right}\", não \"{w}\".", number.describe()),
                        vt.span,
                    ));
                }
                self.bump();
                let mut args = vec![subject];
                args.extend(self.call_args(sig, true, vt.span)?);
                let call = Call { sig, args, span: start.to(self.prev_span()), verb_span: vt.span };
                return Ok(Cond::Atom { negated, kind: Box::new(CondKind::Pred { call }), span: start.to(self.prev_span()) });
            }
        }
        if mood == Mood::Subjunctive
            && let Some(sig) = self.prog.sigs.iter().position(|s| s.kind == SigKind::Predicate && s.infinitive == w)
        {
            let right = Self::verb_form(mood, &self.prog.sigs[sig].infinitive, number);
            return err(Diagnostic::new(
                Category::Morphology,
                format!("Em condição, o predicado vai no futuro do subjuntivo: \"{right}\", não \"{w}\"."),
                vt.span,
            ));
        }
        let be_sing = Self::be_form(mood, Number::Singular);
        let be_plur = Self::be_form(mood, Number::Plural);
        if w == be_sing || w == be_plur {
            self.bump();
            let got = if w == be_sing { Number::Singular } else { Number::Plural };
            if got != number {
                let right = Self::be_form(mood, number);
                return err(Diagnostic::new(
                    Category::Agreement,
                    format!("O verbo não concorda com o sujeito, que é {}: \"{right}\", não \"{w}\".", number.describe()),
                    vt.span,
                ));
            }
            // Caso de enumeração: "Se a figura for um círculo".
            if self.peek().word().and_then(morphology::article).is_some_and(|a| !a.definite)
                && self.peek_at(1).word().is_some_and(|w| self.vocab.variants.contains_key(w))
            {
                let Pattern::Variant { word, gender, span } = self.variant_pattern()? else { unreachable!() };
                return Ok(Cond::Atom {
                    negated,
                    kind: Box::new(CondKind::Variant { subject, word, gender, word_span: span }),
                    span: start.to(span),
                });
            }
            // Predicado adjetivo: "Se o número for par".
            if let Some(adj_tok) = self.peek().word().map(str::to_string) {
                for (sig, s) in self.prog.sigs.iter().enumerate() {
                    let SigKind::Adjective { adjective, base_gender } = &s.kind else { continue };
                    let want = morphology::inflect_adjective(adjective, *base_gender, gender, number);
                    if adj_tok == want {
                        let at = self.bump();
                        let mut args = vec![subject];
                        args.extend(self.call_args(sig, true, at.span)?);
                        let call = Call { sig, args, span: start.to(self.prev_span()), verb_span: at.span };
                        return Ok(Cond::Atom { negated, kind: Box::new(CondKind::Pred { call }), span: start.to(self.prev_span()) });
                    }
                    let any = [Gender::Masculine, Gender::Feminine]
                        .iter()
                        .flat_map(|g| {
                            [Number::Singular, Number::Plural].map(|n| morphology::inflect_adjective(adjective, *base_gender, *g, n))
                        })
                        .any(|f| f == adj_tok);
                    if any {
                        let at = self.peek().clone();
                        return err(Diagnostic::new(
                            Category::Agreement,
                            format!(
                                "O adjetivo concorda com o sujeito ({}): \"{want}\", não \"{}\".",
                                morphology::describe_gn(gender, number),
                                at.orig
                            ),
                            at.span,
                        ));
                    }
                }
            }
            let rt = self.peek().clone();
            let plural_rel = |sing: &str, plur: &str, rel: Rel, me: &Self| -> Result<Rel> {
                let got = if rt.is_word(plur) { Number::Plural } else { Number::Singular };
                if got != number {
                    let right = if number == Number::Plural { plur } else { sing };
                    let _ = me;
                    return err(Diagnostic::new(
                        Category::Agreement,
                        format!(
                            "A comparação concorda com o sujeito, que é {}: \"{right}\", não \"{}\".",
                            number.describe(),
                            rt.word().unwrap()
                        ),
                        rt.span,
                    ));
                }
                Ok(rel)
            };
            let rel = match rt.word() {
                Some("maior que" | "maiores que") => plural_rel("maior que", "maiores que", Rel::Greater, self)?,
                Some("menor que" | "menores que") => plural_rel("menor que", "menores que", Rel::Less, self)?,
                Some("igual a" | "iguais a") => plural_rel("igual a", "iguais a", Rel::Equal, self)?,
                Some("diferente de" | "diferentes de") => plural_rel("diferente de", "diferentes de", Rel::Different, self)?,
                Some("pelo menos") => Rel::AtLeast,
                Some("no máximo") => Rel::AtMost,
                Some("nada") => {
                    self.bump();
                    let rhs = Expr { kind: ExprKind::Nothing, span: rt.span };
                    return Ok(Cond::Atom {
                        negated,
                        kind: Box::new(CondKind::Rel { subject, rel: Rel::Equal, rhs }),
                        span: start.to(rt.span),
                    });
                }
                Some("verdadeiro" | "falso") => {
                    self.bump();
                    let rhs = Expr { kind: ExprKind::Bool(rt.is_word("verdadeiro")), span: rt.span };
                    return Ok(Cond::Atom {
                        negated,
                        kind: Box::new(CondKind::Rel { subject, rel: Rel::Equal, rhs }),
                        span: start.to(rt.span),
                    });
                }
                Some("maior" | "menor" | "igual" | "diferente") => {
                    let need = match rt.word().unwrap() {
                        "maior" | "menor" => "que",
                        "igual" => "a",
                        _ => "de",
                    };
                    return err(Diagnostic::new(
                        Category::Regency,
                        format!("\"{}\" rege \"{need}\": \"{} {need}\".", rt.orig, rt.orig),
                        rt.span,
                    ));
                }
                _ => {
                    return Err(self.expected(
                        "a comparação (\"maior que\", \"menor que\", \"igual a\", \"diferente de\", \"pelo menos\", \"no máximo\")",
                    ));
                }
            };
            self.bump();
            let rhs = self.expr()?;
            return Ok(Cond::Atom { negated, kind: Box::new(CondKind::Rel { subject, rel, rhs }), span: start.to(self.prev_span()) });
        }
        if mood == Mood::Subjunctive && matches!(w.as_str(), "é" | "são" | "seja" | "sejam" | "fosse" | "fossem" | "está" | "estão") {
            let right = morphology::future_subjunctive("ser", number);
            return err(Diagnostic::new(
                Category::Morphology,
                format!("Em condição, o verbo vai no futuro do subjuntivo: \"{right}\", não \"{}\".", vt.orig),
                vt.span,
            ));
        }
        if mood == Mood::Infinitive {
            return Err(self.expected("o verbo no infinitivo (\"ser\", \"serem\", ou um predicado)"));
        }
        Err(self.expected("o verbo da condição no futuro do subjuntivo (\"for\", \"exceder\"...)"))
    }

    // ------------------------------------------------------------------ período

    fn period(&mut self) -> Result<Period> {
        let start = self.peek().span;
        let mut clauses = vec![self.clause()?];
        loop {
            if self.at_punct('.') {
                break;
            }
            let save = self.pos;
            let mut coord = false;
            if self.eat_punct(',') {
                coord = true;
                self.eat_word("e");
            } else if self.eat_word("e") {
                coord = true;
            }
            if coord && self.is_clause_start() {
                clauses.push(self.clause()?);
                continue;
            }
            self.pos = save;
            if self.at_word("de")
                && self.pos >= 2
                && let Some(prev) = self.toks[self.pos - 1].word()
                && !self.vocab.field_names.contains(prev)
                && self.toks[self.pos - 2].word().and_then(morphology::article).is_some_and(|a| a.definite)
            {
                let t = &self.toks[self.pos - 1];
                let mut d = Diagnostic::new(Category::Scope, format!("Nenhum registro tem o campo \"{prev}\"."), t.span);
                if !self.vocab.field_names.is_empty() {
                    let mut names: Vec<&String> = self.vocab.field_names.iter().collect();
                    names.sort();
                    let near =
                        names.iter().min_by_key(|n| morphology::edit_distance(n, prev)).filter(|n| morphology::edit_distance(n, prev) <= 2);
                    d = match near {
                        Some(n) => d.hint(format!("Quis dizer \"{n}\"?")),
                        None => d.hint(format!(
                            "Campos conhecidos: {}.",
                            names.iter().map(|n| format!("\"{n}\"")).collect::<Vec<_>>().join(", ")
                        )),
                    };
                }
                return Err(Box::new(d));
            }
            return Err(self.expected("o ponto final"));
        }
        let end = self.expect_punct('.', "o ponto final")?.span;
        // "Escrevo X e leio Y": a escrita vira pergunta, sem quebra de linha.
        for i in 0..clauses.len().saturating_sub(1) {
            let next_reads = matches!(clauses[i + 1].kind, ClauseKind::ReadInto { .. } | ClauseKind::ReadValue { .. });
            if let ClauseKind::Write { newline, .. } = &mut clauses[i].kind
                && next_reads
            {
                *newline = false;
            }
        }
        Ok(Period { clauses, span: start.to(end) })
    }

    fn is_core_verb(w: &str) -> bool {
        lexicon::is_clause_verb(w)
    }

    /// Depois de "e" ou vírgula: começa outra oração?
    fn is_clause_start(&self) -> bool {
        let t = self.peek();
        if let Some(w) = t.word()
            && (Self::is_core_verb(w) || self.vocab.by_first_person.contains_key(w))
        {
            return true;
        }
        if self.definite_article_at(0).is_none() && !t.word().is_some_and(morphology::is_symbol) {
            return false;
        }
        // Sujeito seguido de verbo de dado-sujeito antes de qualquer pontuação.
        let mut k = 0;
        let mut depth = 0i32;
        loop {
            let t = self.peek_at(k);
            match &t.kind {
                TokenKind::Punct('(') => depth += 1,
                TokenKind::Punct(')') => depth -= 1,
                // Parêntese sem par: a linha lógica acaba e a busca também.
                TokenKind::Newline | TokenKind::Eof | TokenKind::Indent | TokenKind::Dedent if depth > 0 => return false,
                TokenKind::Punct(_) | TokenKind::Newline | TokenKind::Eof | TokenKind::Indent | TokenKind::Dedent if depth <= 0 => {
                    // "O limite, um inteiro, é 100": aposto.
                    return t.is_punct(',')
                        && k == 2
                        && self.peek_at(3).word().is_some_and(|w| morphology::article(w).is_some_and(|a| !a.definite) || w == "talvez");
                }
                TokenKind::Word(w) if depth <= 0 && subject_verb(w).is_some() => return true,
                _ => {}
            }
            k += 1;
        }
    }

    fn clause(&mut self) -> Result<Clause> {
        let t = self.peek().clone();
        let start = t.span;
        let Some(w) = t.word().map(str::to_string) else {
            return Err(self.expected("uma oração (um verbo na 1ª pessoa, ou um sujeito)"));
        };
        let kind = match w.as_str() {
            "escrevo" => {
                self.bump();
                let value = self.expr()?;
                let mut newline = true;
                if self.at_word("sem") && self.peek_at(1).is_word("quebrar") {
                    self.bump();
                    self.bump();
                    self.expect_word("a")?;
                    self.expect_word("linha")?;
                    newline = false;
                }
                ClauseKind::Write { parts: vec![value], newline }
            }
            "leio" => {
                self.bump();
                if let Some(art) = self.peek().word().and_then(morphology::article).filter(|a| !a.definite) {
                    let at = self.bump();
                    let s0 = at.span;
                    let ty = self.type_noun(art, at.span)?;
                    ClauseKind::ReadValue { ty: TypeAnn { ty, span: s0.to(self.prev_span()) } }
                } else {
                    ClauseKind::ReadInto { place: self.place()? }
                }
            }
            "devolvo" => {
                self.bump();
                if self.starts_value(0) { ClauseKind::Return(Some(self.expr()?)) } else { ClauseKind::Return(None) }
            }
            "paro" => {
                self.bump();
                ClauseKind::Break
            }
            "desisto" => {
                self.bump();
                if !self.eat_word("com") {
                    return err(Diagnostic::new(
                        Category::Syntax,
                        "\"Desisto\" diz o motivo: \"Desisto com \\\"sem permissão\\\".\"",
                        t.span,
                    ));
                }
                ClauseKind::Desist(self.expr()?)
            }
            "tomo" => {
                self.bump();
                let which = self.which()?;
                self.expect_word("de")?;
                ClauseKind::Take { place: self.place()?, which }
            }
            "pulo" => {
                self.bump();
                ClauseKind::Continue
            }
            _ if self.definite_article_at(0).is_some() || morphology::is_symbol(&w) => self.subject_clause()?,
            _ if self.vocab.by_first_person.contains_key(&w) => {
                let vt = self.bump();
                let sig = self.pick_overload(self.vocab.by_first_person[&w], vt.span)?;
                let args = self.call_args(sig, false, vt.span)?;
                let call = Call { sig, args, span: vt.span.to(self.prev_span()), verb_span: vt.span };
                let handler = self.fallback()?;
                ClauseKind::Call { call, handler }
            }
            _ => return Err(self.unknown_verb(&t)),
        };
        Ok(Clause { kind, span: start.to(self.prev_span()) })
    }

    fn unknown_verb(&self, t: &Token) -> Box<Diagnostic> {
        let w = t.word().unwrap_or_default();
        // Verbos conhecidos (da língua e do programa) cuja 3ª pessoa é a palavra escrita.
        let known = lexicon::clause_verb_infinitives()
            .map(String::from)
            .chain(self.vocab.by_first_person.values().map(|&i| self.prog.sigs[i].infinitive.clone()));
        let third = known
            .filter(|inf| [Number::Singular, Number::Plural].iter().any(|n| morphology::third_person(inf, *n).as_deref() == Some(w)))
            .find_map(|inf| {
                self.vocab
                    .by_infinitive
                    .get(&inf)
                    .map(|v| self.prog.sigs[v[0]].first_person.clone())
                    .or_else(|| morphology::first_person(&inf))
            });
        if let Some(f) = third {
            return Box::new(Diagnostic::new(
                Category::Morphology,
                format!("\"{}\" está na 3ª pessoa. Quando o programa age, narra em 1ª pessoa: \"{f}\".", t.orig),
                t.span,
            ));
        }
        if let Some(&sig) = self.vocab.by_infinitive.get(w).and_then(|v| v.first()) {
            let f = &self.prog.sigs[sig].first_person;
            return Box::new(Diagnostic::new(
                Category::Morphology,
                format!("\"{}\" está no infinitivo, que define. Para executar, a 1ª pessoa: \"{f}\".", t.orig),
                t.span,
            ));
        }
        let near = self
            .vocab
            .by_first_person
            .keys()
            .map(String::as_str)
            .chain(lexicon::clause_verb_forms().map(|f| -> &str { f }))
            .filter(|k| morphology::edit_distance(k, w) <= 2)
            .min_by_key(|k| morphology::edit_distance(k, w));
        if let Some(n) = near {
            return Box::new(
                Diagnostic::new(Category::Syntax, format!("\"{}\" não é um verbo conhecido.", t.orig), t.span)
                    .hint(format!("Quis dizer \"{n}\"?")),
            );
        }
        let d = Diagnostic::new(Category::Syntax, format!("\"{}\" não é um verbo conhecido.", t.orig), t.span);
        // Formas que alguém escreveria no lugar da construção da língua: a dica vem do léxico.
        let hint = matches!(w, "seja" | "sejam" | "guardo" | "acrescento" | "subtraio").then(|| lexicon::rejected_hint(w)).flatten();
        Box::new(d.maybe_hint(hint.map(String::from)))
    }

    // --------------------------------------------------------- dados-sujeito

    fn subject_clause(&mut self) -> Result<ClauseKind> {
        let place = self.place()?;
        let mut ann = None;
        if self.at_punct(',') {
            self.bump();
            ann = Some(self.aposto()?);
            self.expect_punct(',', "a vírgula que fecha o aposto")?;
        }
        let vt = self.peek().clone();
        let Some((inf, number)) = vt.word().and_then(subject_verb) else {
            return Err(Box::new(
                Diagnostic::new(
                    Category::Syntax,
                    format!("Esperava o verbo de {}, mas encontrei {}.", describe_place(&place), vt.describe()),
                    vt.span,
                )
                .hint("Depois do sujeito vem o que acontece com ele: \"é\", \"começa em\", \"vira\", \"cresce\", \"diminui\"."),
            ));
        };
        self.bump();
        let subj_number = place.number();
        if number != subj_number {
            let right = morphology::third_person(inf, subj_number).unwrap_or_default();
            return err(Diagnostic::new(
                Category::Agreement,
                format!(
                    "O verbo não concorda com o sujeito: {} é {}, então \"{right}\", não \"{}\".",
                    describe_place(&place),
                    subj_number.describe(),
                    vt.orig
                ),
                vt.span,
            ));
        }
        let declares = matches!(inf, "ser" | "começar");
        if (declares || ann.is_some()) && !place.proj.is_empty() {
            return err(Diagnostic::new(Category::Syntax, "Só se declara um nome novo; um campo ou item já existe.", vt.span)
                .hint(format!("Para mudar o valor, use \"vira\": \"{} vira ...\".", describe_place(&place))));
        }
        if ann.is_some() && !declares {
            return err(Diagnostic::new(Category::Syntax, "O aposto com o tipo só cabe na declaração (\"é\" ou \"começa em\").", vt.span));
        }
        Ok(match inf {
            "ser" => {
                let init = if let Some(span) = self.empty_word(&place.root)? { Init::Empty(span) } else { self.item_list()? };
                ClauseKind::Decl { name: place.root, ann, mutable: false, init }
            }
            "começar" => {
                let init = if let Some(span) = self.empty_word(&place.root)? {
                    Init::Empty(span)
                } else if self.eat_word("com") {
                    match self.item_list()? {
                        Init::Expr(e) => {
                            let span = e.span;
                            Init::List(vec![e], span)
                        }
                        other => other,
                    }
                } else {
                    self.expect_word("em")?;
                    Init::Expr(self.expr()?)
                };
                ClauseKind::Decl { name: place.root, ann, mutable: true, init }
            }
            "virar" => ClauseKind::Assign { place, value: Init::Expr(self.expr()?) },
            "ganhar" => ClauseKind::Push { place, value: self.expr()? },
            "perder" => ClauseKind::Remove { which: self.which()?, place },
            "crescer" => ClauseKind::Grow { place, value: self.expr()? },
            "diminuir" => ClauseKind::Shrink { place, value: self.expr()? },
            _ => unreachable!(),
        })
    }

    /// Aposto de tipo: "um inteiro de 32 bits", "talvez um texto".
    fn aposto(&mut self) -> Result<TypeAnn> {
        let start = self.peek().span;
        if self.eat_word("talvez") {
            let inner = self.aposto()?;
            if matches!(inner.ty, TypeExpr::Maybe(_)) {
                return err(Diagnostic::new(Category::Syntax, "\"talvez talvez\" não existe: a ausência já é uma só.", inner.span));
            }
            return Ok(TypeAnn { ty: TypeExpr::Maybe(Box::new(inner.ty)), span: start.to(self.prev_span()) });
        }
        let t = self.bump();
        let Some(art) = t.word().and_then(morphology::article).filter(|a| !a.definite) else {
            return Err(Box::new(Diagnostic::new(
                Category::Syntax,
                format!("Esperava o tipo com artigo indefinido (\"um inteiro\"), mas encontrei {}.", t.describe()),
                t.span,
            )));
        };
        let ty = self.type_noun(art, t.span)?;
        Ok(TypeAnn { ty, span: start.to(self.prev_span()) })
    }

    /// Substantivo de tipo depois do artigo indefinido, conferindo o gênero.
    fn type_noun(&mut self, art: morphology::Article, art_span: Span) -> Result<TypeExpr> {
        let (ty, gender) = self.type_word(Number::Singular)?;
        if gender != art.gender {
            let w = self.toks[self.pos - 1].orig.clone();
            let right = morphology::indefinite_article(gender, Number::Singular);
            return err(Diagnostic::new(
                Category::Agreement,
                format!("O tipo é {}: \"{right} ...\", não \"{} ...\".", gender.describe(), self.toks[art_span_index(self, art_span)].orig),
                art_span.to(self.prev_span()),
            )
            .hint(format!("O artigo concorda com o tipo (\"{w}\").")));
        }
        Ok(ty)
    }

    pub(super) fn bits(&mut self) -> Result<Option<u8>> {
        if !(self.at_word("de") && matches!(self.peek_at(1).kind, TokenKind::Int(_)) && self.peek_at(2).is_word("bits")) {
            return Ok(None);
        }
        self.bump();
        let n = self.bump();
        self.bump();
        let TokenKind::Int(v) = n.kind else { unreachable!() };
        if ![8, 16, 32, 64].contains(&v) {
            return err(Diagnostic::new(Category::Type, "Os inteiros têm 8, 16, 32 ou 64 bits.", n.span));
        }
        Ok(Some(v as u8))
    }

    // ------------------------------------------------------------------ lugares

    /// "de o" se escreve "do": preposição seguida de artigo contrai, obrigatoriamente.
    fn check_contraction(&self) -> Result<()> {
        let t = self.peek();
        if t.contraction.is_some() || self.allow_uncontracted || self.pos == 0 {
            return Ok(());
        }
        let prev = &self.toks[self.pos - 1];
        if prev.contraction.is_some() {
            return Ok(());
        }
        let Some(pw) = prev.word() else { return Ok(()) };
        let prep = pw.rsplit(' ').next().unwrap_or(pw);
        let Some(art) = t.word() else { return Ok(()) };
        if let Some(c) = morphology::contract(prep, art) {
            let written = format!("{prep} {}", t.orig);
            return err(Diagnostic::new(
                Category::Morphology,
                format!("Contração obrigatória: \"{written}\" se escreve \"{c}\"."),
                prev.span.to(t.span),
            ));
        }
        Ok(())
    }

    /// Escolhe entre verbos de mesmo infinitivo pela preposição que vem depois.
    fn pick_overload(&self, first: usize, _span: Span) -> Result<usize> {
        let inf = &self.prog.sigs[first].infinitive;
        let cands = &self.vocab.by_infinitive[inf];
        if cands.len() == 1 {
            return Ok(first);
        }
        for &c in cands {
            if let Some(Slot::Prep { prep, .. }) = self.prog.sigs[c].slots.iter().find(|s| matches!(s, Slot::Prep { .. })) {
                // Procura a preposição adiante na mesma oração.
                let mut k = 0;
                while !matches!(self.peek_at(k).kind, TokenKind::Punct('.' | ':' | ';') | TokenKind::Eof | TokenKind::Newline) {
                    if self.peek_at(k).is_word(prep) {
                        return Ok(c);
                    }
                    k += 1;
                }
            }
        }
        Ok(first)
    }

    fn name_ref(&mut self) -> Result<NameRef> {
        let t = self.peek().clone();
        if let Some(a) = self.definite_article_at(0) {
            self.check_contraction()?;
            self.bump();
            let w = self.bump();
            let Some(word) = w.word().map(str::to_string) else {
                return Err(Box::new(Diagnostic::new(
                    Category::Syntax,
                    format!("Depois do artigo \"{}\" esperava um nome, mas encontrei {}.", t.orig, w.describe()),
                    w.span,
                )));
            };
            let art_span = t.span;
            let contraction = t.contraction.map(|c| c.written);
            return Ok(NameRef {
                word,
                span: art_span.to(w.span),
                word_span: w.span,
                article: Some(ArticleUse { gender: a.gender, number: a.number, span: art_span, contraction }),
            });
        }
        if let Some(w) = t.word()
            && morphology::is_symbol(w)
        {
            self.bump();
            return Ok(NameRef { word: w.to_string(), span: t.span, word_span: t.span, article: None });
        }
        Err(self.expected("um nome com artigo (\"o saldo\") ou um símbolo (\"x\")"))
    }

    // -------------------------------------------------------------- expressões

    fn expr(&mut self) -> Result<Expr> {
        self.concat()
    }

    fn concat(&mut self) -> Result<Expr> {
        let first = self.additive()?;
        if !(self.at_word("seguido de") || self.at_word("seguida de")) {
            return Ok(first);
        }
        let mut parts = vec![first];
        while self.eat_word("seguido de") || self.eat_word("seguida de") {
            parts.push(self.additive()?);
        }
        let span = parts[0].span.to(parts.last().unwrap().span);
        Ok(Expr { kind: ExprKind::Concat(parts), span })
    }

    pub(super) fn additive(&mut self) -> Result<Expr> {
        let mut lhs = self.multiplicative()?;
        loop {
            if self.last_nominal && self.at_infix() {
                return Err(self.mixing_error());
            }
            let op = if self.eat_word("mais") {
                BinOp::Add
            } else if self.eat_word("menos") {
                BinOp::Sub
            } else {
                return Ok(lhs);
            };
            let rhs = self.multiplicative()?;
            let span = lhs.span.to(rhs.span);
            lhs = Expr { kind: ExprKind::Binary { op, lhs: Box::new(lhs), rhs: Box::new(rhs) }, span };
        }
    }

    /// Valor com conversão opcional: "o total como real".
    fn factor(&mut self) -> Result<Expr> {
        let e = self.primary()?;
        if !self.at_word("como") {
            return Ok(e);
        }
        if self.last_nominal {
            return Err(self.mixing_error());
        }
        let como = self.bump();
        let start = self.peek().span;
        if self.peek().word().and_then(morphology::article).is_some() {
            let t = self.peek().clone();
            return err(Diagnostic::new(
                Category::Syntax,
                "Depois de \"como\" o tipo vem sem artigo: \"como real\", \"como texto\".",
                t.span,
            ));
        }
        let (ty, _) = self.type_word(Number::Singular)?;
        let ann = TypeAnn { ty, span: start.to(self.prev_span()) };
        let span = e.span.to(self.prev_span());
        let _ = como;
        Ok(Expr { kind: ExprKind::Convert { value: Box::new(e), ty: ann }, span })
    }

    fn multiplicative(&mut self) -> Result<Expr> {
        let mut lhs = self.factor()?;
        loop {
            if self.last_nominal && self.at_infix() {
                return Err(self.mixing_error());
            }
            let op = if self.at_word("vezes") && self.starts_value(1) {
                self.bump();
                BinOp::Mul
            } else if self.eat_word("dividido por") {
                BinOp::Div
            } else if self.eat_word("módulo") {
                BinOp::Rem
            } else {
                return Ok(lhs);
            };
            let rhs = self.factor()?;
            let span = lhs.span.to(rhs.span);
            lhs = Expr { kind: ExprKind::Binary { op, lhs: Box::new(lhs), rhs: Box::new(rhs) }, span };
        }
    }

    /// ", ou desisto" (passa a falha adiante) ou ", ou VALOR" (valor de reserva).
    fn fallback(&mut self) -> Result<Option<Fallback>> {
        if !(self.at_punct(',') && self.peek_at(1).is_word("ou")) {
            return Ok(None);
        }
        self.bump();
        self.bump();
        if self.eat_word("desisto") {
            if self.eat_word("com") {
                return Ok(Some(Fallback::Desist(Some(Box::new(self.expr()?)))));
            }
            return Ok(Some(Fallback::Desist(None)));
        }
        Ok(Some(Fallback::Value(Box::new(self.additive()?))))
    }

    /// O token na posição `k` à frente pode começar um valor?
    fn starts_value(&self, k: usize) -> bool {
        let t = self.peek_at(k);
        match &t.kind {
            TokenKind::Int(_) | TokenKind::Real(_) | TokenKind::Text(_) | TokenKind::Char(_) | TokenKind::Punct('(') => true,
            TokenKind::Word(w) => {
                self.definite_article_at(k).is_some()
                    || morphology::is_symbol(w)
                    || matches!(w.as_str(), "verdadeiro" | "falso" | "nada")
                    || (morphology::article(w).is_some_and(|a| !a.definite)
                        && self
                            .peek_at(k + 1)
                            .word()
                            .is_some_and(|n| n == "cópia" || self.vocab.records.contains_key(n) || self.vocab.variants.contains_key(n)))
            }
            _ => false,
        }
    }

    fn construct_ahead(&self) -> bool {
        self.peek().word().and_then(morphology::article).is_some_and(|a| !a.definite)
            && self.peek_at(1).word().is_some_and(|w| self.vocab.records.contains_key(w) || self.vocab.variants.contains_key(w))
    }

    fn nominal_ahead(&self) -> bool {
        self.definite_article_at(0).is_some()
            && self.peek_at(1).word().is_some_and(|w| data::is_nominal_word(w) && !self.vocab.field_names.contains(w))
            && (self.peek_at(2).is_word("de") || self.peek_at(2).is_word("entre"))
    }

    fn mixing_error(&self) -> Box<Diagnostic> {
        let t = self.peek();
        Box::new(
            Diagnostic::new(Category::Syntax, "Operação por extenso seguida de conta infixa é ambígua: precisa de parênteses.", t.span)
                .hint("Escreva \"(a diferença de 10 e 3) mais 1\" ou \"a diferença de 10 e (3 mais 1)\", conforme o que quer dizer."),
        )
    }

    fn primary(&mut self) -> Result<Expr> {
        self.last_nominal = false;
        let t = self.peek().clone();
        let lit = |kind| Ok(Expr { kind, span: t.span });
        match &t.kind {
            TokenKind::Int(v) => {
                self.bump();
                lit(ExprKind::Int(*v))
            }
            TokenKind::Real(v) => {
                self.bump();
                lit(ExprKind::Real(*v))
            }
            TokenKind::Text(s) => {
                self.bump();
                lit(ExprKind::Text(s.clone()))
            }
            TokenKind::Char(c) => {
                self.bump();
                lit(ExprKind::Char(*c))
            }
            TokenKind::Punct('(') => {
                self.bump();
                let mut e = self.expr()?;
                let close = self.expect_punct(')', "o parêntese de fechamento")?;
                self.last_nominal = false;
                e.span = t.span.to(close.span);
                Ok(e)
            }
            TokenKind::Word(w) if w == "o" && self.peek_at(1).is_word("resultado") && t.contraction.is_none_or(|c| c.part == 2) => {
                self.bump();
                let r = self.bump();
                if self.at_word("de")
                    && let Some(inf) = self.peek_at(1).word()
                    && let Some(&sig) = self.vocab.by_infinitive.get(inf).and_then(|v| v.first())
                {
                    let de = self.bump();
                    if de.contraction.is_some() {
                        return err(Diagnostic::new(Category::Morphology, "Antes do verbo, \"de\" não contrai.", de.span));
                    }
                    let vt = self.bump();
                    let sig = self.pick_overload(sig, vt.span)?;
                    let args = self.call_args(sig, false, vt.span)?;
                    let span = t.span.to(self.prev_span());
                    let call = Expr { kind: ExprKind::Call(Call { sig, args, span, verb_span: vt.span }), span };
                    return Ok(match self.fallback()? {
                        Some(fallback) => {
                            let span = span.to(self.prev_span());
                            Expr { kind: ExprKind::OrElse { value: Box::new(call), fallback }, span }
                        }
                        None => call,
                    });
                }
                Ok(Expr { kind: ExprKind::Anaphora, span: t.span.to(r.span) })
            }
            TokenKind::Word(w) if w == "o" && self.peek_at(1).is_word("fato") && t.contraction.is_none_or(|c| c.part == 2) => {
                self.bump();
                self.bump();
                let de = self.expect_word("de")?;
                if let Some(c) = de.contraction {
                    let (_, _, art) = morphology::split_contraction(c.written).unwrap();
                    return err(Diagnostic::new(
                        Category::Morphology,
                        format!(
                            "Antes do sujeito de um infinitivo não se contrai: escreva \"o fato de {art} ...\", não \"o fato {} ...\".",
                            c.written
                        ),
                        de.span,
                    ));
                }
                self.allow_uncontracted = true;
                let c = self.cond_mood(Mood::Infinitive);
                self.allow_uncontracted = false;
                let c = c?;
                Ok(Expr { kind: ExprKind::Fact(Box::new(c)), span: t.span.to(self.prev_span()) })
            }
            TokenKind::Word(w) if (w == "uma" || w == "um") && self.peek_at(1).is_word("cópia") => {
                let a = self.bump();
                let c = self.bump();
                if a.is_word("um") {
                    return err(Diagnostic::new(Category::Agreement, "\"cópia\" é feminino: \"uma cópia\".", a.span.to(c.span)));
                }
                self.expect_word("de")?;
                let inner = self.primary()?;
                Ok(Expr { span: t.span.to(inner.span), kind: ExprKind::Copy(Box::new(inner)) })
            }
            TokenKind::Word(_) if self.construct_ahead() => Ok(self.construct()?.unwrap()),
            TokenKind::Word(_) if self.nominal_ahead() => Ok(self.nominal()?.unwrap()),
            TokenKind::Word(w) if w == "nada" => {
                self.bump();
                lit(ExprKind::Nothing)
            }
            TokenKind::Word(w) if w == "verdadeiro" || w == "falso" => {
                self.bump();
                lit(ExprKind::Bool(w == "verdadeiro"))
            }
            TokenKind::Word(_) => {
                let place = self.place()?;
                Ok(Expr { span: place.span, kind: ExprKind::Place(place) })
            }
            _ => Err(self.expected("um valor")),
        }
    }
}

fn art_span_index(p: &Parser, span: Span) -> usize {
    p.toks.iter().position(|t| t.span == span).unwrap_or(0)
}

/// Reconhece a forma de um verbo de dado-sujeito: devolve o infinitivo e o número.
fn subject_verb(w: &str) -> Option<(&'static str, Number)> {
    for v in lexicon::SUBJECT_VERBS {
        for n in [Number::Singular, Number::Plural] {
            if morphology::third_person(v.infinitive, n).as_deref() == Some(w) {
                return Some((v.infinitive, n));
            }
        }
    }
    None
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mood {
    /// Condição: "Se o saldo for ...".
    Subjunctive,
    /// Depois de "o fato de": "o fato de o saldo ser ...".
    Infinitive,
}

fn expr_gender(e: &Expr) -> Gender {
    match &e.kind {
        ExprKind::Place(p) => p.head_gender(),
        _ => Gender::Masculine,
    }
}

/// Número gramatical de uma expressão usada como sujeito.
fn expr_number(e: &Expr) -> Number {
    match &e.kind {
        ExprKind::Place(p) => p.number(),
        _ => Number::Singular,
    }
}

pub fn describe_place(p: &Place) -> String {
    format!("\"{}\"", p.root.display())
}
