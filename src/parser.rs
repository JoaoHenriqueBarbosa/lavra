//! Análise sintática por descida recursiva. Confere a gramática local (contrações,
//! concordância verbal, ordem das palavras); escopo, tipos e posse ficam na `sema`.

use crate::ast::*;
use crate::diagnostics::{Category, Diagnostic, Result, err};
use crate::lexer::{Token, TokenKind};
use crate::morphology::{self, Number};
use crate::span::Span;

pub fn parse(tokens: Vec<Token>) -> Result<Program> {
    let mut p = Parser { toks: tokens, pos: 0, prog: Program::default() };
    p.program()?;
    Ok(p.prog)
}

struct Parser {
    toks: Vec<Token>,
    pos: usize,
    prog: Program,
}

/// Verbos dos dados-sujeito, por infinitivo.
const SUBJECT_VERBS: &[&str] = &["ser", "começar", "virar", "crescer", "diminuir"];

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

    fn error(&self, cat: Category, msg: impl Into<String>, span: Span) -> Box<Diagnostic> {
        Box::new(Diagnostic::new(cat, msg, span))
    }

    /// Erro de "esperava X, encontrei Y", com o caso especial do ponto final esquecido.
    fn expected(&self, what: &str) -> Box<Diagnostic> {
        let t = self.peek();
        if what == "o ponto final" && matches!(t.kind, TokenKind::Newline | TokenKind::Eof | TokenKind::Dedent) {
            let end = self.prev_span().end as usize;
            return Box::new(
                Diagnostic::new(Category::Syntax, "Faltou o ponto final.", Span::new(end.saturating_sub(1), end))
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
        Ok(Stmt::Period(self.period()?))
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
        matches!(w, "escrevo" | "leio" | "devolvo" | "desisto" | "paro" | "pulo" | "tomo")
    }

    /// Depois de "e" ou vírgula: começa outra oração?
    fn is_clause_start(&self) -> bool {
        let t = self.peek();
        if let Some(w) = t.word()
            && Self::is_core_verb(w)
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
                TokenKind::Punct(_) | TokenKind::Newline | TokenKind::Eof | TokenKind::Indent | TokenKind::Dedent
                    if depth <= 0 =>
                {
                    // "O limite, um inteiro, é 100": aposto.
                    return t.is_punct(',') && k == 2 && self.peek_at(3).word().is_some_and(|w| morphology::article(w).is_some_and(|a| !a.definite) || w == "talvez");
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
            _ if self.definite_article_at(0).is_some() || morphology::is_symbol(&w) => self.subject_clause()?,
            _ => return Err(self.unknown_verb(&t)),
        };
        Ok(Clause { kind, span: start.to(self.prev_span()) })
    }

    fn unknown_verb(&self, t: &Token) -> Box<Diagnostic> {
        let w = t.word().unwrap_or_default();
        let d = Diagnostic::new(Category::Syntax, format!("\"{}\" não é um verbo conhecido.", t.orig), t.span);
        let hint = match w {
            "seja" | "sejam" => Some("A Lavra nova declara pelo sujeito: \"O saldo é 0.\" (fixo) ou \"O saldo começa em 0.\" (variável).".to_string()),
            "guardo" => Some("Para mudar um valor, o dado é o sujeito: \"O saldo vira 10.\"".to_string()),
            "acrescento" => Some("Para somar ao valor, o dado é o sujeito: \"O saldo cresce 10.\"".to_string()),
            "subtraio" => Some("Para tirar do valor, o dado é o sujeito: \"O saldo diminui 10.\"".to_string()),
            _ => {
                let third = morphology::conjugation::infinitive_from_third_person(w);
                third
                    .iter()
                    .find_map(|inf| morphology::first_person(inf).filter(|f| Self::is_core_verb(f)))
                    .map(|f| format!("\"{}\" está na 3ª pessoa. Quando o programa age, narra em 1ª pessoa: \"{f}\".", t.orig))
            }
        };
        Box::new(d.maybe_hint(hint))
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
                Diagnostic::new(Category::Syntax, format!("Esperava o verbo de {}, mas encontrei {}.", describe_place(&place), vt.describe()), vt.span)
                    .hint("Depois do sujeito vem o que acontece com ele: \"é\", \"começa em\", \"vira\", \"cresce\", \"diminui\"."),
            ));
        };
        self.bump();
        let subj_number = place.number();
        if number != subj_number {
            let right = morphology::third_person(inf, subj_number).unwrap_or_default();
            return err(Diagnostic::new(
                Category::Agreement,
                format!("O verbo não concorda com o sujeito: {} é {}, então \"{right}\", não \"{}\".", describe_place(&place), subj_number.describe(), vt.orig),
                vt.span,
            ));
        }
        let declares = matches!(inf, "ser" | "começar");
        if (declares || ann.is_some()) && !place.proj.is_empty() {
            return err(Diagnostic::new(
                Category::Syntax,
                "Só se declara um nome novo; um campo ou item já existe.",
                vt.span,
            )
            .hint(format!("Para mudar o valor, use \"vira\": \"{} vira ...\".", describe_place(&place))));
        }
        if ann.is_some() && !declares {
            return err(Diagnostic::new(Category::Syntax, "O aposto com o tipo só cabe na declaração (\"é\" ou \"começa em\").", vt.span));
        }
        Ok(match inf {
            "ser" => ClauseKind::Decl { name: place.root, ann, mutable: false, init: Init::Expr(self.expr()?) },
            "começar" => {
                self.expect_word("em")?;
                ClauseKind::Decl { name: place.root, ann, mutable: true, init: Init::Expr(self.expr()?) }
            }
            "virar" => ClauseKind::Assign { place, value: Init::Expr(self.expr()?) },
            "crescer" => ClauseKind::Grow { place, value: self.expr()? },
            "diminuir" => ClauseKind::Shrink { place, value: self.expr()? },
            _ => unreachable!(),
        })
    }

    /// Aposto de tipo: "um inteiro de 32 bits", "talvez um texto".
    fn aposto(&mut self) -> Result<TypeAnn> {
        let start = self.peek().span;
        let t = self.bump();
        let Some(art) = t.word().and_then(morphology::article).filter(|a| !a.definite) else {
            return Err(Box::new(Diagnostic::new(Category::Syntax, format!("Esperava o tipo com artigo indefinido (\"um inteiro\"), mas encontrei {}.", t.describe()), t.span)));
        };
        let ty = self.type_noun(art, t.span)?;
        Ok(TypeAnn { ty, span: start.to(self.prev_span()) })
    }

    /// Substantivo de tipo depois do artigo indefinido, conferindo o gênero.
    fn type_noun(&mut self, art: morphology::Article, art_span: Span) -> Result<TypeExpr> {
        let t = self.bump();
        let w = t.word().unwrap_or_default().to_string();
        let (ty, gender) = match w.as_str() {
            "inteiro" | "natural" => {
                let bits = self.bits()?;
                (TypeExpr::Int { bits: bits.unwrap_or(64), signed: w == "inteiro" }, morphology::Gender::Masculine)
            }
            "real" => {
                let bits = self.bits()?.unwrap_or(64);
                if bits != 64 && bits != 32 {
                    return err(Diagnostic::new(Category::Type, "Um real tem 32 ou 64 bits.", self.prev_span()));
                }
                (TypeExpr::Real { bits }, morphology::Gender::Masculine)
            }
            "byte" => (TypeExpr::Byte, morphology::Gender::Masculine),
            "lógico" => (TypeExpr::Bool, morphology::Gender::Masculine),
            "caractere" => (TypeExpr::Char, morphology::Gender::Masculine),
            "texto" => (TypeExpr::Text, morphology::Gender::Masculine),
            _ => return Err(Box::new(Diagnostic::new(Category::Type, format!("Tipo desconhecido: \"{}\".", t.orig), t.span).hint("Tipos: inteiro, natural, real, byte, lógico, caractere, texto."))),
        };
        if gender != art.gender {
            let right = morphology::indefinite_article(gender, Number::Singular);
            return err(Diagnostic::new(
                Category::Agreement,
                format!("\"{w}\" é {}: \"{right} {w}\".", gender.describe()),
                art_span.to(t.span),
            ));
        }
        Ok(ty)
    }

    fn bits(&mut self) -> Result<Option<u8>> {
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

    /// Lugar: "a carta", "x". (Campos e itens chegam nas próximas fatias.)
    fn place(&mut self) -> Result<Place> {
        let name = self.name_ref()?;
        Ok(Place { span: name.span, root: name, proj: vec![] })
    }

    fn name_ref(&mut self) -> Result<NameRef> {
        let t = self.peek().clone();
        if let Some(a) = self.definite_article_at(0) {
            self.bump();
            let w = self.bump();
            let Some(word) = w.word().map(str::to_string) else {
                return Err(Box::new(Diagnostic::new(Category::Syntax, format!("Depois do artigo \"{}\" esperava um nome, mas encontrei {}.", t.orig, w.describe()), w.span)));
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

    fn additive(&mut self) -> Result<Expr> {
        let mut lhs = self.multiplicative()?;
        loop {
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

    fn multiplicative(&mut self) -> Result<Expr> {
        let mut lhs = self.primary()?;
        loop {
            let op = if self.eat_word("vezes") {
                BinOp::Mul
            } else if self.eat_word("dividido por") {
                BinOp::Div
            } else if self.eat_word("módulo") {
                BinOp::Rem
            } else {
                return Ok(lhs);
            };
            let rhs = self.primary()?;
            let span = lhs.span.to(rhs.span);
            lhs = Expr { kind: ExprKind::Binary { op, lhs: Box::new(lhs), rhs: Box::new(rhs) }, span };
        }
    }

    fn primary(&mut self) -> Result<Expr> {
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
                e.span = t.span.to(close.span);
                Ok(e)
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

/// Reconhece a forma de um verbo de dado-sujeito: devolve o infinitivo e o número.
fn subject_verb(w: &str) -> Option<(&'static str, Number)> {
    for inf in SUBJECT_VERBS {
        for n in [Number::Singular, Number::Plural] {
            if morphology::third_person(inf, n).as_deref() == Some(w) {
                return Some((inf, n));
            }
        }
    }
    None
}

pub fn describe_place(p: &Place) -> String {
    format!("\"{}\"", p.root.display())
}
