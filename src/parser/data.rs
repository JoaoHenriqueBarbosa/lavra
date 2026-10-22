//! Dados: registros ("Uma conta é um registro de: ..."), lugares com projeção ("o nome
//! do titular da conta", "o item i das notas", "a última das notas"), coleções e as
//! operações nominais ("a soma de x e y").

use super::Parser;
use crate::ast::*;
use crate::diagnostics::{Category, Diagnostic, Result, err};
use crate::lexer::TokenKind;
use crate::morphology::{self, Gender, Number};
use crate::span::Span;

/// Operações nominais de dois termos: palavra, gênero, operação, se é comutativa.
const BINARY_NOMINALS: &[(&str, Gender, BinOp, bool)] = &[
    ("soma", Gender::Feminine, BinOp::Add, true),
    ("diferença", Gender::Feminine, BinOp::Sub, false),
    ("produto", Gender::Masculine, BinOp::Mul, true),
    ("quociente", Gender::Masculine, BinOp::Div, false),
    ("resto", Gender::Masculine, BinOp::Rem, false),
    ("potência", Gender::Feminine, BinOp::Pow, false),
    ("interseção", Gender::Feminine, BinOp::BitAnd, true),
    ("união", Gender::Feminine, BinOp::BitOr, true),
    ("disjunção", Gender::Feminine, BinOp::BitXor, true),
];

const UNARY_NOMINALS: &[(&str, Gender, UnOp)] = &[
    ("negativo", Gender::Masculine, UnOp::Neg),
    ("raiz", Gender::Feminine, UnOp::Sqrt),
    ("tamanho", Gender::Masculine, UnOp::Len),
    ("inverso", Gender::Masculine, UnOp::BitNot),
    ("módulo", Gender::Masculine, UnOp::Abs),
];

pub(super) fn is_nominal_word(w: &str) -> bool {
    BINARY_NOMINALS.iter().any(|n| n.0 == w) || UNARY_NOMINALS.iter().any(|n| n.0 == w) || w == "deslocamento"
}

impl Parser {
    // ------------------------------------------------------------- registros

    /// Pré-varredura dos registros: primeiro os nomes (um registro pode citar outro
    /// definido depois), depois os campos.
    pub(super) fn prescan_records(&mut self) -> Result<()> {
        let mut starts = Vec::new();
        let mut depth = 0usize;
        let mut at_start = true;
        for k in 0..self.toks.len() {
            let t = &self.toks[k];
            match t.kind {
                TokenKind::Indent => depth += 1,
                TokenKind::Dedent => depth = depth.saturating_sub(1),
                _ => {}
            }
            if depth == 0
                && at_start
                && t.word().and_then(morphology::article).is_some_and(|a| !a.definite)
                && self.toks.get(k + 2).is_some_and(|n| n.is_word("é"))
                && self.toks.get(k + 4).is_some_and(|n| n.is_word("registro"))
            {
                starts.push(k);
            }
            at_start = matches!(t.kind, TokenKind::Newline | TokenKind::Dedent | TokenKind::Indent);
        }
        for &k in &starts {
            let art_tok = self.toks[k].clone();
            let name_tok = self.toks[k + 1].clone();
            let art = morphology::article(art_tok.word().unwrap()).unwrap();
            let Some(name) = name_tok.word().map(str::to_string) else {
                return err(Diagnostic::new(Category::Syntax, "Esperava o nome do registro.", name_tok.span));
            };
            if art.number != Number::Singular {
                return err(Diagnostic::new(Category::Agreement, "Um tipo de registro se define no singular: \"Uma conta é um registro de:\".", art_tok.span));
            }
            if self.records.contains_key(&name) {
                return err(Diagnostic::new(Category::Scope, format!("O registro \"{name}\" já foi definido."), name_tok.span));
            }
            let id = self.prog.records.len();
            let plural = morphology::pluralize(&name);
            self.records.insert(name.clone(), id);
            self.record_plurals.insert(plural.clone(), id);
            self.prog.records.push(RecordDef { word: name, plural, gender: art.gender, span: art_tok.span.to(name_tok.span), fields: Vec::new() });
        }
        for &k in &starts {
            self.pos = k + 2;
            let id = self.records[self.toks[k + 1].word().unwrap()];
            self.expect_word("é")?;
            let um = self.bump();
            if !um.is_word("um") {
                return err(Diagnostic::new(Category::Agreement, "\"registro\" é masculino: \"é um registro de\".", um.span));
            }
            self.expect_word("registro")?;
            self.expect_word("de")?;
            self.expect_punct(':', "os dois-pontos")?;
            self.skip_newlines();
            if !matches!(self.peek().kind, TokenKind::Indent) {
                return Err(self.expected("os campos recuados na linha de baixo"));
            }
            self.bump();
            let mut fields: Vec<FieldDef> = Vec::new();
            loop {
                let name = self.name_ref()?;
                if name.is_symbol() {
                    return err(Diagnostic::new(Category::Syntax, "Campo de registro leva nome com artigo (\"o saldo\").", name.span));
                }
                if fields.iter().any(|f| f.name.word == name.word) {
                    return err(Diagnostic::new(Category::Scope, format!("O campo \"{}\" aparece duas vezes.", name.word), name.span));
                }
                if is_nominal_word(&name.word) || name.word == "item" {
                    return err(Diagnostic::new(Category::Morphology, format!("\"{}\" é nome de operação da língua e não serve de campo.", name.word), name.span));
                }
                self.expect_punct(',', "a vírgula entre o campo e o tipo")?;
                let ann = self.aposto()?;
                self.field_names.insert(name.word.clone());
                fields.push(FieldDef { name, ty: ann });
                if self.eat_punct(';') {
                    self.skip_newlines();
                    continue;
                }
                self.expect_punct('.', "o ponto final depois do último campo (ou ponto e vírgula entre campos)")?;
                break;
            }
            self.skip_newlines();
            if !matches!(self.peek().kind, TokenKind::Dedent | TokenKind::Eof) {
                return Err(self.expected("o fim do registro"));
            }
            if matches!(self.peek().kind, TokenKind::Dedent) {
                self.bump();
            }
            self.prog.records[id].fields = fields;
            self.skip_spans.insert(k, self.pos);
        }
        self.pos = 0;
        Ok(())
    }

    /// Tipo de registro no número pedido; devolve o nome do registro e o gênero.
    pub(super) fn record_type_word(&self, w: &str, number: Number) -> Option<(String, Gender)> {
        let id = match number {
            Number::Singular => self.records.get(w),
            Number::Plural => self.record_plurals.get(w),
        }?;
        let r = &self.prog.records[*id];
        Some((r.word.clone(), r.gender))
    }

    // --------------------------------------------------------------- lugares

    /// Um segmento de lugar pode ser seguido de "de" + outro lugar?
    fn projects(&self, seg: &Segment) -> bool {
        match seg {
            Segment::Name(n) => self.field_names.contains(&n.word) && !n.is_symbol(),
            Segment::Index(..) | Segment::Ordinal { .. } => true,
        }
    }

    /// Lugar: "a carta", "x", "o saldo da conta", "o item i das notas", "a última das
    /// notas". O "de" genitivo se lê da direita para a esquerda.
    pub(super) fn place(&mut self) -> Result<Place> {
        let start = self.peek().span;
        let mut segs = vec![self.segment()?];
        while self.at_word("de") && self.projects(segs.last().unwrap()) {
            let save = self.pos;
            self.bump();
            if self.definite_article_at(0).is_none() && !self.peek().word().is_some_and(morphology::is_symbol) {
                self.pos = save;
                break;
            }
            segs.push(self.segment()?);
        }
        let root = match segs.pop().unwrap() {
            Segment::Name(n) => n,
            Segment::Index(_, span) | Segment::Ordinal { span, .. } => {
                return err(Diagnostic::new(Category::Syntax, "Falta a coleção: \"o item 2 das notas\", \"a última das notas\".", span));
            }
        };
        let proj = segs
            .into_iter()
            .rev()
            .map(|s| match s {
                Segment::Name(n) => Proj::Field(n),
                Segment::Index(e, span) => Proj::Index(Box::new(e), span),
                Segment::Ordinal { position, gender, span } => Proj::Ordinal { position, gender, span },
            })
            .collect();
        Ok(Place { root, proj, span: start.to(self.prev_span()) })
    }

    fn segment(&mut self) -> Result<Segment> {
        if self.definite_article_at(0).is_some() {
            let at = self.peek().clone();
            let w = self.peek_at(1).clone();
            if w.is_word("item") {
                self.check_contraction()?;
                self.bump();
                self.bump();
                let e = self.additive()?;
                let span = at.span.to(self.prev_span());
                return Ok(Segment::Index(e, span));
            }
            if let Some((position, g, n)) = w.word().and_then(morphology::ordinal) {
                let a = self.definite_article_at(0).unwrap();
                if a.gender != g || a.number != n || n != Number::Singular {
                    let right = format!("{} {}", morphology::definite_article(g, Number::Singular), morphology::ordinal_word(position, g));
                    return err(Diagnostic::new(Category::Agreement, format!("O ordinal concorda com o artigo: \"{right}\"."), at.span.to(w.span)));
                }
                self.check_contraction()?;
                self.bump();
                self.bump();
                return Ok(Segment::Ordinal { position, gender: g, span: at.span.to(w.span) });
            }
        }
        Ok(Segment::Name(self.name_ref()?))
    }

    /// "o último", "a primeira", "o item 2": qual item sai de uma coleção.
    pub(super) fn which(&mut self) -> Result<Which> {
        match self.segment()? {
            Segment::Ordinal { position, span, .. } => Ok(Which::Ordinal { position, span }),
            Segment::Index(e, _) => Ok(Which::Item(e)),
            Segment::Name(n) => err(Diagnostic::new(Category::Syntax, format!("Esperava qual item (\"o último\", \"a primeira\", \"o item 2\"), mas encontrei \"{}\".", n.display()), n.span)),
        }
    }

    // -------------------------------------------------------------- coleções

    /// Itens escritos por extenso: "7, 8,5 e 9".
    pub(super) fn item_list(&mut self) -> Result<Init> {
        let start = self.peek().span;
        let mut items = vec![self.expr()?];
        loop {
            let save = self.pos;
            let sep = self.eat_punct(',') || self.at_word("e");
            if self.at_word("e") {
                self.bump();
            }
            if sep && self.starts_value(0) && !self.subject_follows() {
                items.push(self.expr()?);
                continue;
            }
            self.pos = save;
            break;
        }
        let span = start.to(self.prev_span());
        Ok(if items.len() == 1 { Init::Expr(items.pop().unwrap()) } else { Init::List(items, span) })
    }

    /// Depois de "e": começa uma oração com sujeito ("e o saldo cresce 1")?
    fn subject_follows(&self) -> bool {
        let mut k = 0;
        loop {
            let t = self.peek_at(k);
            match &t.kind {
                TokenKind::Word(w) if super::subject_verb(w).is_some() => return true,
                TokenKind::Word(_) | TokenKind::Int(_) | TokenKind::Real(_) | TokenKind::Text(_) | TokenKind::Char(_) => k += 1,
                _ => return false,
            }
            if k > 8 {
                return false;
            }
        }
    }

    /// "vazio", "vazias": concorda com o sujeito.
    pub(super) fn empty_word(&mut self, subject: &NameRef) -> Result<Option<Span>> {
        let Some(w) = self.peek().word() else { return Ok(None) };
        let Some((g, n)) = morphology::adjective_form(w, "vazio") else { return Ok(None) };
        let t = self.bump();
        if g != subject.gender() || n != subject.number() {
            let right = morphology::inflect_adjective("vazio", Gender::Masculine, subject.gender(), subject.number());
            return err(Diagnostic::new(Category::Agreement, format!("\"{}\" concorda com {}: \"{right}\".", t.orig, subject.display()), t.span));
        }
        Ok(Some(t.span))
    }

    // ----------------------------------------------------- operações nominais

    /// "a soma de x e y", "o tamanho das notas", "o deslocamento de x por 2 à esquerda".
    pub(super) fn nominal(&mut self) -> Result<Option<Expr>> {
        let Some(art) = self.definite_article_at(0) else { return Ok(None) };
        let at = self.peek().clone();
        let wt = self.peek_at(1).clone();
        let Some(w) = wt.word() else { return Ok(None) };
        if self.field_names.contains(w) || !is_nominal_word(w) {
            return Ok(None);
        }
        let next = self.peek_at(2);
        if !(next.is_word("de") || next.is_word("entre")) {
            return Ok(None);
        }
        let w = w.to_string();
        let check_gender = |g: Gender| -> Result<()> {
            if art.gender != g || art.number != Number::Singular {
                let right = morphology::definite_article(g, Number::Singular);
                return err(Diagnostic::new(Category::Agreement, format!("\"{w}\" é {}: \"{right} {w}\".", g.describe()), at.span.to(wt.span)));
            }
            Ok(())
        };
        self.check_contraction()?;
        self.bump();
        self.bump();
        if w == "deslocamento" {
            check_gender(Gender::Masculine)?;
            self.expect_word("de")?;
            let value = self.nominal_operand()?;
            self.expect_word("por")?;
            let amount = self.nominal_operand()?;
            let dir = self.bump();
            let op = match dir.word() {
                Some("à esquerda") => BinOp::Shl,
                Some("à direita") => BinOp::Shr,
                _ => return err(Diagnostic::new(Category::Syntax, "O deslocamento é \"à esquerda\" ou \"à direita\".", dir.span)),
            };
            let span = at.span.to(self.prev_span());
            self.last_nominal = true;
            return Ok(Some(Expr { kind: ExprKind::Binary { op, lhs: Box::new(value), rhs: Box::new(amount) }, span }));
        }
        if let Some(&(_, g, op, commutative)) = BINARY_NOMINALS.iter().find(|n| n.0 == w) {
            check_gender(g)?;
            let prep = self.bump();
            if prep.is_word("entre") && !commutative {
                return err(Diagnostic::new(Category::Regency, format!("\"entre\" indica simetria e só vale para operação comutativa; \"{w}\" pede \"de\"."), prep.span));
            }
            let lhs = self.nominal_operand()?;
            if !self.eat_word("e") {
                return Err(self.expected(&format!("\"e\" e o segundo termo de \"{w}\"")));
            }
            // "a diferença do x de p e do x de q": o segundo termo pode repetir o "de".
            if self.at_word("de") && self.peek().contraction.is_some() {
                self.bump();
            }
            let rhs = self.nominal_operand()?;
            let span = at.span.to(self.prev_span());
            self.last_nominal = true;
            return Ok(Some(Expr { kind: ExprKind::Binary { op, lhs: Box::new(lhs), rhs: Box::new(rhs) }, span }));
        }
        let &(_, g, op) = UNARY_NOMINALS.iter().find(|n| n.0 == w).unwrap();
        check_gender(g)?;
        self.expect_word("de")?;
        let operand = self.nominal_operand()?;
        let span = at.span.to(self.prev_span());
        self.last_nominal = true;
        Ok(Some(Expr { kind: ExprKind::Unary { op, operand: Box::new(operand) }, span }))
    }

    /// Termo de operação nominal: um valor simples, sem operação infixa. Para misturar,
    /// parênteses.
    fn nominal_operand(&mut self) -> Result<Expr> {
        let e = self.primary()?;
        if self.at_infix() {
            let t = self.peek().clone();
            return err(Diagnostic::new(Category::Syntax, "Dentro de uma operação por extenso, a conta infixa precisa de parênteses.", t.span)
                .hint("Escreva \"a diferença de 10 e (3 mais 1)\" ou \"(a diferença de 10 e 3) mais 1\", conforme o que quer dizer."));
        }
        self.last_nominal = false;
        Ok(e)
    }

    /// O próximo token é um operador infixo?
    pub(super) fn at_infix(&self) -> bool {
        self.at_word("mais") || self.at_word("menos") || self.at_word("dividido por") || self.at_word("módulo") || (self.at_word("vezes") && self.starts_value(1))
    }

    /// "uma conta com o saldo 100 e o titular \"Ana\"".
    pub(super) fn construct(&mut self) -> Result<Option<Expr>> {
        let at = self.peek().clone();
        let Some(art) = at.word().and_then(morphology::article).filter(|a| !a.definite) else { return Ok(None) };
        let wt = self.peek_at(1).clone();
        let Some(w) = wt.word() else { return Ok(None) };
        let Some(&id) = self.records.get(w) else { return Ok(None) };
        let r = &self.prog.records[id];
        if r.gender != art.gender || art.number != Number::Singular {
            let right = morphology::indefinite_article(r.gender, Number::Singular);
            return err(Diagnostic::new(Category::Agreement, format!("\"{w}\" é {}: \"{right} {w}\".", r.gender.describe()), at.span.to(wt.span)));
        }
        let (word, gender) = (r.word.clone(), r.gender);
        self.bump();
        self.bump();
        let mut fields = Vec::new();
        if self.eat_word("com") {
            loop {
                let n = self.name_ref()?;
                let value = self.expr()?;
                fields.push((n.word.clone(), n.span, value));
                if (self.at_word("e") || self.at_punct(',')) && self.definite_article_at(1).is_some() && self.peek_at(2).word().is_some_and(|w| self.field_names.contains(w)) {
                    self.bump();
                    continue;
                }
                break;
            }
        }
        let span = at.span.to(self.prev_span());
        Ok(Some(Expr { kind: ExprKind::Construct { word, gender, word_span: wt.span, fields }, span }))
    }
}

enum Segment {
    Name(NameRef),
    Index(Expr, Span),
    Ordinal { position: u32, gender: Gender, span: Span },
}
